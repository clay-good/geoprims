//! CRS search (geodesy/projections, "CRS registry lookup and transform"): the
//! coordinate systems the converters here produce, found by EPSG code, by
//! words in the name, or by a point inside the area of use, each with its
//! status and the tool that converts to it. EPSG systems come from the index
//! generated from EPSG v13.102 (gp_geo::crs); SPCS2022 zones from the NGS beta
//! asset, read only when a name or a point could match one.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::crs::{self, Crs, CrsKind};
use gp_geo::point;

use crate::spcs2022;

const EPSG: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "Coordinate reference system names, units, and areas of use",
    url: "https://epsg.org/search/by-name",
};
const NGS_SPCS2022: Reference = Reference {
    title: "State Plane Coordinate System of 2022 (SPCS2022) beta zone definitions and example coordinates",
    issuer: "National Geodetic Survey, NOAA",
    year: 2026,
    edition: "SPCS2022 beta zone definitions",
    locator: "Zone names, codes, and bounds, updated June 1, 2026",
    url: "https://beta.ngs.noaa.gov/SPCS/zone-information.html",
};

/// More matches than this are cut, with a note to narrow the search.
const MAX_ROWS: usize = 200;

const fn text(name: &'static str, title: &'static str, help: &'static str, n: usize) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: n })
}

const ROW: &[Field] = &[
    text(
        "code",
        "Code",
        "EPSG code, or the NGS code of an SPCS2022 zone",
        24,
    ),
    text("name", "Name", "As the registry gives it", 120),
    text(
        "kind",
        "Kind",
        "Geographic, UTM, State Plane, or Web Mercator",
        40,
    ),
    text("unit", "Unit", "Of its coordinates", 24),
    text(
        "status",
        "Status",
        "current, legacy, or beta (not yet adopted)",
        12,
    ),
    text(
        "convert_with",
        "Convert with",
        "The tool here that produces these coordinates",
        60,
    ),
];

pub static SEARCH: ToolDef = ToolDef {
    id: "geodesy.crs.search",
    title: "Coordinate system finder",
    summary: "Finds coordinate reference systems by EPSG code, by name, or by a point inside their area of use: UTM, State Plane 1983 and 2022, and geographic, each with its unit, its status, and the tool that converts to it.",
    aliases: &[
        "EPSG lookup",
        "find EPSG code",
        "what coordinate system is this",
        "CRS search",
        "projection finder",
    ],
    keywords: &[
        "EPSG",
        "CRS",
        "coordinate system",
        "projection",
        "UTM",
        "state plane",
        "SPCS",
        "NAD83",
        "WGS 84",
        "zone",
        "datum",
    ],
    inputs: &[
        text(
            "query",
            "EPSG code or name",
            "Like 2232, EPSG:26913, or Colorado Central",
            80,
        )
        .core(),
        Field::new(
            "lat",
            "Latitude",
            "Or a point inside the area of use, like 39.74",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "Decimal degrees, like -104.99",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-180,180)"),
    ],
    outputs: &[
        Field::new(
            "matches",
            "Coordinate systems",
            "Most local first",
            Kind::List {
                items: ROW,
                min: 0,
                max: MAX_ROWS,
            },
        ),
        Field::new(
            "count",
            "Found",
            "How many matched, before any cut",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        text("note", "Note", "How the match was made", 240),
    ],
    errors: &[gp_base::ErrorCode::InvalidInput],
    assets: &[spcs2022::ASSET_ID],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Code or name match against the CRS index from EPSG v13.102 and the SPCS2022 beta zones, or the area-of-use box of each",
    accuracy: "Areas of use are EPSG's and NGS's rectangular boxes, so near a zone line more than one zone covers a point",
    when_to_use: "Use this when you have coordinates and a code or a name but are not sure what they are, or when you need the right system for a place: give an EPSG code from a file’s metadata, a name like Colorado Central, or a point, and it lists the UTM zones, State Plane zones in meters and feet, and geographic systems that fit, with each one’s unit, whether it is current, legacy, or still beta, and which tool here converts to it.",
    limitations: "The index holds only what the converters here produce: geographic WGS 84, NAD83, NAD83(2011), and NAD27, Web Mercator, every UTM zone on WGS 84, NAD83, and NAD83(2011), and every SPCS83 zone on NAD83 and NAD83(2011), plus the SPCS2022 beta zones. Other EPSG systems, county and local grids among them, are not listed. A point search uses each system’s rectangular area of use, not the county or state line, so confirm a State Plane zone by county. Finding a system does not transform between datums; that is the datum tools.",
    references: &[EPSG, NGS_SPCS2022],
    examples: &[Example {
        id: "primary",
        title: "What covers downtown Denver",
        input: r#"{"lat":39.74,"lon":-104.99}"#,
        source: "add-geodesy-suite location scenario: UTM 13N on NAD83 and WGS 84, SPCS83 Colorado Central, and the SPCS2022 zones covering the point",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.zone-lookup",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.zone",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.transform",
            reason: "next",
        },
    ],
    sentence: "Found {count} coordinate {plural count \"system\" \"systems\"}.",
    limits: &[("batchRows", 100)],
    run: run_search,
    ..ToolDef::BLANK
};

/// Sort order: the most local systems first.
fn rank(kind: &str) -> u8 {
    match kind {
        "State Plane 1983" => 0,
        "State Plane 2022" => 1,
        "UTM" => 2,
        "Web Mercator" => 3,
        _ => 4,
    }
}

fn epsg_row(c: &Crs) -> (u8, String, Json) {
    let (kind, convert) = match c.kind {
        CrsKind::Geographic => (
            "Geographic",
            match c.code {
                // NAD27 and the original NAD83 move by the NADCON5 grids;
                // NAD83(2011) by the frame transformations.
                4267 | 4269 => "geodesy.datum.nadcon5",
                6318 => "geodesy.datum.transform",
                _ => "",
            },
        ),
        CrsKind::WebMercator => ("Web Mercator", "geodesy.projection.web-mercator-forward"),
        CrsKind::Utm { .. } => ("UTM", "geodesy.utm.forward"),
        CrsKind::Spcs83 { .. } => ("State Plane 1983", "geodesy.spcs.spcs83-forward"),
    };
    let unit = match c.unit {
        "deg" => "degrees",
        "m" => "meters",
        "ftUS" => "US survey feet",
        _ => "international feet",
    };
    let status = if c.code == 4267 { "legacy" } else { "current" };
    (
        rank(kind),
        format!("{:09}", c.code),
        Json::obj([
            ("code", Json::str(format!("EPSG:{}", c.code))),
            ("name", Json::str(c.name)),
            ("kind", Json::str(kind)),
            ("unit", Json::str(unit)),
            ("status", Json::str(status)),
            ("convert_with", Json::str(convert)),
        ]),
    )
}

fn run_search(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let query = ctx.text("query")?.unwrap_or_default();
    let query = query.trim().to_owned();
    let at = if ctx.is_set("lat") || ctx.is_set("lon") {
        Some(point::read(ctx, "lat", "lon")?)
    } else {
        None
    };
    if query.is_empty() && at.is_none() {
        return Err(
            ToolError::invalid("/query", "Give an EPSG code, a name, or a point.")
                .hint("Like 2232, Colorado Central, or a latitude and longitude"),
        );
    }
    let code = crs::epsg_code(&query);
    let words = crs::words(&query);
    let covers = |c: &Crs| at.is_none_or(|(lat, lon)| c.covers(lat, lon));
    // A system whose whole name was typed comes first, so "NAD83" finds the
    // datum itself before the hundreds of systems built on it.
    let exact = |name: &str| !query.is_empty() && name.eq_ignore_ascii_case(&query);
    let mut rows: Vec<(u8, String, Json)> = crs::REGISTRY
        .iter()
        .filter(|c| match code {
            Some(n) => c.code == n,
            None => crs::matches_words(&words, &c.search_text()),
        })
        .filter(|c| covers(c))
        .map(|c| {
            let mut row = epsg_row(c);
            if exact(c.name) {
                row.0 = 0;
                row.1.insert(0, ' ');
            }
            row
        })
        .collect();

    // SPCS2022: only when a name or a point could match a zone, so a plain
    // EPSG lookup does not fetch the zone file.
    let ngs_code = query.strip_prefix("NGS:SPCS2022:").unwrap_or(&query);
    let ngs = ngs_code.len() == 6 && ngs_code.bytes().all(|b| b.is_ascii_digit());
    if code.is_none() || ngs {
        let data = spcs2022::load(ctx)?;
        for z in &data.zones {
            let named = if ngs {
                z.code == ngs_code
            } else {
                crs::matches_words(
                    &words,
                    &format!("{} state plane spcs spcs2022 2022 {}", z.name, z.code),
                )
            };
            let inside = at.is_none_or(|(lat, lon)| spcs2022::contains(z, lat, lon));
            if named && inside {
                rows.push((
                    rank("State Plane 2022"),
                    z.code.clone(),
                    Json::obj([
                        ("code", Json::str(format!("NGS:SPCS2022:{}", z.code))),
                        ("name", Json::str(z.name.clone())),
                        ("kind", Json::str("State Plane 2022")),
                        ("unit", Json::str("meters")),
                        ("status", Json::str(z.status.clone())),
                        ("convert_with", Json::str("geodesy.spcs.spcs2022-forward")),
                    ]),
                ));
            }
        }
    }
    rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let count = rows.len();
    let how = match (code.is_some() || ngs, !words.is_empty(), at.is_some()) {
        (true, _, false) => "The system with that code.",
        (true, _, true) => "The system with that code, if its area of use covers the point.",
        (false, true, false) => "Systems whose name has every word you gave.",
        (false, true, true) => {
            "Systems whose name has every word you gave and whose area of use covers the point."
        }
        _ => {
            "Systems whose area of use covers the point, the most local first. Confirm a State Plane zone by county."
        }
    };
    let note = if count > MAX_ROWS {
        format!("{how} Showing the first {MAX_ROWS}; add words to narrow it.")
    } else if count == 0 {
        format!(
            "{how} Nothing matched; the index holds geographic, UTM, State Plane, and Web Mercator systems only."
        )
    } else {
        how.to_owned()
    };
    rows.truncate(MAX_ROWS);
    Ok(Json::obj(vec![
        (
            "matches",
            Json::Arr(rows.into_iter().map(|(_, _, j)| j).collect()),
        ),
        ("count", Json::Num(count as f64)),
        ("note", Json::str(note)),
    ]))
}
