//! Geofence generation (add-drone-suite, drone/mission-patterns, "Geofence
//! generation"): a geodesic fence a set distance around a point, line, or
//! polygon (by the geometry buffers), an optional inner warning fence, the
//! fence's area and perimeter, and every mission waypoint outside it.

use geographiclib_rs::Geodesic;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::buffer::{self, Cap, Join, Shape, Style};

use crate::mission::KARNEY;
use crate::unit;

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

const VERTEX: &[Field] = &[
    deg("lat", "Latitude", "Decimal degrees, like 40.4406").required(),
    deg("lon", "Longitude", "Decimal degrees, like -80.002").required(),
];
const FENCE_ROW: &[Field] = &[
    deg("lat", "Latitude", "Degrees").precision(Precision::Decimals(7)),
    deg("lon", "Longitude", "Degrees").precision(Precision::Decimals(7)),
    Field::new(
        "fence",
        "Fence",
        "geofence or warning",
        Kind::Text { max_len: 10 },
    ),
    Field::new(
        "part",
        "Part",
        "0, 1, … when a fence is in pieces",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "ring",
        "Ring",
        "0 for the outline, 1, 2, … for holes",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
];
const FLAG_ROW: &[Field] = &[
    Field::new(
        "waypoint",
        "Waypoint",
        "Its number in the list, from 1",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "status",
        "Status",
        "outside the geofence, or past the warning fence",
        Kind::Text { max_len: 32 },
    ),
    len("beyond", "Beyond the fence", "How far past that fence").precision(Precision::Decimals(1)),
];

pub static GEOFENCE: ToolDef = ToolDef {
    id: "drone.mission.geofence",
    title: "Geofence around an area",
    summary: "A fence a set distance around a point, route, or area, with an optional inner warning fence, its area and perimeter, and any waypoint that falls outside.",
    aliases: &[
        "geofence",
        "geofence generator",
        "keep-in fence",
        "mission boundary",
        "waypoint fence check",
    ],
    keywords: &[
        "geofence",
        "fence",
        "boundary",
        "buffer",
        "keep in",
        "waypoint",
        "outside",
        "warning",
        "containment",
    ],
    inputs: &[
        Field::new(
            "area",
            "Area, route, or point",
            "Vertices in order, one per line, like 40.4406, -80.002",
            Kind::List {
                items: VERTEX,
                min: 1,
                max: 2_000,
            },
        )
        .required()
        .core(),
        len(
            "distance",
            "Fence distance",
            "How far out the fence runs, like 50 m",
        )
        .required()
        .core(),
        len(
            "warning_distance",
            "Warning fence",
            "An inner fence closer in, like 30 m",
        )
        .core(),
        Field::new(
            "waypoints",
            "Waypoints to check",
            "One per line, like 40.4406, -80.002",
            Kind::List {
                items: VERTEX,
                min: 0,
                max: 5_000,
            },
        )
        .core(),
        Field::new(
            "shape",
            "Treat the area as",
            "polygon, line, or point; default by count: 1 point, 2 a line, 3 or more a polygon",
            Kind::Choice(&["polygon", "line", "point"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "area_enclosed",
            "Fence area",
            "Inside the geofence",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "perimeter",
            "Fence perimeter",
            "Around the geofence",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "outside_count",
            "Waypoints outside",
            "Past the geofence",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "warning_count",
            "Waypoints in the warning zone",
            "Past the warning fence, inside the geofence",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0))
        .optional(),
        Field::new(
            "flagged",
            "Flagged waypoints",
            "Each outside a fence, with how far",
            Kind::List {
                items: FLAG_ROW,
                min: 0,
                max: 5_000,
            },
        ),
        len(
            "max_deviation",
            "Largest measured fence error",
            "At every fence vertex and edge midpoint",
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "fence",
            "Fence",
            "The geofence, then the warning fence",
            Kind::List {
                items: FENCE_ROW,
                min: 0,
                max: 1_000_000,
            },
        ),
    ],
    errors: &[
        gp_base::ErrorCode::OutOfDomain,
        gp_base::ErrorCode::LimitExceeded,
    ],
    warnings: &["WAYPOINT_OUTSIDE_GEOFENCE", "EXPERIMENTAL_TOOL"],
    model: "The fence is the round geodesic buffer of the area (geometry.buffer.geodesic): every point within the distance of it. A waypoint is outside when its geodesic distance to the area exceeds the fence distance, and by exactly the difference (Karney 2013)",
    accuracy: "The fence is within 0.1% of the distance or 0.5 m, as measured; waypoint distances are exact geodesic distances to the area",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 50 m fence around a field, with a waypoint 12 m outside",
        input: r#"{"area":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.998},{"lat":40.0015,"lon":-104.998},{"lat":40.0015,"lon":-105.0}],"distance":"50 m","warning_distance":"30 m","waypoints":[{"lat":40.00075,"lon":-104.999},{"lat":39.99966,"lon":-104.999},{"lat":39.999442,"lon":-104.999}]}"#,
        source: "add-drone-suite waypoint-outside-fence scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("rings", "fence")],
    }],
    related: &[
        Related {
            id: "drone.mission.survey-grid",
            reason: "parent",
        },
        Related {
            id: "drone.mission.corridor",
            reason: "alternative",
        },
    ],
    sentence: "The fence encloses {area_enclosed}.{if outside_count > 0} {outside_count} {plural outside_count \"waypoint is\" \"waypoints are\"} outside it.{/if}{if outside_count < 1} Every waypoint is inside it.{/if}",
    limits: &[("batchRows", 20)],
    run: run_geofence,
    ..ToolDef::BLANK
};

fn read_points(ctx: &mut Ctx, list: &str) -> Result<Vec<(f64, f64)>, ToolError> {
    if !ctx.is_set(list) {
        return Ok(Vec::new());
    }
    let d = unit(QT::Angle, "deg");
    let rows = ctx.rows(list)?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity(list, i, row, "lat")?
            .expect("required")
            .to(d);
        let lon = ctx
            .row_quantity(list, i, row, "lon")?
            .expect("required")
            .to(d);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::invalid(
                &format!("/{list}/{i}/lat"),
                "Latitude must be between -90° and 90°.",
            ));
        }
        out.push((lat, lon));
    }
    Ok(out)
}

fn run_geofence(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let mut area = read_points(ctx, "area")?;
    area.dedup();
    let kind = match ctx.choice("shape")? {
        Some("point") => Shape::Point,
        Some("line") => Shape::Line,
        Some(_) => Shape::Polygon,
        None if area.len() == 1 => Shape::Point,
        None if area.len() == 2 => Shape::Line,
        None => Shape::Polygon,
    };
    if kind == Shape::Polygon && area.len() > 1 && area.first() == area.last() {
        area.pop();
    }
    if kind == Shape::Polygon {
        gp_geo::point::refuse_repeated_corner(&area, "/area")?;
    }
    let need = match kind {
        Shape::Point => 1,
        Shape::Line => 2,
        Shape::Polygon => 3,
    };
    if area.len() < need || (kind == Shape::Point && area.len() != 1) {
        return Err(ToolError::invalid(
            "/area",
            "A point is one vertex, a line two or more, and an area three or more distinct corners.",
        ));
    }
    let d = ctx.req_quantity("distance")?.to(m);
    if d <= 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The fence distance must be positive, like 50 m.",
        ));
    }
    let warn = ctx.quantity("warning_distance")?.map(|q| q.to(m));
    if let Some(w) = warn
        && (w <= 0.0 || w >= d)
    {
        return Err(ToolError::invalid(
            "/warning_distance",
            "The warning fence sits inside the geofence: a distance above 0 and under the fence distance.",
        ));
    }
    let wps = read_points(ctx, "waypoints")?;
    let g = Geodesic::wgs84();
    let rings = vec![area];
    let round = Style {
        join: Join::Round,
        cap: Cap::Round,
        mitre_limit: 5.0,
    };
    let fence = buffer::geodesic(&g, kind, &rings, d, &round)?;
    let inner = warn
        .map(|w| buffer::geodesic(&g, kind, &rings, w, &round))
        .transpose()?;
    let dist = buffer::distances(&g, kind, &rings, &wps, 1.2 * d + 100.0)?;
    let mut flagged = Vec::new();
    let (mut outside, mut in_zone) = (Vec::new(), 0usize);
    for (i, &x) in dist.iter().enumerate() {
        let row = |status: &str, beyond: f64| {
            Json::obj([
                ("waypoint", Json::Num((i + 1) as f64)),
                ("status", Json::Str(status.into())),
                (
                    "beyond",
                    Q {
                        value: beyond,
                        unit: m,
                    }
                    .to_json(),
                ),
            ])
        };
        if x > d {
            outside.push(format!("waypoint {} by {:.1} m", i + 1, x - d));
            flagged.push(row("outside the geofence", x - d));
        } else if let Some(w) = warn
            && x > w
        {
            in_zone += 1;
            flagged.push(row("past the warning fence", x - w));
        }
    }
    if !outside.is_empty() {
        let shown = outside
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = if outside.len() > 10 {
            format!(", and {} more", outside.len() - 10)
        } else {
            String::new()
        };
        ctx.warnings.push(Warning::new(
            "WAYPOINT_OUTSIDE_GEOFENCE",
            format!("Outside the geofence: {shown}{more}."),
        ));
    }
    let deg = unit(QT::Angle, "deg");
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    let mut rows = Vec::new();
    let (mut area_m2, mut perimeter) = (0.0, 0.0);
    for (name, b) in
        core::iter::once(("geofence", &fence)).chain(inner.as_ref().map(|b| ("warning", b)))
    {
        for (pi, (outline, holes)) in b.parts.iter().enumerate() {
            for (ri, ring) in core::iter::once(outline).chain(holes.iter()).enumerate() {
                if name == "geofence" {
                    let (a, p) = ring_area(&g, ring);
                    area_m2 += if ri == 0 { a.abs() } else { -a.abs() };
                    perimeter += p;
                }
                for &(lat, lon) in ring {
                    rows.push(Json::obj([
                        ("lat", dq(lat)),
                        ("lon", dq(lon)),
                        ("fence", Json::Str(name.into())),
                        ("part", Json::Num(pi as f64)),
                        ("ring", Json::Num(ri as f64)),
                    ]));
                }
            }
        }
    }
    let max_dev = inner.as_ref().map_or(fence.max_deviation, |b| {
        b.max_deviation.max(fence.max_deviation)
    });
    let mut out = vec![
        (
            "area_enclosed",
            ctx.out(
                "area_enclosed",
                Q {
                    value: area_m2,
                    unit: unit(QT::Area, "m2"),
                },
            ),
        ),
        (
            "perimeter",
            ctx.out(
                "perimeter",
                Q {
                    value: perimeter,
                    unit: unit(QT::Distance, "m"),
                },
            ),
        ),
        ("outside_count", Json::Num(outside.len() as f64)),
    ];
    if warn.is_some() {
        out.push(("warning_count", Json::Num(in_zone as f64)));
    }
    out.push(("flagged", Json::Arr(flagged)));
    out.push((
        "max_deviation",
        ctx.out(
            "max_deviation",
            Q {
                value: max_dev,
                unit: m,
            },
        ),
    ));
    out.push(("fence", Json::Arr(rows)));
    Ok(Json::obj(out))
}

/// Area (counterclockwise positive) and perimeter of a geodesic ring.
fn ring_area(g: &Geodesic, ring: &[(f64, f64)]) -> (f64, f64) {
    let mut p = geographiclib_rs::PolygonArea::new(g, geographiclib_rs::Winding::CounterClockwise);
    for &(lat, lon) in ring {
        p.add_point(lat, lon);
    }
    let (perimeter, area, _) = p.compute(true);
    (area, perimeter)
}
