//! Terrain-aware overlap (add-drone-suite, drone/photogrammetry, "Terrain-aware
//! overlap"): a mission flown at a fixed height above takeoff sits closer to
//! the ground over high terrain, so each photo covers less and the planned
//! overlap shrinks there. Reports the worst-case overlap and GSD.

use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::{mm_field, unit};

const WOLF: Reference = Reference {
    title: "Elements of Photogrammetry with Applications in GIS",
    issuer: "Wolf, P. R., Dewitt, B. A., and Wilkinson, B. E., McGraw-Hill",
    year: 2014,
    edition: "4th edition",
    locator: "Chapter 18 (flight planning: effect of terrain on overlap)",
    url: "https://www.accessengineeringlibrary.com/content/book/9780071761123",
};

const fn pct(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: 0.0,
            max: 99.0,
        },
    )
}
const fn pct_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e9,
            max: 100.0,
        },
    )
    .precision(Precision::Decimals(1))
}
const fn m_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

pub static TERRAIN_OVERLAP: ToolDef = ToolDef {
    id: "drone.photogrammetry.terrain-overlap",
    title: "Overlap over high terrain",
    summary: "A mission flown at one height above takeoff is closer to the ground over hills, so photos cover less and overlap drops. The lowest overlap and the height left over the highest ground.",
    aliases: &[
        "terrain overlap",
        "overlap over hills",
        "hill overlap loss",
        "terrain-aware overlap",
    ],
    keywords: &[
        "terrain",
        "hill",
        "overlap",
        "front overlap",
        "side overlap",
        "above takeoff",
        "AGL",
        "mapping",
        "worst case",
    ],
    inputs: &[
        m_field(
            "height",
            "Height above takeoff",
            "The mission's flight height, like 100 m",
        )
        .required()
        .core(),
        m_field(
            "highest_terrain",
            "Highest ground",
            "Its height above the takeoff point, like 40 m",
        )
        .required()
        .core(),
        pct(
            "front_overlap",
            "Planned front overlap",
            "Percent over flat ground, like 75",
        )
        .required()
        .core(),
        pct(
            "side_overlap",
            "Planned side overlap",
            "Percent over flat ground, like 65",
        )
        .core(),
        pct(
            "target_overlap",
            "Lowest overlap you accept",
            "Percent, like 70; the planned overlap if left out",
        ),
        mm_field("sensor_width", "Sensor width", "For the GSD, like 13.2 mm"),
        mm_field("focal_length", "Focal length", "For the GSD, like 8.8 mm"),
        Field::new(
            "image_width",
            "Image width",
            "For the GSD: pixels across, like 5472",
            Kind::Number { min: 1.0, max: 1e6 },
        ),
    ],
    outputs: &[
        m_field(
            "effective_height",
            "Height over the highest ground",
            "Flight height minus the ground's height",
        )
        .precision(Precision::Decimals(1)),
        pct_out(
            "front_overlap_worst",
            "Front overlap there",
            "Percent; below zero means gaps between photos",
        ),
        pct_out(
            "side_overlap_worst",
            "Side overlap there",
            "Percent; below zero means gaps between lines",
        )
        .optional(),
        Field::new(
            "gsd_worst",
            "GSD there",
            "Finer than planned, since the ground is closer",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "gsd_takeoff",
            "GSD at takeoff level",
            "As planned",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        m_field(
            "min_height",
            "Height for the lowest overlap",
            "Above takeoff, with spacing replanned there, to hold the lowest overlap you accept",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[],
    warnings: &["OVERLAP_BELOW_TARGET", "EXPERIMENTAL_TOOL"],
    model: "The footprint scales with height above the ground, h − t, while photo and line spacing stay fixed from the flat plan at h. Overlap there = 1 − (1 − o)·h/(h − t); GSD = pixel pitch × (h − t)/f. The height that keeps overlap o_min at the terrain solves (1 − o)·h/(h − t) = 1 − o_min for h, with spacing replanned at that height (Wolf, Dewitt & Wilkinson 2014, ch. 18)",
    accuracy: "Exact for a vertical camera over the stated highest ground; use the highest point along any flight line, and a DEM profile for more",
    references: &[WOLF],
    examples: &[Example {
        id: "primary",
        title: "100 m above takeoff over a 40 m hill at 75/65 overlap",
        input: r#"{"height":"100 m","highest_terrain":"40 m","front_overlap":75,"side_overlap":65}"#,
        source: "add-drone-suite hill scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "next",
        },
    ],
    sentence: "Over the highest ground the drone is {effective_height} up, and front overlap drops to {front_overlap_worst} percent.",
    limits: &[("batchRows", 10_000)],
    run: run_terrain,
    ..ToolDef::BLANK
};

fn run_terrain(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let h = ctx.req_quantity("height")?.to(m);
    let t = ctx.req_quantity("highest_terrain")?.to(m);
    if h <= 0.0 {
        return Err(ToolError::invalid(
            "/height",
            "The flight height must be above the takeoff point.",
        ));
    }
    if t >= h {
        return Err(ToolError::invalid(
            "/highest_terrain",
            "The ground reaches the flight height: the drone would hit it. Fly higher, or plan this area separately.",
        ));
    }
    let front = ctx.number("front_overlap")?.expect("required") / 100.0;
    let side = ctx.number("side_overlap")?.map(|s| s / 100.0);
    let hag = h - t;
    // Spacing is fixed by the flat plan; the footprint shrinks by hag / h.
    let worst = |o: f64| 1.0 - (1.0 - o) * h / hag;
    let fw = worst(front);
    let sw = side.map(worst);
    let target = ctx.number("target_overlap")?.map(|v| v / 100.0);
    let mut short = Vec::new();
    for (name, planned, got) in [("front", Some(front), Some(fw)), ("side", side, sw)] {
        let (Some(p), Some(g)) = (planned, got) else {
            continue;
        };
        let tgt = target.unwrap_or(p);
        if g < tgt {
            short.push(format!(
                "{name} overlap falls to {:.1}% over the highest ground, under {:.0}%",
                g * 100.0,
                tgt * 100.0
            ));
        }
    }
    if !short.is_empty() {
        let joined = short.join("; ");
        ctx.warnings.push(Warning::new(
            "OVERLAP_BELOW_TARGET",
            format!(
                "{}{}{}",
                joined[..1].to_uppercase(),
                &joined[1..],
                if fw < 0.0 || sw.is_some_and(|s| s < 0.0) {
                    ", leaving gaps."
                } else {
                    "."
                }
            ),
        ));
    }
    // Replanning the spacing at a new height H keeps overlap at the terrain
    // at 1 − (1 − o)·H/(H − t) ≥ m exactly when H ≥ t(1 − m)/(o − m).
    let min_h = match target {
        Some(mn) if t > 0.0 && [Some(front), side].into_iter().flatten().all(|o| mn < o) => {
            [Some(front), side]
                .into_iter()
                .flatten()
                .map(|o| t * (1.0 - mn) / (o - mn))
                .reduce(f64::max)
        }
        _ => None,
    };
    let q = |v: f64| Q { value: v, unit: m };
    let mut out = vec![
        ("effective_height", ctx.emit("effective_height", q(hag), m)),
        ("front_overlap_worst", Json::Num(fw * 100.0)),
    ];
    if let Some(s) = sw {
        out.push(("side_overlap_worst", Json::Num(s * 100.0)));
    }
    let cam = (
        ctx.quantity("sensor_width")?,
        ctx.quantity("focal_length")?,
        ctx.number("image_width")?,
    );
    if let (Some(s), Some(f), Some(iw)) = cam {
        let (s, f) = (s.to(m), f.to(m));
        if s <= 0.0 || f <= 0.0 {
            return Err(ToolError::invalid(
                "/sensor_width",
                "Sensor size and focal length must be positive.",
            ));
        }
        let cm = unit(QT::Length, "cm");
        out.push(("gsd_worst", ctx.emit("gsd_worst", q(s / iw * hag / f), cm)));
        out.push((
            "gsd_takeoff",
            ctx.emit("gsd_takeoff", q(s / iw * h / f), cm),
        ));
    }
    if let Some(v) = min_h {
        out.push(("min_height", ctx.emit("min_height", q(v), m)));
    }
    Ok(Json::obj(out))
}
