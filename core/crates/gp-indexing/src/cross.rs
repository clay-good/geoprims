//! Cross-index conversion (indexing/hierarchical-cells, "Cross-index
//! conversion"): one place in every indexing system the catalog has, at the
//! resolution whose cell is closest to a size you name.
//!
//! The systems disagree about what a cell is — a hexagon, a rectangle in
//! degrees, a square in Web Mercator — so "the same resolution" does not exist
//! across them. Each system's resolution is chosen by the ratio of its cell
//! size to the target, and the size it actually has at this point is reported
//! beside the reference, because that is the number a reader has to compare.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::{ellipsoid, gridref, mgrs, point, utmups};
use h3o::{LatLng, Resolution};
use libm::{cos, log, sqrt};

use crate::codes;

/// Meters per degree of latitude and of longitude on WGS 84 at `lat`, which is
/// how a cell measured in degrees becomes a cell measured on the ground.
fn degree_lengths(lat: f64) -> (f64, f64) {
    let (a, f) = (ellipsoid::CATALOG[0].a, ellipsoid::CATALOG[0].f);
    let e2 = f * (2.0 - f);
    let s = libm::sin(lat.to_radians());
    let w = sqrt(1.0 - e2 * s * s);
    (
        (a * (1.0 - e2) / (w * w * w)).to_radians(),
        (a / w).to_radians() * cos(lat.to_radians()),
    )
}

/// The geometric mean of a cell's width and height: one number to compare
/// systems by, since their cells are different shapes.
fn mean_size(width: f64, height: f64) -> f64 {
    sqrt(width * height)
}

/// Picks the resolution whose cell size is closest to the target by ratio,
/// which is the right comparison when sizes step by factors rather than
/// amounts.
fn closest<T: Copy>(options: &[(T, f64)], target: f64) -> (T, f64) {
    let best = options
        .iter()
        .min_by(|a, b| log(a.1 / target).abs().total_cmp(&log(b.1 / target).abs()))
        .expect("at least one option");
    (best.0, best.1)
}

fn run_cross(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let target = ctx.req_quantity("target_size")?.base();
    if !(0.05..=5.0e6).contains(&target) {
        return Err(ToolError::invalid(
            "/target_size",
            "Give a target cell size between 5 cm and 5,000 km.",
        ));
    }
    let (per_lat, per_lon) = degree_lengths(lat);
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let mut rows: Vec<Json> = Vec::new();
    // The nearest system by the same ratio the resolutions were chosen with.
    let mut best: Option<(f64, String)> = None;
    let mut row = |system: &str, reference: String, resolution: String, size: f64| {
        let d = log(size / target).abs();
        if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
            best = Some((d, system.to_owned()));
        }
        rows.push(Json::obj([
            ("system", Json::str(system)),
            ("reference", Json::str(reference)),
            ("resolution", Json::str(resolution)),
            (
                "cell_size",
                Q {
                    value: size,
                    unit: m,
                }
                .to_json(),
            ),
        ]));
    };

    // H3: hexagons, so the comparable size is the side of an equal-area square.
    let h3_options: Vec<(u8, f64)> = (0..=15u8)
        .map(|r| {
            let res = Resolution::try_from(r).expect("0-15");
            (r, sqrt(res.area_km2() * 1e6))
        })
        .collect();
    let (h3_res, h3_size) = closest(&h3_options, target);
    let cell = LatLng::new(lat, lon)
        .map_err(|e| ToolError::invalid("/lat", format!("{e}.")))?
        .to_cell(Resolution::try_from(h3_res).expect("0-15"));
    row(
        "H3",
        cell.to_string(),
        format!("resolution {h3_res}"),
        h3_size,
    );

    // Geohash: a rectangle in degrees, alternating 8 and 4 subdivisions.
    let geohash_options: Vec<(usize, f64)> = (1..=12)
        .map(|p| {
            let (dlat, dlon) = geohash_span(p);
            (p, mean_size(dlon * per_lon, dlat * per_lat))
        })
        .collect();
    let (gh_p, gh_size) = closest(&geohash_options, target);
    row(
        "Geohash",
        codes::geohash_encode(lat, lon, gh_p),
        format!("precision {gh_p}"),
        gh_size,
    );

    // Plus Code: degrees again, in steps of 20 and then 5, with a grid refinement.
    let olc_options: Vec<(usize, f64)> = [2usize, 4, 6, 8, 10, 11]
        .iter()
        .map(|&len| {
            let (dlat, dlon) = olc_span(len);
            (len, mean_size(dlon * per_lon, dlat * per_lat))
        })
        .collect();
    let (olc_len, olc_size) = closest(&olc_options, target);
    row(
        "Plus Code",
        codes::olc_encode(lat, lon, olc_len),
        format!("{olc_len} characters"),
        olc_size,
    );

    // Web Mercator tiles: square in the projection, so their ground size
    // shrinks with the cosine of the latitude.
    let tile_options: Vec<(u32, f64)> = (0..=22u32)
        .map(|z| (z, codes::ground_resolution(lat, z, 256.0) * 256.0))
        .collect();
    let (zoom, tile_size) = closest(&tile_options, target);
    let (tx, ty) = codes::tile(lat, lon, zoom);
    row(
        "Map tile",
        format!(
            "{zoom}/{tx}/{ty} (quadkey {})",
            codes::quadkey(zoom, tx, ty)
        ),
        format!("zoom {zoom}"),
        tile_size,
    );

    // Maidenhead: pairs of letters and digits, 20 deg down to arc seconds.
    let qth_options: Vec<(usize, f64)> = [2usize, 4, 6, 8, 10]
        .iter()
        .map(|&chars| {
            let (dlat, dlon) = maidenhead_span(chars);
            (chars, mean_size(dlon * per_lon, dlat * per_lat))
        })
        .collect();
    let (qth_chars, qth_size) = closest(&qth_options, target);
    row(
        "Maidenhead",
        gridref::maidenhead_encode(lat, lon, qth_chars),
        format!("{qth_chars} characters"),
        qth_size,
    );

    // MGRS: squares in meters on the grid, so their size does not vary.
    let mgrs_options: Vec<(i32, f64)> = (0..=5)
        .map(|digits| (digits, 100_000.0 / 10f64.powi(digits)))
        .collect();
    let (digits, mgrs_size) = closest(&mgrs_options, target);
    let wgs = ellipsoid::CATALOG[0];
    let grid = utmups::forward_auto(wgs.a, wgs.f, lat, lon);
    let reference = mgrs::encode(&grid, lat, digits)
        .map_err(|e| ToolError::new(ErrorCode::OutOfDomain, format!("{e}.")))?;
    row(
        "MGRS",
        reference,
        format!("{digits} digits, {} m squares", mgrs_size as i64),
        mgrs_size,
    );

    let closest_system = best.map(|(_, s)| s).unwrap_or_default();
    Ok(Json::obj([
        ("cells", Json::Arr(rows)),
        ("closest_match", Json::str(closest_system)),
    ]))
}

/// A geohash cell's span in degrees at precision `p`: each character adds 5
/// bits, split 3 to longitude and 2 to latitude, or the other way round.
fn geohash_span(p: usize) -> (f64, f64) {
    let bits = 5 * p;
    let lon_bits = bits.div_ceil(2);
    let lat_bits = bits / 2;
    (
        180.0 / 2f64.powi(lat_bits as i32),
        360.0 / 2f64.powi(lon_bits as i32),
    )
}

/// A Plus Code's span in degrees at code length `len`.
fn olc_span(len: usize) -> (f64, f64) {
    if len <= 10 {
        let pairs = len / 2;
        let size = 20.0 / 20f64.powi(pairs as i32 - 1);
        (size, size)
    } else {
        // Past ten characters the cell is refined on a 4 by 5 grid per step.
        let extra = len - 10;
        (
            (20.0 / 20f64.powi(4)) / 5f64.powi(extra as i32),
            (20.0 / 20f64.powi(4)) / 4f64.powi(extra as i32),
        )
    }
}

/// A Maidenhead locator's span in degrees at `chars` characters.
fn maidenhead_span(chars: usize) -> (f64, f64) {
    let (mut dlat, mut dlon) = (10.0, 20.0);
    let mut k = 2;
    while k < chars {
        if (k / 2) % 2 == 1 {
            dlat /= 10.0;
            dlon /= 10.0;
        } else {
            dlat /= 24.0;
            dlon /= 24.0;
        }
        k += 2;
    }
    (dlat, dlon)
}

const CELL_ROW: &[Field] = &[
    Field::new(
        "system",
        "System",
        "The indexing system",
        Kind::Text { max_len: 16 },
    ),
    Field::new(
        "reference",
        "Reference",
        "This place in that system",
        Kind::Text { max_len: 64 },
    ),
    Field::new(
        "resolution",
        "Resolution",
        "The level chosen, in that system's own terms",
        Kind::Text { max_len: 64 },
    ),
    Field::new(
        "cell_size",
        "Cell size",
        "The cell's ground size here, as the side of a square of the same area",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(1)),
];

pub static CROSS_INDEX: ToolDef = ToolDef {
    id: "indexing.convert.cross-index",
    title: "One place in every index, at a matched cell size",
    summary: "A point written as an H3 cell, a geohash, a Plus Code, a map tile, a Maidenhead locator, and an MGRS reference, each at the resolution whose cell is closest to the size you name, with the size it actually has there.",
    aliases: &[
        "cross index conversion",
        "geohash to h3",
        "compare cell sizes",
        "index converter",
    ],
    keywords: &[
        "cross index",
        "H3",
        "geohash",
        "Plus Code",
        "tile",
        "quadkey",
        "Maidenhead",
        "MGRS",
        "cell size",
        "resolution",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude").core(),
        point::lon_field("lon", "Longitude").core(),
        Field::new(
            "target_size",
            "Target cell size",
            "The cell size to match, like 150 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "cells",
            "This place in each system",
            "One row per system, at its closest resolution",
            Kind::List {
                items: CELL_ROW,
                min: 1,
                max: 16,
            },
        ),
        Field::new(
            "closest_match",
            "Closest to the target",
            "The system whose cell is nearest the size you asked for",
            Kind::Text { max_len: 16 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "For each system, the resolution whose cell size is closest to the target by ratio (log distance), since sizes step by factors; cell size is the side of a square of the same area, computed at this latitude for the systems whose cells are measured in degrees or in Web Mercator",
    accuracy: "Exact encodings. Cell sizes are that system's own cell at this point, except H3, whose figure is the resolution's average area over the globe.",
    when_to_use: "Use this when data arrives keyed by one index and has to be joined to data keyed by another, or when choosing which system to key by: the same ground is a resolution 10 hexagon, a seven-character geohash, and a zoom 18 tile, and they are not the same size. It also answers what a resolution in one system is worth in another, which is the question behind most cross-dataset joins.",
    limitations: "No two systems tile the ground the same way, so these are the nearest resolutions rather than equivalents, and the sizes differ by tens of percent. Cells in degrees narrow toward the poles and Web Mercator tiles shrink with the cosine of the latitude, so the same resolutions compare differently at other latitudes. S2, which the spec also names, is not implemented yet and is absent here rather than approximated.",
    references: &[
        crate::h3::H3_DOCS,
        crate::GEOHASH_REF,
        crate::OLC_SPEC,
        crate::OSM_TILES,
    ],
    examples: &[Example {
        id: "primary",
        title: "A place at about 150 m in every system",
        input: r#"{"lat":40.6892,"lon":-74.0445,"target_size":"150 m"}"#,
        source: "add-spatial-indexing-and-raster hierarchical-cells scenario: a 150 m target gives geohash precision 7 and H3 resolution 10",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.h3.resolution-chooser",
            reason: "alternative",
        },
        Related {
            id: "indexing.h3.lat-lng-to-cell",
            reason: "next",
        },
        Related {
            id: "indexing.geohash.encode",
            reason: "next",
        },
    ],
    sentence: "At about {target_size}, the closest match is {closest_match}.",
    limits: &[("batchRows", 10_000)],
    run: run_cross,
    ..ToolDef::BLANK
};
