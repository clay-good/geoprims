//! Instrument reductions (add-survey-suite, survey/instrument-reductions):
//! slope distance to horizontal and vertical with two-face zenith means, the
//! curvature-and-refraction correction with its textbook coefficient, stadia,
//! heights of objects you cannot reach, and total-station offset shots.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Bare, Ctx, Example, Field, Kind, Layer, Precision, Q, Related, Slot, ToolDef};
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
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
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
    slots: &[
        Slot::new("distance", &["distance", "sight", "d"]),
        Slot::new("refraction", &["k", "refraction", "coefficient"])
            .range(0.0, 1.0)
            .bare(Bare::Decimal),
    ],
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

pub static STADIA: ToolDef = ToolDef {
    id: "survey.reduction.stadia",
    title: "Stadia reduction",
    summary: "Horizontal distance, vertical difference, and elevation difference from a stadia rod interval and the sight's zenith angle.",
    aliases: &["stadia", "stadia reduction", "tacheometry"],
    keywords: &[
        "stadia",
        "rod interval",
        "stadia constant",
        "tacheometry",
        "zenith",
        "middle hair",
    ],
    inputs: &[
        len(
            "interval",
            "Rod interval",
            "Upper hair minus lower hair, like 1.234 m",
        )
        .required()
        .core(),
        angle(
            "zenith",
            "Zenith angle",
            "Like 88°30'00\" (or give a vertical angle)",
        )
        .core(),
        len("instrument_height", "Instrument height HI", "Like 1.50 m").core(),
        len("rod_reading", "Middle-hair reading", "Like 1.60 m").core(),
        angle(
            "vertical_angle",
            "Vertical angle",
            "From the horizon, up positive, like 1°30'",
        ),
        Field::new(
            "stadia_constant",
            "Stadia constant K",
            "The multiplier, 100 on most instruments",
            Kind::Number {
                min: 1.0,
                max: 1000.0,
            },
        ),
        len(
            "additive_constant",
            "Additive constant C",
            "0 for internal-focusing instruments",
        ),
    ],
    outputs: &[
        len_out(
            "horizontal_distance",
            "Horizontal distance",
            "K × s × sin² Z + C × sin Z",
        ),
        len_out(
            "vertical_difference",
            "Vertical difference",
            "K × s × sin Z × cos Z + C × cos Z",
        ),
        len_out(
            "elevation_difference",
            "Elevation difference",
            "HI + V − the middle-hair reading",
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Inclined stadia sight: H = K·s·sin²Z + C·sin Z and V = K·s·sin Z·cos Z + C·cos Z (Ghilani & Wolf 2018, stadia); K = 100 and C = 0 by default",
    accuracy: "Stadia distances are good to about 1 part in 300 to 1 in 1,000, set by reading the rod interval; the arithmetic is exact",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A 1.234 m interval at a zenith of 88°30'",
        input: r#"{"interval":"1.234 m","zenith":"88°30'00\"","instrument_height":"1.50 m","rod_reading":"1.60 m"}"#,
        source: "Stadia formulas with K = 100, C = 0",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.reduction.slope",
            reason: "alternative",
        },
        Related {
            id: "survey.reduction.inaccessible-height",
            reason: "next",
        },
    ],
    sentence: "The horizontal distance is {horizontal_distance} and the vertical difference {vertical_difference}.",
    limits: &[("batchRows", 10_000)],
    run: run_stadia,
    ..ToolDef::BLANK
};

/// The zenith of a single sight: a zenith, or 90° less a vertical angle.
fn one_zenith(ctx: &mut Ctx, zname: &str, vname: &str) -> Result<f64, ToolError> {
    match (plain_angle(ctx, zname)?, plain_angle(ctx, vname)?) {
        (Some(_), Some(_)) => Err(ToolError::invalid(
            &format!("/{vname}"),
            "Give a zenith angle or a vertical angle, not both.",
        )),
        (Some(z), None) if (0.0..=180.0).contains(&z) => Ok(z),
        (Some(_), None) => Err(ToolError::invalid(
            &format!("/{zname}"),
            "A zenith angle is between 0° and 180°.",
        )),
        (None, Some(a)) if (-90.0..=90.0).contains(&a) => Ok(90.0 - a),
        (None, Some(_)) => Err(ToolError::invalid(
            &format!("/{vname}"),
            "A vertical angle is between -90° and 90°.",
        )),
        (None, None) => Err(ToolError::invalid(
            &format!("/{zname}"),
            "Give the zenith angle (or a vertical angle).",
        )),
    }
}

fn run_stadia(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.req_quantity("interval")?;
    let hi = ctx.quantity("instrument_height")?;
    let rod = ctx.quantity("rod_reading")?;
    let c = ctx.quantity("additive_constant")?;
    let named: Vec<(&str, Q)> = [
        ("interval", Some(s)),
        ("instrument_height", hi),
        ("rod_reading", rod),
        ("additive_constant", c),
    ]
    .into_iter()
    .filter_map(|(n, q)| q.map(|q| (n, q)))
    .collect();
    let u = common_unit(&named)?;
    let iv = s.to(u);
    if iv <= 0.0 {
        return Err(ToolError::invalid(
            "/interval",
            "The rod interval must be positive.",
        ));
    }
    let k = ctx.number("stadia_constant")?.unwrap_or(100.0);
    let c = c.map_or(0.0, |q| q.to(u));
    let z = one_zenith(ctx, "zenith", "vertical_angle")?.to_radians();
    let (sz, cz) = (sin(z), cos(z));
    let h = k * iv * sz * sz + c * sz;
    let v = k * iv * sz * cz + c * cz;
    let q = |x: f64| Q { value: x, unit: u };
    let mut out = vec![
        (
            "horizontal_distance",
            ctx.emit("horizontal_distance", q(h), u),
        ),
        (
            "vertical_difference",
            ctx.emit("vertical_difference", q(v), u),
        ),
    ];
    if hi.is_some() || rod.is_some() {
        let de = hi.map_or(0.0, |x| x.to(u)) + v - rod.map_or(0.0, |x| x.to(u));
        out.push((
            "elevation_difference",
            ctx.emit("elevation_difference", q(de), u),
        ));
    }
    Ok(Json::obj(out))
}

pub static INACCESSIBLE: ToolDef = ToolDef {
    id: "survey.reduction.inaccessible-height",
    title: "Height of an inaccessible object",
    summary: "The height of a tower, tree, or building you cannot reach, from zenith angles to its base and top and a horizontal distance, or from two stations in line with it.",
    aliases: &[
        "tower height",
        "height of a building",
        "inaccessible height",
        "tree height",
    ],
    keywords: &[
        "height",
        "tower",
        "inaccessible",
        "zenith",
        "baseline",
        "two stations",
        "trigonometric height",
    ],
    inputs: &[
        angle(
            "zenith_top",
            "Zenith to the top",
            "From the near station, like 70°00'00\"",
        )
        .required()
        .core(),
        angle(
            "zenith_base",
            "Zenith to the base",
            "From the same station, like 92°00'00\"",
        )
        .core(),
        len(
            "distance",
            "Horizontal distance",
            "Station to the object, like 120 m",
        )
        .core(),
        len(
            "baseline",
            "Baseline",
            "Instead of the distance: from a second station farther away, in line, like 30 m",
        ),
        angle(
            "zenith_top_far",
            "Zenith to the top from the far station",
            "With the baseline, like 75°00'00\"",
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
            "height",
            "Height",
            "Top above base (or above the instrument, with no base sight)",
        ),
        len_out(
            "top_above_instrument",
            "Top above the instrument",
            "D / tan Z to the top, plus curvature and refraction when applied",
        ),
        len_out(
            "distance_used",
            "Horizontal distance",
            "Measured, or solved from the two stations",
        ),
        len_out(
            "curvature_refraction",
            "Curvature and refraction",
            "Added to each point's height above the instrument; it cancels in the height",
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "One station: height = D·(cot Z_top − cot Z_base). Two stations in line: D = b·cot Z_far / (cot Z_near − cot Z_far) from the near station, then as for one station (Ghilani & Wolf 2018, trigonometric leveling); curvature and refraction (1 − k)·D²/(2R) when D exceeds the threshold",
    accuracy: "Exact for the geometry; the result carries the angle and distance errors, and assumes the top is plumb over the base",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A tower 120 m away, zenith 70° to the top and 92° to the base",
        input: r#"{"zenith_top":"70°00'00\"","zenith_base":"92°00'00\"","distance":"120 m"}"#,
        source: "One-station trigonometric height",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.reduction.stadia",
            reason: "alternative",
        },
        Related {
            id: "survey.reduction.curvature-refraction",
            reason: "next",
        },
    ],
    sentence: "The object is {height} tall, measured {distance_used} away.",
    limits: &[("batchRows", 10_000)],
    run: run_inaccessible,
    ..ToolDef::BLANK
};

fn cot_of(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let Some(z) = plain_angle(ctx, name)? else {
        return Ok(None);
    };
    if !(z > 0.0 && z < 180.0) {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            "A zenith angle is between 0° and 180°, not straight up or down.",
        ));
    }
    let r = z.to_radians();
    Ok(Some(cos(r) / sin(r)))
}

fn run_inaccessible(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let top = cot_of(ctx, "zenith_top")?.expect("required");
    let base = cot_of(ctx, "zenith_base")?;
    let far = cot_of(ctx, "zenith_top_far")?;
    let dist = ctx.quantity("distance")?;
    let b = ctx.quantity("baseline")?;
    let beyond = ctx.quantity("curvature_beyond")?;
    let named: Vec<(&str, Q)> = [
        ("distance", dist),
        ("baseline", b),
        ("curvature_beyond", beyond),
    ]
    .into_iter()
    .filter_map(|(n, q)| q.map(|q| (n, q)))
    .collect();
    let u = common_unit(&named)?;
    let d = match (dist, b, far) {
        (Some(d), None, _) => d.to(u),
        (None, Some(b), Some(far)) => {
            // Same top, same height: d·cot Z_near = (d + b)·cot Z_far, so
            // d = b·cot Z_far / (cot Z_near − cot Z_far). The far station must
            // see the top lower, a larger zenith and so a smaller cotangent.
            if top - far <= 0.0 {
                return Err(ToolError::invalid(
                    "/zenith_top_far",
                    "From the farther station the top must look lower (a larger zenith) than from the near one.",
                ));
            }
            b.to(u) * far / (top - far)
        }
        (None, Some(_), None) => {
            return Err(ToolError::invalid(
                "/zenith_top_far",
                "With a baseline, give the zenith to the top from the far station too.",
            ));
        }
        (Some(_), Some(_), _) => {
            return Err(ToolError::invalid(
                "/baseline",
                "Give the distance or a baseline, not both.",
            ));
        }
        (None, None, _) => {
            return Err(ToolError::invalid(
                "/distance",
                "Give the horizontal distance, or a baseline with the far station's zenith.",
            ));
        }
    };
    if d <= 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The horizontal distance must be positive.",
        ));
    }
    let k = ctx.number("refraction")?.unwrap_or(0.13);
    let m = unit(QT::Length, "m");
    let q = |x: f64| Q { value: x, unit: u };
    let cr = beyond.map(|t| t.to(u)).filter(|t| d > *t).map(|_| {
        let dm = q(d).to(m);
        Q {
            value: (1.0 - k) * dm * dm / (2.0 * 6_371_000.0),
            unit: m,
        }
        .to(u)
    });
    let top_h = d * top + cr.unwrap_or(0.0);
    // Base and top share the distance, so curvature and refraction cancel in the height.
    let height = base.map_or(top_h, |bc| d * top - d * bc);
    let mut out = vec![
        ("height", ctx.emit("height", q(height), u)),
        (
            "top_above_instrument",
            ctx.emit("top_above_instrument", q(top_h), u),
        ),
        ("distance_used", ctx.emit("distance_used", q(d), u)),
    ];
    if let Some(c) = cr {
        out.push((
            "curvature_refraction",
            ctx.emit("curvature_refraction", q(c), u),
        ));
    }
    Ok(Json::obj(out))
}

pub static OFFSET: ToolDef = ToolDef {
    id: "survey.cogo.offset-shot",
    title: "Total-station offset shot",
    summary: "The coordinates of a point the prism cannot sit on: from a shot beside it with left, right, in, or out offsets, or from an angle offset to the center of a tree or pole.",
    aliases: &[
        "offset shot",
        "angle offset",
        "distance offset",
        "tree center",
    ],
    keywords: &[
        "offset",
        "angle offset",
        "distance offset",
        "tree",
        "pole",
        "center",
        "left",
        "right",
        "in",
        "out",
        "prism",
    ],
    inputs: &[
        len(
            "northing",
            "Station northing",
            "The occupied point, like 5000.000 ft",
        )
        .required()
        .core(),
        len("easting", "Station easting", "Like 5000.000 ft")
            .required()
            .core(),
        Field::new(
            "direction",
            "Direction to the prism",
            "Bearing or azimuth, like N 30°00'00\" E or 30",
            Kind::Text { max_len: 32 },
        )
        .required()
        .core(),
        len(
            "distance",
            "Horizontal distance",
            "Station to the prism, like 150.00 ft",
        )
        .required()
        .core(),
        len(
            "offset_right",
            "Offset right",
            "Right of the line of sight, negative for left, like 2.5 ft",
        )
        .core(),
        len(
            "offset_out",
            "Offset out",
            "Beyond the prism along the line, negative for in, like 1.0 ft",
        ),
        Field::new(
            "center_direction",
            "Direction to the center",
            "Angle offset: the direction turned to the center, like N 31°00'00\" E",
            Kind::Text { max_len: 32 },
        ),
        len(
            "radius",
            "Radius",
            "Angle offset: the tree or pole's radius, like 0.75 ft",
        ),
    ],
    outputs: &[
        len_out("northing", "Northing", "Of the offset point"),
        len_out("easting", "Easting", "Of the offset point"),
        len_out(
            "distance",
            "Horizontal distance",
            "Station to the offset point",
        ),
        Field::new(
            "azimuth",
            "Azimuth",
            "Station to the offset point",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "bearing",
            "Bearing",
            "Station to the offset point",
            Kind::Text { max_len: 32 },
        ),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Distance offset: out moves along the line of sight and right at 90° to it. Angle offset: the center lies at the measured distance plus the radius, along the direction turned to the center (Ghilani & Wolf 2018, total-station field practice)",
    accuracy: "Exact for the geometry; an angle offset assumes the shot was taken to the side of a round object at the same distance as its face",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "The center of a tree: 150.00 ft to its side, 0.75 ft radius",
        input: r#"{"northing":"5000 ft","easting":"5000 ft","direction":"N 30°00'00\" E","distance":"150 ft","center_direction":"N 30°17'00\" E","radius":"0.75 ft"}"#,
        source: "add-survey-suite tree-center scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.forward",
            reason: "parent",
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "next",
        },
    ],
    sentence: "The point is at northing {northing}, easting {easting}, {distance} from the station on {bearing}.",
    limits: &[("batchRows", 10_000)],
    run: run_offset,
    ..ToolDef::BLANK
};

fn run_offset(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let n = ctx.req_quantity("northing")?;
    let e = ctx.req_quantity("easting")?;
    let d = ctx.req_quantity("distance")?;
    let right = ctx.quantity("offset_right")?;
    let out_ = ctx.quantity("offset_out")?;
    let radius = ctx.quantity("radius")?;
    let named: Vec<(&str, Q)> = [
        ("northing", Some(n)),
        ("easting", Some(e)),
        ("distance", Some(d)),
        ("offset_right", right),
        ("offset_out", out_),
        ("radius", radius),
    ]
    .into_iter()
    .filter_map(|(k, q)| q.map(|q| (k, q)))
    .collect();
    let u = common_unit(&named)?;
    let dir = |ctx: &Ctx, name: &str| -> Result<Option<f64>, ToolError> {
        ctx.text(name)?
            .map(|t| {
                crate::direction::parse(&t).map_err(|m| ToolError::invalid(&format!("/{name}"), m))
            })
            .transpose()
    };
    let az = dir(ctx, "direction")?.expect("required");
    let center = dir(ctx, "center_direction")?;
    let dist = d.to(u);
    if dist <= 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The distance must be positive.",
        ));
    }
    let (along, across, toward) = match (center, radius) {
        // Angle offset: the center, a radius beyond the face, along the direction turned to it.
        (Some(c), Some(r)) => {
            if right.is_some() || out_.is_some() {
                return Err(ToolError::invalid(
                    "/center_direction",
                    "Use distance offsets or an angle offset, not both.",
                ));
            }
            let r = r.to(u);
            if r < 0.0 {
                return Err(ToolError::invalid(
                    "/radius",
                    "The radius cannot be negative.",
                ));
            }
            (dist + r, 0.0, c)
        }
        (Some(_), None) => {
            return Err(ToolError::invalid(
                "/radius",
                "An angle offset needs the radius of the tree or pole.",
            ));
        }
        (None, Some(_)) => {
            return Err(ToolError::invalid(
                "/center_direction",
                "An angle offset needs the direction turned to the center.",
            ));
        }
        (None, None) => (
            dist + out_.map_or(0.0, |q| q.to(u)),
            right.map_or(0.0, |q| q.to(u)),
            az,
        ),
    };
    let t = toward.to_radians();
    let n2 = n.to(u) + along * cos(t) + across * cos(t + core::f64::consts::FRAC_PI_2);
    let e2 = e.to(u) + along * sin(t) + across * sin(t + core::f64::consts::FRAC_PI_2);
    let (dn, de) = (n2 - n.to(u), e2 - e.to(u));
    let az2 = gp_base::angle::wrap_azimuth(libm::atan2(de, dn).to_degrees());
    let q = |x: f64| Q { value: x, unit: u };
    Ok(Json::obj(vec![
        ("northing", ctx.emit("northing", q(n2), u)),
        ("easting", ctx.emit("easting", q(e2), u)),
        ("distance", ctx.emit("distance", q(libm::hypot(dn, de)), u)),
        (
            "azimuth",
            ctx.out(
                "azimuth",
                Q {
                    value: az2,
                    unit: unit(QT::Angle, "deg"),
                },
            ),
        ),
        ("bearing", Json::Str(crate::direction::bearing(az2, 1))),
    ]))
}
