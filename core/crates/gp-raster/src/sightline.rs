//! Terrain line of sight (raster/terrain-analysis, "Terrain line of sight"):
//! whether an observer sees a target over a ground profile, with the Earth's
//! curve and refraction, the point that blocks the view, the observer height
//! that would clear it, and the first Fresnel zone clearance of a radio link.
//!
//! The profile is the ground under the path as distance and elevation pairs,
//! from the observer to the target. Until the terrain tiles are hosted it comes
//! from the user (a map, a survey, or a DEM read elsewhere).
//!
//! The geometry is exact on a sphere of effective radius R/(1 − k): each point
//! sits at the angle s/Re from the observer, the sight line is the straight
//! chord between the two antennas, and clearance is measured along the local
//! vertical. The usual planner's form, ground plus a bulge of d1 d2 / (2 Re)
//! under a straight line, is its parabolic approximation.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use libm::{cos, sin, sqrt};
use serde_json::{Map, Value};

const ITU_P530: Reference = Reference {
    title: "ITU-R P.530: Propagation data and prediction methods for terrestrial line-of-sight systems",
    issuer: "International Telecommunication Union",
    year: 2025,
    edition: "P.530-19 (09/2025)",
    locator: "Section 2.2.2, path clearance: the first Fresnel ellipsoid over the path profile with the effective Earth radius factor",
    url: "https://www.itu.int/rec/R-REC-P.530",
};

/// Mean Earth radius, meters.
const R_DEFAULT: f64 = 6_371_000.0;
const C: f64 = 299_792_458.0;

const fn len(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}
const fn dist(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Distance,
            unit: "km",
        },
    )
}
const fn yes_no(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 3 })
}

const PT: &[Field] = &[
    dist(
        "distance",
        "Distance",
        "From the observer along the path, like 4.2 km",
    )
    .required(),
    len("elevation", "Ground elevation", "Like 312 m").required(),
];
const ROW: &[Field] = &[
    dist("distance", "Distance", "From the observer"),
    len(
        "sight_line",
        "Sight line",
        "Elevation of the sight line above that point",
    )
    .precision(Precision::Decimals(2)),
    len(
        "clearance",
        "Clearance",
        "Sight line above the ground; negative where the ground is in the way",
    )
    .precision(Precision::Decimals(2)),
    len(
        "fresnel_radius",
        "First Fresnel radius",
        "√(λ d1 d2 / d) at that point",
    )
    .precision(Precision::Decimals(2))
    .optional(),
    yes_no(
        "fresnel_clear",
        "60% of the zone clear",
        "yes when the clearance is at least 0.6 of the Fresnel radius",
    )
    .optional(),
];

pub static LINE_OF_SIGHT: ToolDef = ToolDef {
    id: "raster.terrain.line-of-sight",
    title: "Line of sight over a ground profile",
    summary: "Whether an observer can see a target over the ground between them, with the Earth's curve and refraction: the point that blocks the view, the height that would clear it, and the Fresnel zone clearance of a radio link.",
    aliases: &[
        "terrain line of sight",
        "path profile",
        "link profile",
        "can the antennas see each other",
        "Fresnel clearance over terrain",
    ],
    keywords: &[
        "line of sight",
        "terrain",
        "profile",
        "obstruction",
        "ridge",
        "Fresnel",
        "radio link",
        "antenna height",
        "refraction",
        "curvature",
    ],
    inputs: &[
        Field::new(
            "points",
            "Ground profile",
            "Distance from the observer and ground elevation, from observer to target, one per line, like 0 km, 312 m",
            Kind::List {
                items: PT,
                min: 3,
                max: 100_000,
            },
        )
        .required()
        .core(),
        len(
            "observer_height",
            "Observer height above ground",
            "Eye or antenna height at the first point, like 2 m",
        )
        .required()
        .core(),
        len(
            "target_height",
            "Target height above ground",
            "At the last point, like 30 m",
        )
        .required()
        .core(),
        Field::new(
            "frequency",
            "Radio frequency",
            "For the Fresnel zone check, like 5.8 GHz",
            Kind::Quantity {
                q: QT::Frequency,
                unit: "GHz",
            },
        )
        .core(),
        Field::new(
            "k",
            "Refraction coefficient k",
            "0.13 optical (default), 0.25 radio (4/3 Earth), 0 none",
            Kind::Number {
                min: -1.0,
                max: 0.9,
            },
        ),
        len("radius", "Earth radius", "Default 6,371,000 m"),
    ],
    outputs: &[
        yes_no(
            "visible",
            "Target in sight",
            "yes when the sight line clears every point of the profile",
        ),
        len(
            "clearance",
            "Least clearance",
            "The sight line's least height above the ground; negative when the ground is in the way",
        )
        .precision(Precision::Decimals(2)),
        dist(
            "clearance_at",
            "Least clearance at",
            "Distance from the observer",
        )
        .precision(Precision::Decimals(3)),
        len(
            "obstruction_elevation",
            "Obstruction elevation",
            "Ground elevation at the point that blocks the view the most",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        len(
            "obstruction_height",
            "Obstruction above the sight line",
            "How far the ground there rises above the sight line",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        len(
            "observer_height_needed",
            "Observer height to see the target",
            "Above the ground at the observer, with the target unchanged",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "obstructions",
            "Points in the way",
            "Profile points above the sight line",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        yes_no(
            "fresnel_clear",
            "60% of the Fresnel zone clear",
            "yes when every point clears 0.6 of the first Fresnel radius",
        )
        .optional(),
        Field::new(
            "fresnel_worst_percent",
            "Least Fresnel clearance, percent",
            "Clearance as a percent of the first Fresnel radius at the worst point; 60 is the usual minimum",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        dist(
            "fresnel_worst_at",
            "Least Fresnel clearance at",
            "Distance from the observer",
        )
        .precision(Precision::Decimals(3))
        .optional(),
        Field::new(
            "fresnel_short",
            "Points short of 60%",
            "Profile points inside 60% of the first Fresnel zone",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0))
        .optional(),
        Field::new(
            "profile",
            "Along the path",
            "Each interior point of the profile",
            Kind::List {
                items: ROW,
                min: 1,
                max: 100_000,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Spherical Earth of effective radius R/(1 − k), the sight line as the straight chord between the antennas, clearance along the local vertical, and the first Fresnel radius √(λ d1 d2 / d)",
    accuracy: "Exact for the model at the points given; the ground between points is not checked, and real refraction varies with the weather",
    when_to_use: "Use this when there is ground between two points and the question is whether it gets in the way: a ridge between a pilot and a drone, a hill between a tower and a receiver, or the path of a radio link across a valley. Enter the ground profile from the observer to the target with both heights above the ground. It says whether the target is in sight, where the ground comes closest to the sight line, how high the observer would have to be to see over it, and, with a frequency, whether 60% of the first Fresnel zone is clear and where it is not.",
    limitations: "Only the points you give are checked, and the ground between them is not, so a narrow peak missed by the profile can still block the view; sample at least every break in the ground. Buildings, trees, and towers are not part of the ground unless you add their heights to it. Refraction enters only through k, and real air varies: plan a radio link against a lower k as well. The Fresnel check is a clearance rule, not a diffraction loss, so it does not say how much signal an obstruction costs.",
    references: &[ITU_P530],
    examples: &[Example {
        id: "primary",
        title: "A ridge between a hilltop and a tower",
        input: r#"{"points":[{"distance":"0 km","elevation":"300 m"},{"distance":"4 km","elevation":"340 m"},{"distance":"8 km","elevation":"395 m"},{"distance":"12 km","elevation":"330 m"},{"distance":"16 km","elevation":"310 m"}],"observer_height":"2 m","target_height":"30 m","frequency":"5.8 GHz"}"#,
        source: "add-spatial-indexing-and-raster ridge scenario; the exact circular geometry worked independently in Python (tools/vectors/gen_sightline.py)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.los.visibility",
            reason: "parent",
        },
        Related {
            id: "navigation.los.fresnel",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.profile-grades",
            reason: "alternative",
        },
    ],
    sentence: "{if obstructions == 0}The target is in sight, with at least {clearance} to spare at {clearance_at}.{/if}{if obstructions > 0}The ground at {clearance_at} blocks the view, rising {obstruction_height} above the sight line. An observer {observer_height_needed} up would see over it.{/if}{if fresnel_short > 0} {fresnel_short} {plural fresnel_short \"point is\" \"points are\"} inside 60% of the Fresnel zone.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_los,
    ..ToolDef::BLANK
};

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

/// A point `z` meters above the sphere of radius `re`, `s` meters along it.
fn at(re: f64, s: f64, z: f64) -> (f64, f64) {
    let a = s / re;
    ((re + z) * sin(a), (re + z) * cos(a))
}

/// Height above the sphere where the line through `p` and `q` crosses the
/// vertical `s` meters along it.
fn line_height(re: f64, p: (f64, f64), q: (f64, f64), s: f64) -> f64 {
    let a = s / re;
    let u = (sin(a), cos(a));
    let dir = (q.0 - p.0, q.1 - p.1);
    cross(p, dir) / cross(u, dir) - re
}

fn run_los(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows: Vec<Map<String, Value>> = ctx.rows("points")?;
    let mut pts = Vec::with_capacity(rows.len());
    let (mut du, mut eu) = (None, None);
    for (i, row) in rows.iter().enumerate() {
        let d = ctx
            .row_quantity("points", i, row, "distance")?
            .expect("required");
        let e = ctx
            .row_quantity("points", i, row, "elevation")?
            .expect("required");
        du.get_or_insert(d.unit);
        eu.get_or_insert(e.unit);
        let (d, e) = (d.base(), e.base());
        if !(-1e4..=1e4).contains(&e) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Ground elevations must be between -10,000 m and 10,000 m.",
            )
            .at(&format!("/points/{i}/elevation")));
        }
        pts.push((d, e));
    }
    let (du, eu) = (du.expect("rows"), eu.expect("rows"));
    if let Some(i) = pts.windows(2).position(|w| w[1].0 <= w[0].0) {
        return Err(ToolError::invalid(
            &format!("/points/{}/distance", i + 1),
            "Each distance must be farther along than the one before.",
        ));
    }
    let s0 = pts[0].0;
    let total = pts[pts.len() - 1].0 - s0;
    if total > 1e6 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The profile can be at most 1,000 km long.",
        )
        .at("/points"));
    }
    let mut height = |name: &str| -> Result<f64, ToolError> {
        let h = ctx.req_quantity(name)?.base();
        if !(0.0..=1e5).contains(&h) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Heights above the ground must be between 0 and 100 km.",
            )
            .at(&format!("/{name}")));
        }
        Ok(h)
    };
    let (ho, ht) = (height("observer_height")?, height("target_height")?);
    let r = ctx.quantity("radius")?.map_or(R_DEFAULT, |q| q.base());
    if !(1e5..=1e8).contains(&r) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The radius must be between 100 km and 100,000 km.",
        )
        .at("/radius"));
    }
    let k = ctx.number("k")?.unwrap_or(0.13);
    let re = r / (1.0 - k);
    let lambda = match ctx.quantity("frequency")? {
        Some(f) if f.base() > 0.0 => Some(C / f.base()),
        Some(_) => {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The frequency must be more than zero.",
            )
            .at("/frequency"));
        }
        None => None,
    };

    let (first, last) = (pts[0], pts[pts.len() - 1]);
    let obs = at(re, 0.0, first.1 + ho);
    let tgt = at(re, total, last.1 + ht);
    let base = (0.0, 1.0);
    let (mut least, mut least_i) = (f64::INFINITY, 0);
    let (mut worst, mut worst_i, mut short) = (f64::INFINITY, 0, 0.0);
    let (mut blocked, mut need) = (0.0, f64::NEG_INFINITY);
    let mut rows_out = Vec::with_capacity(pts.len() - 2);
    for (i, &(s, e)) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let s = s - s0;
        let line = line_height(re, obs, tgt, s);
        let clear = line - e;
        if clear < least {
            (least, least_i) = (clear, i);
        }
        if clear < 0.0 {
            blocked += 1.0;
        }
        // The observer height whose sight line to the target grazes this point.
        let g = at(re, s, e);
        let dir = (g.0 - tgt.0, g.1 - tgt.1);
        need = need.max(cross(tgt, dir) / cross(base, dir) - re - first.1);
        let mut row = vec![
            ("distance", ctx.emit("distance", dq(s), du)),
            ("sight_line", ctx.emit("sight_line", m(line), eu)),
            ("clearance", ctx.emit("clearance", m(clear), eu)),
        ];
        if let Some(l) = lambda {
            let f1 = sqrt(l * s * (total - s) / total);
            let ratio = clear / f1;
            if ratio < worst {
                (worst, worst_i) = (ratio, i);
            }
            if ratio < 0.6 {
                short += 1.0;
            }
            row.push(("fresnel_radius", ctx.emit("fresnel_radius", m(f1), eu)));
            row.push(("fresnel_clear", Json::str(yes_no_str(ratio >= 0.6))));
        }
        rows_out.push(Json::obj(row));
    }
    let visible = blocked == 0.0;
    let need = need.max(0.0);

    if ctx.explaining() {
        let fmt = ctx.options.format;
        let nn = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let at_s = pts[least_i].0 - s0;
        ctx.step(
            "Effective Earth radius",
            "Re = R / (1 − k), with k the refraction coefficient",
            format!("{} km / (1 − {})", nn(r / 1000.0, 1), k),
            format!("{} km", nn(re / 1000.0, 1)),
        );
        ctx.step(
            "Earth bulge where the ground comes closest",
            "about d1 × d2 / (2 Re)",
            format!(
                "{} km × {} km / (2 × {} km)",
                nn(at_s / 1000.0, 3),
                nn((total - at_s) / 1000.0, 3),
                nn(re / 1000.0, 1)
            ),
            format!("{} m", nn(at_s * (total - at_s) / (2.0 * re), 2)),
        );
        ctx.step(
            "Least clearance",
            "sight line − ground, along the vertical",
            format!(
                "{} m − {} m",
                nn(pts[least_i].1 + least, 2),
                nn(pts[least_i].1, 2)
            ),
            format!("{} m", nn(least, 2)),
        );
        ctx.step(
            "Is the target in sight?",
            "yes when the least clearance is above zero",
            format!("{} m", nn(least, 2)),
            yes_no_str(visible).to_owned(),
        );
    }
    ctx.model = Some(format!(
        "Spherical Earth, R = {r:.0} m, refraction k = {k} (effective radius {re:.0} m); sight line from {ho} m above the first point to {ht} m above the last"
    ));

    let mut out = vec![
        ("visible", Json::str(yes_no_str(visible))),
        ("clearance", ctx.emit("clearance", m(least), eu)),
        (
            "clearance_at",
            ctx.emit("clearance_at", dq(pts[least_i].0 - s0), du),
        ),
    ];
    if !visible {
        out.push((
            "obstruction_elevation",
            ctx.emit("obstruction_elevation", m(pts[least_i].1), eu),
        ));
        out.push((
            "obstruction_height",
            ctx.emit("obstruction_height", m(-least), eu),
        ));
    }
    out.push((
        "observer_height_needed",
        ctx.emit("observer_height_needed", m(need), eu),
    ));
    out.push(("obstructions", Json::Num(blocked)));
    if lambda.is_some() {
        out.push(("fresnel_clear", Json::str(yes_no_str(short == 0.0))));
        out.push(("fresnel_worst_percent", Json::Num(worst * 100.0)));
        out.push((
            "fresnel_worst_at",
            ctx.emit("fresnel_worst_at", dq(pts[worst_i].0 - s0), du),
        ));
        out.push(("fresnel_short", Json::Num(short)));
    }
    out.push(("profile", Json::Arr(rows_out)));
    Ok(Json::obj(out))
}

fn yes_no_str(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("registered unit"),
    }
}

fn dq(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Distance, "m").expect("registered unit"),
    }
}
