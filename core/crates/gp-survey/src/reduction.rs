//! Instrument reductions (add-survey-suite, survey/instrument-reductions):
//! slope distance to horizontal and vertical with two-face zenith means, and
//! the curvature-and-refraction correction with its textbook coefficient.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point::plain_angle;
use libm::{cos, sin};

use gp_base::tool::Reference;

use crate::{common_unit, len, len_out, unit};

/// The same textbook, at the chapters these reductions come from.
const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapter 4 (curvature and refraction), chapter 6 (reducing slope distances), and the total-station chapters (two-face zenith observations and index error)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003237",
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
    .angle_range("unbounded")
}

/// The zenith angle a reading means, in degrees, with the two-face mean and
/// index error when both faces were read.
fn zenith(ctx: &mut Ctx) -> Result<(f64, Option<(f64, f64)>), ToolError> {
    // Degrees, or degrees-minutes-seconds as a field book writes them.
    let z = plain_angle(ctx, "zenith")?;
    let va = plain_angle(ctx, "vertical_angle")?;
    let fr = plain_angle(ctx, "zenith_face_right")?;
    let fl = match (z, va) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/vertical_angle",
                "Give a zenith angle or a vertical angle, not both.",
            ));
        }
        (Some(z), None) => z,
        // A vertical angle is measured up from the horizon: Z = 90° − α.
        (None, Some(a)) => {
            if !(-90.0..=90.0).contains(&a) {
                return Err(ToolError::invalid(
                    "/vertical_angle",
                    "A vertical angle is between -90° and 90°.",
                ));
            }
            90.0 - a
        }
        (None, None) => {
            return Err(ToolError::invalid(
                "/zenith",
                "Give the zenith angle (or a vertical angle).",
            ));
        }
    };
    if !(0.0..=180.0).contains(&fl) {
        return Err(ToolError::invalid(
            "/zenith",
            "A face-left zenith is between 0° and 180°. A reading over 180° is face right: enter it as the face-right zenith, with the face-left reading here.",
        ));
    }
    let Some(fr) = fr else { return Ok((fl, None)) };
    if va.is_some() {
        return Err(ToolError::invalid(
            "/zenith_face_right",
            "A face-right reading pairs with a face-left zenith, not a vertical angle.",
        ));
    }
    if !(180.0..=360.0).contains(&fr) {
        return Err(ToolError::invalid(
            "/zenith_face_right",
            "A face-right zenith is between 180° and 360°.",
        ));
    }
    // Face right reads 360° − Z plus twice the index error: the mean cancels it.
    let mean = (fl + (360.0 - fr)) / 2.0;
    let index = (fl + fr - 360.0) / 2.0;
    Ok((mean, Some((mean, index))))
}

pub static SLOPE: ToolDef = ToolDef {
    id: "survey.reduction.slope",
    title: "Slope distance to horizontal and vertical",
    summary: "Horizontal distance, vertical difference, and elevation difference from a slope distance and zenith angle, with two-face means and the curvature and refraction correction for long sights.",
    aliases: &[
        "slope reduction",
        "slope distance to horizontal",
        "trig leveling",
        "two face mean",
    ],
    keywords: &[
        "slope distance",
        "zenith",
        "vertical angle",
        "horizontal distance",
        "HD",
        "VD",
        "face left",
        "face right",
        "index error",
        "instrument height",
    ],
    inputs: &[
        len(
            "slope_distance",
            "Slope distance",
            "Measured along the line of sight, like 500 m",
        )
        .required()
        .core(),
        angle(
            "zenith",
            "Zenith angle",
            "Face left, from straight up, like 85°00'00\"",
        )
        .core(),
        angle(
            "zenith_face_right",
            "Face-right zenith",
            "The same target read face right, like 274°59'40\", for the two-face mean",
        )
        .core(),
        len(
            "instrument_height",
            "Instrument height HI",
            "Above the station mark, like 1.55 m",
        )
        .core(),
        len(
            "target_height",
            "Target height HR",
            "Rod or prism height, like 1.80 m",
        ),
        angle(
            "vertical_angle",
            "Vertical angle",
            "From the horizon, up positive, like 5° (instead of a zenith)",
        ),
        len(
            "curvature_beyond",
            "Correct curvature beyond",
            "Apply curvature and refraction to sights longer than this, like 150 m",
        ),
        Field::new(
            "refraction",
            "Refraction coefficient k",
            "Default 0.13",
            Kind::Number { min: 0.0, max: 1.0 },
        ),
    ],
    outputs: &[
        len_out(
            "horizontal_distance",
            "Horizontal distance",
            "HD = SD × sin Z",
        ),
        len_out(
            "vertical_difference",
            "Vertical difference",
            "VD = SD × cos Z",
        ),
        len_out(
            "elevation_difference",
            "Elevation difference",
            "HI + VD − HR, plus curvature and refraction when applied",
        )
        .optional(),
        len_out(
            "curvature_refraction",
            "Curvature and refraction",
            "(1 − k) × HD² / 2R, added to the elevation difference",
        )
        .optional(),
        Field::new(
            "mean_zenith",
            "Mean zenith",
            "The two-face mean, (FL + 360° − FR) / 2",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6))
        .optional(),
        Field::new(
            "mean_zenith_dms",
            "Mean zenith (DMS)",
            "Degrees, minutes, and seconds",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        Field::new(
            "index_error",
            "Index error",
            "(FL + FR − 360°) / 2, positive when the circle reads high",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "HD = SD·sin Z and VD = SD·cos Z (Ghilani & Wolf 2018, ch. 6); two-face mean Z = (FL + 360° − FR)/2 with index error (FL + FR − 360°)/2; curvature and refraction (1 − k)·HD²/(2R) with R = 6,371,000 m",
    accuracy: "Exact for the reduction; the curvature and refraction correction carries the uncertainty of k, which varies with the air near the ground",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "500 m at a zenith of 85°",
        input: r#"{"slope_distance":"500 m","zenith":"85°00'00\""}"#,
        source: "add-survey-suite scenario: HD ≈ 498.097 m, VD ≈ 43.578 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.reduction.curvature-refraction",
            reason: "next",
        },
        Related {
            id: "survey.reduction.combined-factor",
            reason: "next",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "next",
        },
    ],
    sentence: "The horizontal distance is {horizontal_distance} and the vertical difference {vertical_difference}.{if mean_zenith > 0} The two-face mean zenith is {mean_zenith_dms}, with an index error of {index_error}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_slope,
    ..ToolDef::BLANK
};

fn run_slope(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let sd = ctx.req_quantity("slope_distance")?;
    let hi = ctx.quantity("instrument_height")?;
    let hr = ctx.quantity("target_height")?;
    let beyond = ctx.quantity("curvature_beyond")?;
    let named: Vec<(&str, Q)> = [
        ("slope_distance", Some(sd)),
        ("instrument_height", hi),
        ("target_height", hr),
        ("curvature_beyond", beyond),
    ]
    .into_iter()
    .filter_map(|(n, q)| q.map(|q| (n, q)))
    .collect();
    let u = common_unit(&named)?;
    let s = sd.to(u);
    if s <= 0.0 {
        return Err(ToolError::invalid(
            "/slope_distance",
            "The slope distance must be positive.",
        ));
    }
    let (z, faces) = zenith(ctx)?;
    let zr = z.to_radians();
    let hd = s * sin(zr);
    let vd = s * cos(zr);
    let q = |v: f64| Q { value: v, unit: u };
    let mut out = vec![
        (
            "horizontal_distance",
            ctx.emit("horizontal_distance", q(hd), u),
        ),
        (
            "vertical_difference",
            ctx.emit("vertical_difference", q(vd), u),
        ),
    ];
    // Curvature and refraction, when the sight is long enough to ask for it.
    let k = ctx.number("refraction")?.unwrap_or(0.13);
    let m = unit(QT::Length, "m");
    let cr = beyond.map(|b| b.to(u)).filter(|b| hd > *b).map(|_| {
        let hd_m = q(hd).to(m);
        Q {
            value: (1.0 - k) * hd_m * hd_m / (2.0 * 6_371_000.0),
            unit: m,
        }
        .to(u)
    });
    if hi.is_some() || hr.is_some() {
        let de = hi.map_or(0.0, |h| h.to(u)) + vd - hr.map_or(0.0, |h| h.to(u)) + cr.unwrap_or(0.0);
        out.push((
            "elevation_difference",
            ctx.emit("elevation_difference", q(de), u),
        ));
    }
    if let Some(c) = cr {
        out.push((
            "curvature_refraction",
            ctx.emit("curvature_refraction", q(c), u),
        ));
    }
    if let Some((mean, index)) = faces {
        let d = unit(QT::Angle, "deg");
        out.push((
            "mean_zenith",
            ctx.out(
                "mean_zenith",
                Q {
                    value: mean,
                    unit: d,
                },
            ),
        ));
        out.push((
            "mean_zenith_dms",
            Json::Str(dms::format(mean, Axis::Lon, Style::Dms, 1, false)),
        ));
        let s = unit(QT::Angle, "arcsec");
        out.push((
            "index_error",
            ctx.out(
                "index_error",
                Q {
                    value: index * 3600.0,
                    unit: s,
                },
            ),
        ));
    }
    Ok(Json::obj(out))
}

pub static CURVATURE: ToolDef = ToolDef {
    id: "survey.reduction.curvature-refraction",
    title: "Curvature and refraction correction",
    summary: "How much the earth's curve, less the bending of the line of sight, lowers a level sight over a distance, with the textbook coefficient for the refraction coefficient used.",
    aliases: &[
        "curvature and refraction",
        "c and r correction",
        "earth curvature correction",
    ],
    keywords: &[
        "curvature",
        "refraction",
        "leveling",
        "long sight",
        "coefficient",
        "0.0675",
        "trig leveling",
    ],
    inputs: &[
        len(
            "distance",
            "Sight distance",
            "Horizontal distance to the rod, like 1 km",
        )
        .required()
        .core(),
        Field::new(
            "refraction",
            "Refraction coefficient k",
            "Default 0.13; 0.14 gives the 0.0675 m/km² rule",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .core(),
        len("radius", "Earth radius", "Default 6,371,000 m"),
    ],
    outputs: &[
        len_out(
            "correction",
            "Curvature and refraction",
            "(1 − k) × D² / 2R",
        ),
        len_out("curvature", "Curvature alone", "D² / 2R"),
        len_out("refraction_part", "Refraction", "−k × D² / 2R"),
        Field::new(
            "coefficient",
            "Coefficient",
            "(1 − k) / 2R, in meters per square kilometer",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "coefficient_label",
            "Coefficient and its k",
            "The coefficient with the refraction coefficient it matches",
            Kind::Text { max_len: 60 },
        ),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "h = (1 − k)·D²/(2R), the curvature D²/(2R) less the refraction k·D²/(2R) (Ghilani & Wolf 2018, ch. 4); R = 6,371,000 m and k = 0.13 by default",
    accuracy: "Exact for the given k and R; k itself varies with the air near the ground, most at low sights and midday",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A 1 km sight with k = 0.14",
        input: r#"{"distance":"1 km","refraction":0.14}"#,
        source: "add-survey-suite scenario: ≈ 0.0675 m with k = 0.14, ≈ 0.0683 m with k = 0.13",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.reduction.slope",
        reason: "parent",
    }],
    sentence: "Curvature and refraction lower a level sight by {correction} at this distance, a coefficient of {coefficient_label}.",
    limits: &[("batchRows", 10_000)],
    run: run_curvature,
    ..ToolDef::BLANK
};

fn run_curvature(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let d = ctx.req_quantity("distance")?;
    let m = unit(QT::Length, "m");
    let dm = d.to(m);
    if dm < 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The distance cannot be negative.",
        ));
    }
    let k = ctx.number("refraction")?.unwrap_or(0.13);
    let r = ctx.quantity("radius")?.map_or(6_371_000.0, |q| q.to(m));
    if r <= 0.0 {
        return Err(ToolError::invalid(
            "/radius",
            "The earth radius must be positive.",
        ));
    }
    let curv = dm * dm / (2.0 * r);
    // Results in the unit the distance was given in.
    let u = d.unit;
    let q = |v: f64| Q { value: v, unit: m };
    let coef = (1.0 - k) * 1.0e6 / (2.0 * r);
    Ok(Json::obj(vec![
        ("correction", ctx.emit("correction", q((1.0 - k) * curv), u)),
        ("curvature", ctx.emit("curvature", q(curv), u)),
        (
            "refraction_part",
            ctx.emit("refraction_part", q(-k * curv), u),
        ),
        ("coefficient", Json::Num(coef)),
        (
            "coefficient_label",
            Json::Str(format!("{coef:.4} m/km² (k = {k})")),
        ),
    ]))
}
