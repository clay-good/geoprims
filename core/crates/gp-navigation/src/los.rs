//! Line of sight over a smooth, refracting Earth (navigation/line-of-sight):
//! horizon distance, mutual visibility and hidden height, dip, and Fresnel
//! clearance. Refraction bends rays toward the Earth; it is modeled with the
//! usual effective radius R/(1 − k). Terrain and obstacles are not
//! considered, and every result says so.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{acos, sqrt};

use crate::unit;

const BOWDITCH: Reference = Reference {
    title: "The American Practical Navigator (Bowditch)",
    issuer: "National Geospatial-Intelligence Agency, Pub. No. 9",
    year: 2024,
    edition: "2024 edition",
    locator: "Volume II, Table 12 (Distance of the Horizon) and Table 14 (Dip of the Sea Short of the Horizon)",
    url: "https://msi.nga.mil/Publications/APN",
};
const ITU_P530: Reference = Reference {
    title: "ITU-R P.530: Propagation data and prediction methods for terrestrial line-of-sight systems",
    issuer: "International Telecommunication Union",
    year: 2025,
    edition: "P.530-19 (09/2025)",
    locator: "Fresnel ellipsoid clearance and the effective Earth radius factor",
    url: "https://www.itu.int/rec/R-REC-P.530",
};

/// Mean Earth radius the tools default to, meters.
pub const R_DEFAULT: f64 = 6_371_000.0;

const fn qf(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const RADIUS: Field = qf(
    "radius",
    "Earth radius",
    "Default 6,371,000 m",
    QT::Length,
    "m",
);
const K: Field = Field::new(
    "k",
    "Refraction coefficient k",
    "0.13 optical (default), 0.25 radio (4/3 Earth), 0 none",
    Kind::Number {
        min: -1.0,
        max: 0.9,
    },
);

fn terrain_note(ctx: &mut Ctx) {
    ctx.warnings.push(Warning::new(
        "TERRAIN_NOT_CONSIDERED",
        "This assumes a smooth Earth: terrain, buildings, and trees can block the view or the signal.",
    ));
}

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

/// Earth radius and effective radius R/(1 − k) from the inputs.
fn radii(ctx: &mut Ctx, default_k: f64) -> Result<(f64, f64, f64), ToolError> {
    let r = ctx.quantity("radius")?.map_or(R_DEFAULT, |q| q.base());
    if !(1e5..=1e8).contains(&r) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The radius must be between 100 km and 100,000 km.",
        )
        .at("/radius"));
    }
    let k = ctx.number("k")?.unwrap_or(default_k);
    Ok((r, k, r / (1.0 - k)))
}

fn height(ctx: &mut Ctx, name: &str) -> Result<f64, ToolError> {
    let h = ctx.req_quantity(name)?.base();
    if !(0.0..=1e6).contains(&h) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Heights must be between 0 and 1,000 km.",
        )
        .at(&format!("/{name}")));
    }
    Ok(h)
}

/// Straight-line (slant) and surface (arc) distance to the horizon on a sphere of radius `re`.
fn horizon(re: f64, h: f64) -> (f64, f64) {
    (sqrt(2.0 * re * h + h * h), re * acos(re / (re + h)))
}

pub static HORIZON: ToolDef = ToolDef {
    id: "navigation.los.horizon",
    title: "Distance to the horizon",
    summary: "How far you can see from a height: the horizon distance with no refraction, with optical refraction, and for radio (4/3 Earth), with the rules of thumb and their errors.",
    aliases: &[
        "horizon distance",
        "how far can I see",
        "visual horizon",
        "radio horizon",
    ],
    keywords: &[
        "horizon",
        "line of sight",
        "refraction",
        "radio horizon",
        "visibility",
        "curvature",
    ],
    inputs: &[
        qf(
            "height",
            "Height above the surface",
            "Eye or antenna height, like 100 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        K,
        RADIUS,
    ],
    outputs: &[
        qf(
            "optical",
            "Visual horizon",
            "Surface distance with refraction k",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "geometric",
            "Geometric horizon",
            "No refraction",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "radio",
            "Radio horizon",
            "4/3 Earth (k = 0.25)",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "optical_slant",
            "Visual horizon, straight line",
            "Line-of-sight length",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(3)),
        qf(
            "rule_km",
            "Rule of thumb 3.57√h",
            "Kilometers from meters, no refraction",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "rule_visual",
            "Rule of thumb 1.17√h",
            "Nautical miles from feet, visual",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "rule_radio",
            "Rule of thumb 1.23√h",
            "Nautical miles from feet, radio",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "rule_visual_error",
            "Visual rule error",
            "Rule minus the visual horizon",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "TERRAIN_NOT_CONSIDERED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Spherical Earth with effective radius R/(1 − k)",
    accuracy: "Exact for the model; real refraction varies with the weather, often by 10% or more near the surface",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "From 100 m up",
        input: r#"{"height":"100 m"}"#,
        source: "navigation line-of-sight scenario: geometric 35.70 km, optical (k = 0.13) 38.27 km, radio 41.22 km",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.los.visibility",
        reason: "next",
    }],
    sentence: "From that height the visible horizon is {optical} away ({geometric} with no refraction, {radio} for radio).",
    limits: &[("batchRows", 10_000)],
    run: run_horizon,
    ..ToolDef::BLANK
};

fn run_horizon(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = height(ctx, "height")?;
    let (r, k, re) = radii(ctx, 0.13)?;
    let (slant, arc) = horizon(re, h);
    let (_, geo) = horizon(r, h);
    let (_, radio) = horizon(r / 0.75, h);
    let ft = h / 0.3048;
    let nm = |v: f64| v * 1852.0;
    let (rule_km, rule_vis, rule_radio) = (
        3.57 * sqrt(h) * 1000.0,
        nm(1.17 * sqrt(ft)),
        nm(1.23 * sqrt(ft)),
    );
    terrain_note(ctx);
    ctx.model = Some(format!(
        "Spherical Earth, R = {r:.0} m, refraction k = {k} (effective radius {re:.0} m)"
    ));
    Ok(Json::obj([
        ("optical", ctx.out("optical", m(arc))),
        ("geometric", ctx.out("geometric", m(geo))),
        ("radio", ctx.out("radio", m(radio))),
        ("optical_slant", ctx.out("optical_slant", m(slant))),
        ("rule_km", ctx.out("rule_km", m(rule_km))),
        ("rule_visual", ctx.out("rule_visual", m(rule_vis))),
        ("rule_radio", ctx.out("rule_radio", m(rule_radio))),
        (
            "rule_visual_error",
            ctx.out("rule_visual_error", m(rule_vis - arc)),
        ),
    ]))
}

pub static VISIBILITY: ToolDef = ToolDef {
    id: "navigation.los.visibility",
    title: "Can two points see each other?",
    summary: "The farthest two raised points can see each other over the Earth's curve, and for a given distance whether they can, the clearance at the midpoint, and how much of the far target is hidden.",
    aliases: &[
        "mutual visibility",
        "hidden height",
        "geographic range",
        "can I see it from here",
        "curvature hidden height",
    ],
    keywords: &[
        "visibility",
        "hidden height",
        "curvature",
        "horizon",
        "geographic range",
        "lighthouse",
    ],
    inputs: &[
        qf(
            "observer_height",
            "Observer height",
            "Like 2 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qf(
            "target_height",
            "Target height",
            "Like 50 m; 0 for the surface",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qf(
            "distance",
            "Distance between them",
            "Like 30 km; optional",
            QT::Distance,
            "km",
        )
        .core(),
        K,
        RADIUS,
    ],
    outputs: &[
        qf(
            "max_range",
            "Farthest mutual visibility",
            "Sum of the two horizon distances",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "observer_horizon",
            "Observer's horizon",
            "Surface distance",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "visible",
            "Visible at this distance",
            "yes or no",
            Kind::Text { max_len: 3 },
        )
        .optional(),
        qf(
            "hidden_height",
            "Hidden height of the target",
            "The part below the observer's horizon",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qf(
            "midpoint_clearance",
            "Clearance at the midpoint",
            "Sight line above the surface halfway",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "TERRAIN_NOT_CONSIDERED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Spherical Earth with effective radius R/(1 − k)",
    accuracy: "Exact for the model; real refraction varies with the weather",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "From 2 m, a target 30 km away",
        input: r#"{"observer_height":"2 m","target_height":"50 m","distance":"30 km"}"#,
        source: "navigation line-of-sight scenario: the observer's horizon is 5.41 km and the target's lowest 41.3 m is hidden (k = 0.13)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.los.horizon",
        reason: "alternative",
    }],
    sentence: "They can see each other up to {max_range} apart.",
    limits: &[("batchRows", 10_000)],
    run: run_visibility,
    ..ToolDef::BLANK
};

fn run_visibility(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h1 = height(ctx, "observer_height")?;
    let h2 = height(ctx, "target_height")?;
    let (r, k, re) = radii(ctx, 0.13)?;
    let (_, d1) = horizon(re, h1);
    let (_, d2) = horizon(re, h2);
    let mut out = vec![
        ("max_range", ctx.out("max_range", m(d1 + d2))),
        ("observer_horizon", ctx.out("observer_horizon", m(d1))),
    ];
    if let Some(d) = ctx.quantity("distance")?.map(|q| q.base()) {
        if !(0.0..=re * core::f64::consts::PI).contains(&d) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The distance must be between 0 and half way around the Earth.",
            )
            .at("/distance"));
        }
        out.push((
            "visible",
            Json::str(if d <= d1 + d2 { "yes" } else { "no" }),
        ));
        // Past the observer's horizon, the target is hidden below the tangent
        // ray from the observer: the height of that ray above the surface there.
        let beyond = (d - d1).max(0.0) / re;
        let hidden = if d > d1 {
            re / libm::cos(beyond) - re
        } else {
            0.0
        };
        out.push(("hidden_height", ctx.out("hidden_height", m(hidden))));
        // The straight sight line between the tops, above the surface halfway
        // (for points a meter or less apart, simply their average height).
        let clearance = if d <= 1.0 {
            (h1 + h2) / 2.0
        } else {
            let half = d / 2.0 / re;
            let (x1, y1) = (0.0, re + h1);
            let (x2, y2) = (
                (re + h2) * libm::sin(2.0 * half),
                (re + h2) * libm::cos(2.0 * half),
            );
            let (mx, my) = (re * libm::sin(half), re * libm::cos(half));
            // Where the radius through the midpoint meets the chord.
            let (dx, dy) = (x2 - x1, y2 - y1);
            let t = (mx * y1 - my * x1) / (my * dx - mx * dy);
            libm::hypot(x1 + t * dx, y1 + t * dy) - re
        };
        out.push((
            "midpoint_clearance",
            ctx.out("midpoint_clearance", m(clearance)),
        ));
    }
    terrain_note(ctx);
    ctx.model = Some(format!(
        "Spherical Earth, R = {r:.0} m, refraction k = {k} (effective radius {re:.0} m)"
    ));
    Ok(Json::obj(out))
}

pub static DIP: ToolDef = ToolDef {
    id: "navigation.los.dip",
    title: "Dip of the horizon",
    summary: "How far below eye level the visible horizon lies from a height, with refraction, and the navigator's 1.76′√h rule with its difference.",
    aliases: &["horizon dip", "dip correction", "sextant dip"],
    keywords: &[
        "dip",
        "horizon",
        "sextant",
        "celestial navigation",
        "refraction",
    ],
    inputs: &[
        qf("height", "Height of eye", "Like 10 m", QT::Length, "m")
            .required()
            .core(),
        K,
        RADIUS,
    ],
    outputs: &[
        qf(
            "dip",
            "Dip",
            "Below the true horizontal",
            QT::Angle,
            "arcmin",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "rule",
            "Rule of thumb 1.76′√h",
            "Arcminutes from meters",
            QT::Angle,
            "arcmin",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "rule_error",
            "Rule difference",
            "Rule minus the computed dip",
            QT::Angle,
            "arcmin",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "TERRAIN_NOT_CONSIDERED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Spherical Earth with effective radius R/(1 − k)",
    accuracy: "Exact for the model; abnormal refraction over water can change dip by several arcminutes",
    references: &[BOWDITCH],
    examples: &[Example {
        id: "primary",
        title: "Height of eye 10 m",
        input: r#"{"height":"10 m"}"#,
        source: "navigation line-of-sight scenario: dip in arcminutes with the 1.76′√h approximation and its difference",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.los.horizon",
        reason: "alternative",
    }],
    sentence: "The horizon dips {dip} below eye level.",
    limits: &[("batchRows", 10_000)],
    run: run_dip,
    ..ToolDef::BLANK
};

fn run_dip(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = height(ctx, "height")?;
    let (_, k, re) = radii(ctx, 0.13)?;
    let dip = acos(re / (re + h));
    let rad = unit(QT::Angle, "rad");
    let rule = 1.76 * sqrt(h) / 60.0; // degrees
    let deg = unit(QT::Angle, "deg");
    terrain_note(ctx);
    ctx.model = Some(format!(
        "Spherical Earth with refraction k = {k} (effective radius {re:.0} m)"
    ));
    Ok(Json::obj([
        (
            "dip",
            ctx.out(
                "dip",
                Q {
                    value: dip,
                    unit: rad,
                },
            ),
        ),
        (
            "rule",
            ctx.out(
                "rule",
                Q {
                    value: rule,
                    unit: deg,
                },
            ),
        ),
        (
            "rule_error",
            ctx.out(
                "rule_error",
                Q {
                    value: rule - dip.to_degrees(),
                    unit: deg,
                },
            ),
        ),
    ]))
}

pub static FRESNEL: ToolDef = ToolDef {
    id: "navigation.los.fresnel",
    title: "Fresnel zone and radio link clearance",
    summary: "The first Fresnel zone radius at a point on a radio link, the 60% clearance it needs, and the Earth's bulge there, summed into the clearance above a smooth Earth.",
    aliases: &[
        "fresnel zone",
        "radio link clearance",
        "earth bulge",
        "point to point link",
    ],
    keywords: &[
        "Fresnel",
        "radio",
        "link",
        "clearance",
        "earth bulge",
        "antenna height",
        "drone link",
    ],
    inputs: &[
        qf(
            "frequency",
            "Frequency",
            "Like 5.8 GHz",
            QT::Frequency,
            "GHz",
        )
        .required()
        .core(),
        qf("distance", "Link length", "Like 10 km", QT::Distance, "km")
            .required()
            .core(),
        qf(
            "position",
            "Distance from one end",
            "Where to evaluate; default the midpoint",
            QT::Distance,
            "km",
        )
        .core(),
        Field::new(
            "k",
            "Refraction coefficient k",
            "0.25 (4/3 Earth) by default",
            Kind::Number {
                min: -1.0,
                max: 0.9,
            },
        ),
        RADIUS,
    ],
    outputs: &[
        qf(
            "required_clearance",
            "Required clearance",
            "60% of the first Fresnel radius plus the Earth bulge",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "fresnel_radius",
            "First Fresnel radius",
            "√(λ d1 d2 / d)",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "clearance_60",
            "60% of the Fresnel radius",
            "The usual minimum",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2)),
        qf(
            "earth_bulge",
            "Earth bulge",
            "d1 d2 / (2 R/(1 − k))",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "TERRAIN_NOT_CONSIDERED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "First Fresnel zone √(λ d1 d2 / d) and Earth bulge on an effective radius R/(1 − k)",
    accuracy: "Exact for the model; the effective Earth radius varies with the weather, so plan margin for lower k",
    references: &[ITU_P530],
    examples: &[Example {
        id: "primary",
        title: "A 5.8 GHz drone link of 10 km, at the midpoint",
        input: r#"{"frequency":"5.8 GHz","distance":"10 km"}"#,
        source: "navigation line-of-sight scenario: first Fresnel radius and Earth bulge (K = 4/3) summed as required clearance",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[],
    }],
    related: &[Related {
        id: "navigation.los.visibility",
        reason: "alternative",
    }],
    sentence: "At that point the link needs {required_clearance} of clearance above a smooth Earth.",
    limits: &[("batchRows", 10_000)],
    run: run_fresnel,
    ..ToolDef::BLANK
};

fn run_fresnel(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let f = ctx.req_quantity("frequency")?.base();
    let d = ctx.req_quantity("distance")?.base();
    if f <= 0.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The frequency must be more than zero.",
        )
        .at("/frequency"));
    }
    if !(d > 0.0 && d <= 1e6) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The link must be longer than zero and at most 1,000 km.",
        )
        .at("/distance"));
    }
    let d1 = ctx.quantity("position")?.map_or(d / 2.0, |q| q.base());
    if !(0.0..=d).contains(&d1) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The position must be along the link.",
        )
        .at("/position"));
    }
    let (_, k, re) = radii(ctx, 0.25)?;
    let d2 = d - d1;
    let lambda = 299_792_458.0 / f;
    let f1 = sqrt(lambda * d1 * d2 / d);
    let bulge = d1 * d2 / (2.0 * re);
    terrain_note(ctx);
    ctx.model = Some(format!(
        "First Fresnel zone and Earth bulge with k = {k} (effective radius {re:.0} m)"
    ));
    Ok(Json::obj([
        (
            "required_clearance",
            ctx.out("required_clearance", m(0.6 * f1 + bulge)),
        ),
        ("fresnel_radius", ctx.out("fresnel_radius", m(f1))),
        ("clearance_60", ctx.out("clearance_60", m(0.6 * f1))),
        ("earth_bulge", ctx.out("earth_bulge", m(bulge))),
    ]))
}
