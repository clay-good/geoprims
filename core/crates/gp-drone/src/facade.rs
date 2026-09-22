//! Facade scan (add-drone-suite, drone/mission-patterns, "Facade and structure
//! scan"): a vertical lawnmower parallel to a facade line at a fixed standoff,
//! with a level camera, so the facade GSD uses the standoff as the object
//! distance.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{ceil, hypot};

use crate::mission::KARNEY;
use crate::{mm_field, unit};

const WOLF: Reference = Reference {
    title: "Elements of Photogrammetry with Applications in GIS",
    issuer: "Wolf, P. R., Dewitt, B. A., and Wilkinson, B. E., McGraw-Hill",
    year: 2014,
    edition: "4th edition",
    locator: "Chapter 6 (scale and coverage, with the object distance in place of the flying height)",
    url: "https://www.accessengineeringlibrary.com/content/book/9780071761123",
};

const fn deg(name: &'static str, title: &'static str, help: &'static str) -> Field {
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
const fn count(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Number { min: 0.0, max: 1e9 })
        .precision(Precision::Decimals(0))
}

const END: &[Field] = &[
    deg("lat", "Latitude", "Decimal degrees, like 40.4406")
        .required()
        .angle_range("[-90,90]"),
    deg("lon", "Longitude", "Decimal degrees, like -80.0020")
        .required()
        .angle_range("[-180,180)"),
];
const ROW: &[Field] = &[
    deg("lat", "Latitude", "Degrees").precision(Precision::Decimals(7)),
    deg("lon", "Longitude", "Degrees").precision(Precision::Decimals(7)),
    len("height", "Height", "Above the facade's base").precision(Precision::Decimals(1)),
    deg("heading", "Heading", "Facing the facade, camera level").precision(Precision::Decimals(1)),
];

pub static FACADE: ToolDef = ToolDef {
    id: "drone.mission.facade",
    title: "Facade scan",
    summary: "Waypoints for photographing a building face: level passes at a fixed standoff, stacked up the wall with the overlap you set, and the GSD on the facade.",
    aliases: &[
        "facade scan",
        "building facade mission",
        "wall inspection",
        "vertical lawnmower",
        "structure scan",
    ],
    keywords: &[
        "facade",
        "wall",
        "building",
        "standoff",
        "vertical",
        "inspection",
        "structure",
        "GSD",
        "waypoints",
    ],
    inputs: &[
        Field::new(
            "facade",
            "Facade ends",
            "The wall's two ends, one per line, like 40.4406, -80.0020",
            Kind::List {
                items: END,
                min: 2,
                max: 2,
            },
        )
        .required()
        .core(),
        len("standoff", "Standoff", "Distance from the wall, like 30 m")
            .required()
            .core(),
        len("top_height", "Top of the wall", "Above its base, like 25 m")
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
        mm_field(
            "sensor_height",
            "Sensor height",
            "Physical height, like 8.8 mm; sets how far apart the passes are",
        ),
        Field::new(
            "image_width",
            "Image width",
            "Pixels across, like 5472, for the GSD",
            Kind::Number { min: 1.0, max: 1e6 },
        ),
        len(
            "bottom_height",
            "Bottom of the scan",
            "Above the base, like 2 m; default 0",
        ),
        Field::new(
            "horizontal_overlap",
            "Overlap along a pass",
            "Percent, like 75 (the default)",
            Kind::Number {
                min: 0.0,
                max: 95.0,
            },
        ),
        Field::new(
            "vertical_overlap",
            "Overlap between passes",
            "Percent, like 60 (the default)",
            Kind::Number {
                min: 0.0,
                max: 95.0,
            },
        ),
        Field::new(
            "side",
            "Side to fly",
            "right or left of the line from start to end, looking along it; default right",
            Kind::Choice(&["right", "left"]),
        ),
    ],
    outputs: &[
        Field::new(
            "gsd",
            "GSD on the facade",
            "Wall per pixel at the standoff",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        len(
            "footprint_width",
            "Photo width on the wall",
            "sensor width × standoff ÷ focal length",
        )
        .precision(Precision::Decimals(2)),
        len(
            "footprint_height",
            "Photo height on the wall",
            "sensor height × standoff ÷ focal length",
        )
        .precision(Precision::Decimals(2)),
        len(
            "photo_spacing",
            "Photo spacing",
            "Along each pass, evened out",
        )
        .precision(Precision::Decimals(2)),
        len(
            "pass_spacing",
            "Pass spacing",
            "Between passes, evened out; 0 for one pass",
        )
        .precision(Precision::Decimals(2)),
        count("passes", "Passes", "Level passes up the wall"),
        count("photos_per_pass", "Photos per pass", "Along the wall"),
        count("photos", "Photos", "In all"),
        len("facade_length", "Facade length", "Geodesic, start to end")
            .precision(Precision::Decimals(2)),
        len(
            "path_length",
            "Path length",
            "Waypoint to waypoint, climbs included",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "waypoints",
            "Waypoints",
            "In flight order, passes alternating direction",
            Kind::List {
                items: ROW,
                min: 0,
                max: 10_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Photo footprint on the wall = sensor size × standoff / focal length; GSD = pixel pitch × standoff / focal length (Wolf, Dewitt & Wilkinson 2014, ch. 6). Passes and photos: n = 1 when the span fits one photo, else ⌈(span − footprint)/(footprint × (1 − overlap))⌉ + 1, centered and evenly spaced. Stations by the geodesic direct problem along the facade, offset square to it by the standoff (Karney 2013)",
    accuracy: "Exact for a flat, vertical facade and a level camera; heights are above the facade's base, not MSL",
    references: &[WOLF, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 50 m wall, 25 m tall, from 30 m away",
        input: r#"{"facade":[{"lat":40.4406,"lon":-80.0020},{"lat":40.4406,"lon":-80.001411}],"standoff":"30 m","top_height":"25 m","sensor_width":"13.2 mm","focal_length":"8.8 mm","image_width":5472,"sensor_height":"8.8 mm"}"#,
        source: "add-drone-suite facade GSD scenario (30 m standoff)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("path", "waypoints")],
    }],
    related: &[
        Related {
            id: "drone.mission.orbit",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
    ],
    sentence: "Fly {passes} {plural passes \"pass\" \"passes\"} of {photos_per_pass} photos at {standoff} from the wall.{if gsd > 0} Each pixel covers {gsd} of the facade.{/if}",
    limits: &[("batchRows", 100)],
    run: run_facade,
    ..ToolDef::BLANK
};

/// How many evenly spread photos cover `span` with `foot`-wide photos at
/// `overlap`, and the centers' offsets from the span's start.
/// How many stations, counted before any are made, so a tiny footprint
/// cannot ask for billions of them.
fn station_count(span: f64, foot: f64, overlap: f64) -> f64 {
    if span <= foot {
        return 1.0;
    }
    // A hair under the exact count keeps 2.0000000001 steps at 3 stations.
    ceil((span - foot) / (foot * (1.0 - overlap)) - 1e-9) + 1.0
}

fn stations(span: f64, foot: f64, overlap: f64) -> Vec<f64> {
    if span <= foot {
        return vec![span / 2.0];
    }
    let n = station_count(span, foot, overlap) as usize;
    (0..n)
        .map(|k| foot / 2.0 + k as f64 * (span - foot) / (n - 1) as f64)
        .collect()
}

fn run_facade(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (m, d) = (unit(QT::Length, "m"), unit(QT::Angle, "deg"));
    let given: Vec<serde_json::Map<String, serde_json::Value>> = ctx.rows("facade")?;
    let mut ends = [(0.0, 0.0); 2];
    for (i, row) in given.iter().enumerate() {
        let la = ctx
            .row_quantity("facade", i, row, "lat")?
            .expect("required");
        let lo = ctx
            .row_quantity("facade", i, row, "lon")?
            .expect("required");
        ends[i] = (la.to(d), lo.to(d));
    }
    let [(lat1, lon1), (lat2, lon2)] = ends;
    if !(-89.0..=89.0).contains(&lat1) || !(-89.0..=89.0).contains(&lat2) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The facade must be between 89° S and 89° N.",
        )
        .at("/facade"));
    }
    let standoff = ctx.req_quantity("standoff")?.to(m);
    let top = ctx.req_quantity("top_height")?.to(m);
    let bottom = ctx.quantity("bottom_height")?.map_or(0.0, |q| q.to(m));
    let sw = ctx.req_quantity("sensor_width")?.to(m);
    let f = ctx.req_quantity("focal_length")?.to(m);
    let iw = ctx.number("image_width")?;
    let Some(sh) = ctx.quantity("sensor_height")?.map(|q| q.to(m)) else {
        return Err(ToolError::invalid(
            "/sensor_height",
            "Give the sensor height, like 8.8 mm (under More options): it sets how far apart the passes are.",
        ));
    };
    if standoff <= 0.0 || sw <= 0.0 || f <= 0.0 || sh <= 0.0 {
        return Err(ToolError::invalid(
            "/standoff",
            "The standoff, sensor size, and focal length must be positive.",
        ));
    }
    if top <= bottom {
        return Err(ToolError::invalid(
            "/top_height",
            "The top of the wall must be above the bottom of the scan.",
        ));
    }
    let oh = ctx.number("horizontal_overlap")?.unwrap_or(75.0) / 100.0;
    let ov = ctx.number("vertical_overlap")?.unwrap_or(60.0) / 100.0;
    let right = ctx.choice("side")? != Some("left");
    let geod = Geodesic::wgs84();
    let (length, az1, _, _): (f64, f64, f64, f64) = geod.inverse(lat1, lon1, lat2, lon2);
    if length < 0.01 {
        return Err(ToolError::invalid(
            "/facade",
            "The two ends of the facade are the same point.",
        ));
    }
    let (w, h) = (sw * standoff / f, sh * standoff / f);
    let planned = station_count(length, w, oh) * station_count(top - bottom, h, ov);
    // NaN (a degenerate footprint) counts as over the limit too.
    if planned.is_nan() || planned > 10_000.0 {
        let total = planned;
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!("That is {total:.0} photos, over the 10,000 limit. Stand farther off, lower the overlap, or split the facade."),
        )
        .at("/standoff"));
    }
    let cols = stations(length, w, oh);
    let rows = stations(top - bottom, h, ov);
    let total = cols.len() * rows.len();
    // Each station: along the facade, then square to it by the standoff.
    let stn: Vec<(f64, f64, f64)> = cols
        .iter()
        .map(|&s| {
            let (la, lo, az): (f64, f64, f64) = geod.direct(lat1, lon1, az1, s);
            let out = az + if right { 90.0 } else { -90.0 };
            let (wla, wlo, _): (f64, f64, f64) = geod.direct(la, lo, out, standoff);
            (
                wla,
                (wlo + 540.0).rem_euclid(360.0) - 180.0,
                (out + 180.0).rem_euclid(360.0),
            )
        })
        .collect();
    let dq = |v: f64| Q { value: v, unit: d }.to_json();
    let mut wps = Vec::with_capacity(total);
    let mut path = 0.0;
    let mut prev: Option<(f64, f64, f64)> = None;
    for (r, z) in rows.iter().enumerate() {
        let order: Vec<usize> = if r % 2 == 0 {
            (0..stn.len()).collect()
        } else {
            (0..stn.len()).rev().collect()
        };
        for k in order {
            let (la, lo, hd) = stn[k];
            let z = bottom + z;
            if let Some((pla, plo, pz)) = prev {
                let s: f64 = geod.inverse(pla, plo, la, lo);
                path += hypot(s, z - pz);
            }
            prev = Some((la, lo, z));
            wps.push(Json::obj([
                ("lat", dq(la)),
                ("lon", dq(lo)),
                ("height", Q { value: z, unit: m }.to_json()),
                ("heading", dq(hd)),
            ]));
        }
    }
    let gap = |v: &[f64]| if v.len() > 1 { v[1] - v[0] } else { 0.0 };
    let q = |v: f64| Q { value: v, unit: m };
    let mut out = Vec::new();
    if let Some(iw) = iw {
        let cm = unit(QT::Length, "cm");
        out.push(("gsd", ctx.emit("gsd", q(sw / iw * standoff / f), cm)));
    }
    out.extend([
        ("footprint_width", ctx.out("footprint_width", q(w))),
        ("footprint_height", ctx.out("footprint_height", q(h))),
        ("photo_spacing", ctx.out("photo_spacing", q(gap(&cols)))),
        ("pass_spacing", ctx.out("pass_spacing", q(gap(&rows)))),
        ("passes", Json::Num(rows.len() as f64)),
        ("photos_per_pass", Json::Num(cols.len() as f64)),
        ("photos", Json::Num(total as f64)),
        ("facade_length", ctx.out("facade_length", q(length))),
        ("path_length", ctx.out("path_length", q(path))),
        ("waypoints", Json::Arr(wps)),
    ]);
    Ok(Json::obj(out))
}
