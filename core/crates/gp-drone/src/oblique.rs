//! Oblique imagery GSD (add-drone-suite, drone/photogrammetry, "Oblique
//! imagery GSD"): for a camera pitched off nadir, the GSD at the image
//! center and at its near and far edges, the trapezoid it covers on flat
//! ground, and a flag when the far edge reaches the horizon.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::point::plain_angle;
use libm::{cos, hypot, sin};

use crate::{HEIGHT, mm_field, unit};

const WOLF: Reference = Reference {
    title: "Elements of Photogrammetry with Applications in GIS",
    issuer: "Wolf, P. R., Dewitt, B. A., and Wilkinson, B. E., McGraw-Hill",
    year: 2014,
    edition: "4th edition",
    locator: "Chapter 10 (tilted and oblique photographs)",
    url: "https://www.accessengineeringlibrary.com/content/book/9780071761123",
};

const CORNER: &[Field] = &[
    Field::new(
        "corner",
        "Corner",
        "near left, near right, far right, far left",
        Kind::Text { max_len: 16 },
    ),
    Field::new(
        "ahead",
        "Ahead",
        "Meters ahead of the point below the camera, toward where it looks",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "right",
        "Right",
        "Meters to the right of that line",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(2)),
];

pub static OBLIQUE_GSD: ToolDef = ToolDef {
    id: "drone.photogrammetry.oblique-gsd",
    title: "Oblique GSD and footprint",
    summary: "For a camera tilted off straight down, the ground each pixel covers at the image center and its near and far edges, the trapezoid the image covers, and a warning when the view reaches the horizon.",
    aliases: &[
        "oblique GSD",
        "tilted camera footprint",
        "oblique imagery",
        "gimbal pitch GSD",
    ],
    keywords: &[
        "oblique",
        "tilt",
        "pitch",
        "gimbal",
        "GSD",
        "footprint",
        "trapezoid",
        "horizon",
    ],
    inputs: &[
        HEIGHT,
        Field::new(
            "pitch",
            "Tilt from straight down",
            "0° is nadir; like 45°",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded")
        .required()
        .core(),
        mm_field(
            "sensor_width",
            "Sensor width",
            "Physical width, like 13.2 mm",
        )
        .required()
        .core(),
        mm_field(
            "focal_length",
            "Focal length",
            "Physical focal length, like 8.8 mm",
        )
        .required()
        .core(),
        Field::new(
            "image_width",
            "Image width",
            "Pixels across, like 5472",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .required()
        .core(),
        mm_field(
            "sensor_height",
            "Sensor height",
            "Physical height, like 8.8 mm (or give the image height)",
        ),
        Field::new(
            "image_height",
            "Image height",
            "Pixels on the short side, like 3648, for square pixels",
            Kind::Number { min: 1.0, max: 1e6 },
        ),
    ],
    outputs: &[
        Field::new(
            "gsd_center",
            "GSD at the center, across",
            "Ground per pixel across the view",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "gsd_center_along",
            "GSD at the center, along",
            "Ground per pixel toward the view: stretched by the tilt",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "gsd_near",
            "GSD at the near edge",
            "Along the view, the worse direction",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "gsd_far",
            "GSD at the far edge",
            "Along the view",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "gsd_nadir",
            "GSD straight down",
            "For comparison, at the same height",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "near_distance",
            "Near edge ahead",
            "From the point below the camera",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "far_distance",
            "Far edge ahead",
            "From the point below the camera",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "footprint",
            "Footprint corners",
            "The trapezoid on flat ground",
            Kind::List {
                items: CORNER,
                min: 0,
                max: 4,
            },
        ),
        Field::new(
            "horizon",
            "Horizon",
            "Whether the far edge reaches the horizon",
            Kind::Text { max_len: 60 },
        ),
    ],
    errors: &[],
    warnings: &["BEYOND_HORIZON", "EXPERIMENTAL_TOOL"],
    model: "Pinhole camera pitched θ from nadir over flat ground: each sensor point's ray, f·a + y·u + x·r with a the optical axis, meets the ground at height H; GSD is the ground span of one pixel step there, along and across (Wolf, Dewitt & Wilkinson 2014, ch. 10)",
    accuracy: "Exact for a pinhole camera over flat ground; lens distortion and terrain change it",
    references: &[WOLF],
    examples: &[Example {
        id: "primary",
        title: "Tilted 45° at 100 m with a 1-inch, 8.8 mm camera",
        input: r#"{"height":"100 m","pitch":"45 deg","sensor_width":"13.2 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648}"#,
        source: "add-drone-suite 45° oblique scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.altitude-for-gsd",
            reason: "next",
        },
    ],
    sentence: "At the center each pixel covers {gsd_center} across and {gsd_center_along} along the view, against {gsd_nadir} straight down; {horizon}.",
    limits: &[("batchRows", 10_000)],
    run: run_oblique,
    ..ToolDef::BLANK
};

fn run_oblique(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let h = ctx.req_quantity("height")?.to(m);
    let sw = ctx.req_quantity("sensor_width")?.to(m);
    let f = ctx.req_quantity("focal_length")?.to(m);
    let iw = ctx.number("image_width")?.expect("required");
    let sh = match (
        ctx.quantity("sensor_height")?.map(|q| q.to(m)),
        ctx.number("image_height")?,
    ) {
        (Some(s), _) => s,
        (None, Some(ih)) => sw * ih / iw,
        (None, None) => {
            return Err(ToolError::invalid(
                "/sensor_height",
                "Give the sensor height, or the image height in pixels (square pixels are assumed).",
            ));
        }
    };
    if h <= 0.0 || sw <= 0.0 || f <= 0.0 || sh <= 0.0 {
        return Err(ToolError::invalid(
            "/height",
            "The height, sensor size, and focal length must be positive.",
        ));
    }
    let th = plain_angle(ctx, "pitch")?.expect("required");
    if !(0.0..90.0).contains(&th) {
        return Err(ToolError::invalid(
            "/pitch",
            "The tilt is from 0° (straight down) up to, not including, 90°.",
        ));
    }
    let t = th.to_radians();
    let p = sw / iw; // pixel pitch
    // Axis, image-up (toward the far edge), and across, in (ahead, right, down).
    let (a, up) = ((sin(t), 0.0, cos(t)), (cos(t), 0.0, -sin(t)));
    // Where the ray through sensor point (x right, y up) meets the ground, or None past the horizon.
    let ground = |x: f64, y: f64| -> Option<(f64, f64)> {
        let d = (f * a.0 + y * up.0, x, f * a.2 + y * up.2);
        (d.2 > 1e-12 * f).then(|| (h * d.0 / d.2, h * d.1 / d.2))
    };
    let span = |a: Option<(f64, f64)>, b: Option<(f64, f64)>| -> Option<f64> {
        Some(hypot(b?.0 - a?.0, b?.1 - a?.1))
    };
    let row_gsd = |y: f64| -> (Option<f64>, Option<f64>) {
        (
            span(ground(-p / 2.0, y), ground(p / 2.0, y)),
            span(ground(0.0, y - p / 2.0), ground(0.0, y + p / 2.0)),
        )
    };
    let (half_w, half_h) = (sw / 2.0, sh / 2.0);
    let (c_across, c_along) = row_gsd(0.0);
    let (_, near_along) = row_gsd(-half_h + p / 2.0);
    let (_, far_along) = row_gsd(half_h - p / 2.0);
    let near = ground(0.0, -half_h)
        .expect("the near edge is below the horizon when the tilt is under 90°");
    let far = ground(0.0, half_h);
    let cm = unit(QT::Length, "cm");
    let q = |v: f64| Q { value: v, unit: m };
    let corners = [
        ("near left", -half_w, -half_h),
        ("near right", half_w, -half_h),
        ("far right", half_w, half_h),
        ("far left", -half_w, half_h),
    ];
    let pts: Vec<(&str, Option<(f64, f64)>)> =
        corners.iter().map(|&(n, x, y)| (n, ground(x, y))).collect();
    let all = pts.iter().all(|(_, g)| g.is_some());
    if !all {
        ctx.warnings.push(gp_base::error::Warning::new(
            "BEYOND_HORIZON",
            "The top of the image reaches the horizon: the far edge and the footprint are unbounded. Tilt the camera down for a measurable footprint.",
        ));
    }
    let mut out = vec![
        (
            "gsd_center",
            ctx.emit("gsd_center", q(c_across.expect("center")), cm),
        ),
        (
            "gsd_center_along",
            ctx.emit("gsd_center_along", q(c_along.expect("center")), cm),
        ),
        (
            "gsd_near",
            ctx.emit("gsd_near", q(near_along.expect("near edge")), cm),
        ),
        ("gsd_nadir", ctx.emit("gsd_nadir", q(p * h / f), cm)),
        ("near_distance", ctx.out("near_distance", q(near.0))),
    ];
    if let (Some(g), Some(fr)) = (far_along, far) {
        out.push(("gsd_far", ctx.emit("gsd_far", q(g), cm)));
        out.push(("far_distance", ctx.out("far_distance", q(fr.0))));
    }
    let foot = if all {
        pts.iter()
            .map(|(n, g)| {
                let (x, y) = g.expect("all corners on the ground");
                Json::obj(vec![
                    ("corner", Json::Str((*n).into())),
                    ("ahead", ctx.emit("ahead", q(x), m)),
                    ("right", ctx.emit("right", q(y), m)),
                ])
            })
            .collect()
    } else {
        Vec::new()
    };
    out.push(("footprint", Json::Arr(foot)));
    out.push((
        "horizon",
        Json::Str(if all {
            "the whole image is on the ground".into()
        } else {
            "the far edge reaches the horizon".into()
        }),
    ));
    Ok(Json::obj(out))
}
