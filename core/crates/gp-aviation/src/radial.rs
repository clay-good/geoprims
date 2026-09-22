//! Fix from a radial and distance (add-practitioner-essentials,
//! instrument-procedures "Station magnetic variation"): a VOR radial is
//! referenced to the station's own published variation, not today's
//! magnetic field, so the fix uses the variation the user enters and warns
//! STATION_VARIATION_DIFFERS when the WMM for the date differs by over 1°.

use geographiclib_rs::{DirectGeodesic, Geodesic};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::magnetic::{self as mag, Model};
use gp_geo::point;

use crate::ifr::IFH;
use crate::unit;

const IFH_VOR: Reference = Reference {
    locator: "Chapter 9 (VOR radials are magnetic, referenced to the station's declared variation, which is updated only when the station is realigned)",
    ..IFH
};

const fn angle(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
}

pub static RADIAL_FIX: ToolDef = ToolDef {
    id: "aviation.ifr.radial-fix",
    title: "Fix from a radial and distance",
    summary: "The position on a VOR radial at a distance from the station, using the station's published variation, with a warning when today's magnetic field differs from it by more than 1°.",
    aliases: &[
        "radial DME fix",
        "fix from radial and distance",
        "VOR radial position",
        "station declination",
    ],
    keywords: &[
        "radial",
        "VOR",
        "DME",
        "fix",
        "variation",
        "station declination",
        "magnetic",
    ],
    inputs: &[
        point::lat_field("lat", "Station latitude"),
        point::lon_field("lon", "Station longitude"),
        angle(
            "radial",
            "Radial",
            "Magnetic, from the station, like 098 deg",
        )
        .required()
        .core(),
        Field::new(
            "distance",
            "Distance",
            "Over the ground from the station, like 12.5 NM",
            Kind::Quantity {
                q: QT::Distance,
                unit: "NM",
            },
        )
        .required()
        .core(),
        angle(
            "variation",
            "Station variation",
            "Published for the station, east positive, like 11 for 11° E",
        )
        .required()
        .core(),
        Field::new(
            "date",
            "Date",
            "To compare with today's magnetic field, like 2026-09-22",
            Kind::Text { max_len: 24 },
        ),
    ],
    outputs: &[
        point::lat_field("fix_lat", "Fix latitude").precision(Precision::Decimals(7)),
        point::lon_field("fix_lon", "Fix longitude").precision(Precision::Decimals(7)),
        angle(
            "true_course",
            "True course from the station",
            "Radial + station variation",
        )
        .precision(Precision::Decimals(2))
        .angle_range("[0,360)"),
        angle(
            "wmm_declination",
            "WMM declination at the station",
            "For the date, east positive",
        )
        .precision(Precision::Decimals(2))
        .angle_range("unbounded")
        .optional(),
        angle(
            "variation_difference",
            "Station variation − WMM",
            "Over 1° brings a warning",
        )
        .precision(Precision::Decimals(2))
        .angle_range("unbounded")
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "STATION_VARIATION_DIFFERS",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "True course = radial + station variation (east positive); the fix is the Karney geodesic direct on WGS 84 from the station. With a date, WMM2025 declination at the station at sea level is compared with the station variation",
    accuracy: "Exact geometry for the entered ground distance (convert a DME reading with the slant-range tool first). The station variation governs the radial, even when the Earth's field has moved since",
    references: &[IFH_VOR],
    examples: &[Example {
        id: "primary",
        title: "Radial 098 at 12.5 NM from a station with 11° E variation",
        input: r#"{"lat":39.8,"lon":-104.7,"radial":"98 deg","distance":"12.5 NM","variation":"11 deg","date":"2026-09-22"}"#,
        source: "add-practitioner-essentials variation scenario: the fix uses the station's 11° E, and the difference from WMM is reported and warned when over 1°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.dme-slant-range",
            reason: "parent",
        },
        Related {
            id: "aviation.ifr.dme-arc-lead",
            reason: "alternative",
        },
    ],
    sentence: "The fix is at {fix_lat}, {fix_lon}, {true_course} true from the station.",
    limits: &[("batchRows", 10_000)],
    run: run_radial,
    ..ToolDef::BLANK
};

fn run_radial(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let radial = ctx.req_quantity("radial")?.to(dg);
    let var = ctx.req_quantity("variation")?.to(dg);
    let s = ctx.req_quantity("distance")?.base();
    if !(-60.0..=60.0).contains(&var) {
        return Err(ToolError::invalid(
            "/variation",
            "A station variation is within ±60°, east positive.",
        ));
    }
    if s.is_nan() || !(0.0..=1_000_000.0).contains(&s) {
        return Err(ToolError::invalid(
            "/distance",
            "Give a distance from 0 to 540 NM.",
        ));
    }
    let tc = (radial + var).rem_euclid(360.0);
    let (flat, flon): (f64, f64) = Geodesic::wgs84().direct(lat, lon, tc, s);
    let mut out = vec![
        (
            "fix_lat",
            ctx.out(
                "fix_lat",
                Q {
                    value: flat,
                    unit: dg,
                },
            ),
        ),
        (
            "fix_lon",
            ctx.out(
                "fix_lon",
                Q {
                    value: flon,
                    unit: dg,
                },
            ),
        ),
        (
            "true_course",
            ctx.out(
                "true_course",
                Q {
                    value: tc,
                    unit: dg,
                },
            ),
        ),
    ];
    if let Some(raw) = ctx.text("date")? {
        let t = mag::parse_date(&raw).map_err(|m| ToolError::invalid("/date", m))?;
        let (lo, hi) = Model::Wmm2025.window();
        if !(lo..=hi).contains(&t) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                format!("WMM2025 covers {lo} to {hi}; leave the date out to skip the comparison."),
            )
            .at("/date"));
        }
        ctx.assets.push(AssetRef {
            id: Model::Wmm2025.id().into(),
            version: Model::Wmm2025.version().into(),
        });
        let (b, sv) = mag::field(&mag::coeffs_at(Model::Wmm2025, t), lat, lon, 0.0);
        let d = mag::elements(b, sv).d;
        let diff = var - d;
        if diff.abs() > 1.0 {
            let fmt = ctx.options.format;
            let f = |x: f64| display::number(x, Precision::Decimals(1), fmt);
            ctx.warnings.push(Warning::new(
                "STATION_VARIATION_DIFFERS",
                format!(
                    "The station's published variation ({}°) differs from WMM2025 ({}°) by {}°. Radials follow the station's value, which this fix uses.",
                    f(var), f(d), f(diff.abs())
                ),
            ).at("/variation"));
        }
        out.push((
            "wmm_declination",
            ctx.out("wmm_declination", Q { value: d, unit: dg }),
        ));
        out.push((
            "variation_difference",
            ctx.out(
                "variation_difference",
                Q {
                    value: diff,
                    unit: dg,
                },
            ),
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "True course",
            "radial + station variation",
            format!("{}° + {}°", n(radial, 1), n(var, 1)),
            format!("{}°", n(tc, 2)),
        );
        ctx.step(
            "Fix latitude",
            "geodesic direct on WGS 84 from the station",
            format!("{} NM on {}° true", n(s / 1852.0, 2), n(tc, 2)),
            display::quantity(flat, "deg", Precision::Decimals(7), fmt),
        );
    }
    Ok(Json::obj(out))
}
