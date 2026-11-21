//! NZGD1949 to NZGD2000 and back by LINZ's distortion grid
//! (add-international-grids, geodesy/national-grids, "NZGD2000 and its
//! deformation model"). The interpolation is in gp_geo::hgrid.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::hgrid::Grid;
use gp_geo::point;

pub const NZGD2000_GRID_ID: &str = "nzgd2000-grid";
pub const NZGD2000_GRID_VERSION: &str = "0005";
const FILE: &str = "nzgd2kgrid0005.grid";

const NOTICE: &str =
    "Sourced from the LINZ Data Service and licensed for reuse under the CC BY 4.0 licence.";
const ACCURACY: &str = "LINZ states that distortion grid transformations have an expected accuracy of 0.1 to 1.0 metres at the time the grid was calculated, and that they become less accurate over time as the datums continue to deform; this tool reproduces PROJ applying the same grid to a nanodegree.";

const LINZ: Reference = Reference {
    title: "Transformations between NZGD1949 and NZGD2000",
    issuer: "Land Information New Zealand (LINZ)",
    year: 2026,
    edition: "As published, read 2026-11-16",
    locator: "The distortion grid transformation, and the expected accuracy of each method: 3-parameter 5 m, 7-parameter 4 m, and the distortion grid 0.1 to 1.0 m at the time it was calculated, degrading as the datums deform; the grid itself, nzgd2kgrid0005, as PROJ-data distributes it under CC BY 4.0",
    url: "https://www.linz.govt.nz/guidance/geodetic-system/understanding-coordinate-conversions/geodetic-datum-conversions/transformations-nzgd1949-nzgd2000",
};

fn unit(q: QT, s: &str) -> &'static gp_base::units::Unit {
    units::by_symbol(q, s).expect("registered unit")
}
fn qv(q: QT, s: &str, value: f64) -> Q {
    Q {
        value,
        unit: unit(q, s),
    }
}

pub static NZGD2000: ToolDef = ToolDef {
    id: "geodesy.datum.nzgd2000",
    title: "NZGD1949 to NZGD2000 by LINZ's grid",
    summary: "Moves a New Zealand latitude and longitude between NZGD1949 and NZGD2000 by Land Information New Zealand's distortion grid, accurate to 0.1 to 1.0 m, with the shift on the ground and its direction.",
    aliases: &[
        "NZGD49 to NZGD2000",
        "NZGD1949 to NZGD2000 grid",
        "New Zealand datum conversion",
        "NZGD2000 to NZGD49",
    ],
    keywords: &[
        "NZGD2000",
        "NZGD1949",
        "NZGD49",
        "New Zealand",
        "LINZ",
        "distortion grid",
        "datum",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, so New Zealand is negative, like -36.5",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "East positive, like 175.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        Field::new(
            "direction",
            "Direction",
            "to-nzgd2000 (the default) from an NZGD1949 position, or to-nzgd1949, like to-nzgd2000",
            Kind::Choice(&["to-nzgd2000", "to-nzgd1949"]),
        ),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On the datum moved to",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "On the datum moved to",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        Field::new(
            "shift",
            "How far it moved",
            "On the ground, horizontally",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "azimuth",
            "Which way it moved",
            "Degrees from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        Field::new(
            "lat_shift",
            "Latitude shift",
            "The grid's offset here, NZGD1949 to NZGD2000",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(5)),
        Field::new(
            "lon_shift",
            "Longitude shift",
            "The grid's offset here, NZGD1949 to NZGD2000, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(5)),
        Field::new(
            "notice",
            "Notice",
            "The attribution LINZ's licence requires",
            Kind::Text { max_len: 120 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["LOW_ACCURACY_TRANSFORM", "EXPERIMENTAL_TOOL"],
    model: "LINZ's NZGD1949 to NZGD2000 distortion grid, nzgd2kgrid0005: latitude and longitude offsets at 0.1° nodes from 166°E to 180°E and 34°S to 48°S, interpolated bilinearly and added; the reverse by iterating the grid until the estimate moves by under 1e-12 degrees, as PROJ's hgridshift does",
    accuracy: ACCURACY,
    when_to_use: "Use this to bring a position off an older New Zealand map, survey plan, or record on NZGD1949 into NZGD2000, the datum of current New Zealand mapping, or to go back. The distortion grid models how the old datum was bent, so it is several times better than the seven-parameter set the legacy datum tool carries.",
    limitations: "LINZ states 0.1 to 1.0 m at the time the grid was calculated, and the datums keep deforming, so the grid gets less accurate with time; it is not for survey control. NZGD2000 is itself a point in time: moving a position to today's epoch needs LINZ's deformation model, which is not here. Outside the grid, 166°E to 180°E and 34°S to 48°S, there is no answer; the Chatham Islands are outside it.",
    references: &[LINZ],
    examples: &[Example {
        id: "primary",
        title: "LINZ's example point",
        source: "The national-grids specification's LINZ example: 36°30'00\"S 175°00'00\"E on NZGD1949 is 36°29'53.48482\"S 175°00'00.69469\"E on NZGD2000; PROJ 9.9.0 applying the same grid gives the same to the fifth decimal of a second",
        input: r#"{"lat":"36°30'00\"S","lon":"175°00'00\"E"}"#,
    }],
    primary_example: "primary",
    assets: &[NZGD2000_GRID_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.legacy",
            reason: "alternative",
        },
        Related {
            id: "geodesy.projection.nztm-forward",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon}, {shift} away on the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_nzgd2000,
    ..ToolDef::BLANK
};

fn run_nzgd2000(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let to_2000 = ctx.choice("direction")? != Some("to-nzgd1949");
    let bytes = ctx.asset(NZGD2000_GRID_ID, NZGD2000_GRID_VERSION, FILE)?;
    let grid = Grid::parse(&bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The NZGD2000 grid could not be read: {e}."),
        )
    })?;
    let outside = || {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "The point is outside LINZ's distortion grid, which covers 166°E to 180°E and 34°S to 48°S; the Chatham Islands are outside it.",
        )
        .at("/lat")
    };
    let (lat2, lon2, iterations) = if to_2000 {
        let (a, b) = grid.forward(lat, lon).ok_or_else(outside)?;
        (a, b, 0)
    } else {
        grid.inverse(lat, lon).ok_or_else(outside)?
    };
    // The grid's offsets, NZGD1949 to NZGD2000, at the NZGD1949 point.
    let (old_lat, old_lon) = if to_2000 { (lat, lon) } else { (lat2, lon2) };
    let (dp, dl) = grid.offsets(old_lat, old_lon).ok_or_else(outside)?;
    let (north, east) = gp_geo::beta2007::grs80().ground_offset(lat, lat2 - lat, lon2 - lon);
    let shift = north.hypot(east);
    let az = if shift == 0.0 {
        0.0
    } else {
        east.atan2(north).to_degrees().rem_euclid(360.0)
    };
    ctx.accuracy = Some(ACCURACY.into());
    ctx.warnings.push(Warning::new(
        "LOW_ACCURACY_TRANSFORM",
        "LINZ states 0.1 to 1.0 m for this grid when it was calculated, and less since, as the datums deform.",
    ));
    if ctx.explaining() {
        ctx.step(
            "The grid's offsets",
            "bilinear between the four 0.1° nodes around the NZGD1949 point",
            format!("{old_lat:.9}°, {old_lon:.9}°"),
            format!("Δφ {dp:.5}\", Δλ {dl:.5}\""),
        );
        ctx.step(
            if to_2000 {
                "Onto NZGD2000"
            } else {
                "Back onto NZGD1949"
            },
            if to_2000 {
                "the offsets added"
            } else {
                "the offsets taken away, iterated until the point settles"
            },
            format!("{lat:.9}°, {lon:.9}°"),
            format!(
                "{}°{}",
                gp_base::display::number(lat2, Precision::Decimals(9), ctx.options.format),
                if to_2000 {
                    String::new()
                } else {
                    format!(" after {iterations} iterations")
                }
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", qv(QT::Angle, "deg", lat2))),
        ("lon", ctx.out("lon", qv(QT::Angle, "deg", lon2))),
        ("shift", ctx.out("shift", qv(QT::Length, "m", shift))),
        ("azimuth", ctx.out("azimuth", qv(QT::Angle, "deg", az))),
        (
            "lat_shift",
            ctx.out("lat_shift", qv(QT::Angle, "arcsec", dp)),
        ),
        (
            "lon_shift",
            ctx.out("lon_shift", qv(QT::Angle, "arcsec", dl)),
        ),
        ("notice", Json::str(NOTICE)),
    ]))
}
