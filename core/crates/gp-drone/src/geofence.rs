//! Geofence generation (add-drone-suite, drone/mission-patterns, "Geofence
//! generation"): a geodesic fence a set distance around a point, line, or
//! polygon (by the geometry buffers), an optional inner warning fence, the
//! fence's area and perimeter, and every mission waypoint outside it.

use geographiclib_rs::{Geodesic, InverseGeodesic};
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
    stability: gp_base::tool::Stability::Stable,
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
    warnings: &["WAYPOINT_OUTSIDE_GEOFENCE"],
    model: "The fence is the round geodesic buffer of the area (geometry.buffer.geodesic): every point within the distance of it. A waypoint is outside when its geodesic distance to the area exceeds the fence distance, and by exactly the difference (Karney 2013)",
    accuracy: "The fence is within 0.1% of the distance or 0.5 m, as measured; waypoint distances are exact geodesic distances to the area",
    when_to_use: "Use this to draw a keep-in boundary a set distance around a job site, a route, or a single point, with an optional inner warning fence. It gives the fence outline, its area and perimeter, and checks your mission waypoints, listing any that fall past a fence and by how far. The outline can go into your flight app's fence settings, if it has them, or onto a site briefing map.",
    limitations: "The fence is drawn in plan only; it sets no height limit and does not check airspace, terrain, or obstacles. Waypoints are tested as points, so a straight leg between two inside points can still cut across a concave edge of the fence. GPS error and wind drift are not included, so leave room for them in the distance you choose. Whether the drone obeys a fence depends on its own software.",
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
        Related {
            id: "drone.mission.export",
            reason: "next",
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

pub static GEOFENCE_CHECK: ToolDef = ToolDef {
    id: "drone.mission.geofence-check",
    title: "Check waypoints against a geofence",
    summary: "Which waypoints of a mission fall outside a geofence drawn a set distance around an area, and by how far, with any that cross an inner warning fence.",
    aliases: &[
        "geofence check",
        "waypoints outside geofence",
        "mission boundary check",
    ],
    keywords: &[
        "geofence",
        "waypoints",
        "outside",
        "boundary",
        "keep-in",
        "warning fence",
        "preflight check",
    ],
    inputs: &[
        GEOFENCE.inputs[0],
        GEOFENCE.inputs[1],
        GEOFENCE.inputs[3].required(),
        GEOFENCE.inputs[2],
        GEOFENCE.inputs[4],
    ],
    outputs: &[
        GEOFENCE.outputs[2],
        Field::new(
            "waypoints_checked",
            "Waypoints checked",
            "How many were tested",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        GEOFENCE.outputs[3],
        GEOFENCE.outputs[4],
        GEOFENCE.outputs[5],
    ],
    errors: GEOFENCE.errors,
    warnings: GEOFENCE.warnings,
    model: GEOFENCE.model,
    accuracy: GEOFENCE.accuracy,
    when_to_use: "Use this before uploading a mission to confirm every waypoint is inside the fence you intend to set. Enter the area the fence is drawn around, the fence distance, and the mission's waypoints. The tool lists each waypoint that falls outside and how far beyond the fence it is. Add a warning distance to also list the ones past an inner warning fence.",
    limitations: "Only the waypoints are tested, not the path between them, so a leg that cuts a corner of the fence between two inside waypoints is not caught; add waypoints along long legs near the boundary. The fence here is the geometric one this tool draws, which your aircraft's own geofence may round or simplify differently. Position error in flight is not allowed for, so leave a margin.",
    references: GEOFENCE.references,
    examples: &[Example {
        id: "primary",
        title: "Four waypoints against a 40 m fence around a field",
        input: r#"{"area":[{"lat":41,"lon":-96},{"lat":41,"lon":-95.997},{"lat":41.002,"lon":-95.997},{"lat":41.002,"lon":-96}],"distance":"40 m","waypoints":[{"lat":41.001,"lon":-95.9985},{"lat":41.0022,"lon":-95.9985},{"lat":41.0026,"lon":-95.9985},{"lat":41.001,"lon":-96.0008}]}"#,
        source: "add-drone-suite geofence requirement: waypoints flagged with their distance beyond the fence",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.mission.geofence",
            reason: "parent",
        },
        Related {
            id: "drone.mission.survey-grid",
            reason: "parent",
        },
        Related {
            id: "drone.mission.export",
            reason: "next",
        },
    ],
    sentence: "{outside_count} of {waypoints_checked} waypoints are outside the geofence.",
    limits: GEOFENCE.limits,
    run: run_geofence_check,
    ..ToolDef::BLANK
};

fn run_geofence_check(ctx: &mut Ctx) -> Result<Json, ToolError> {
    geofence(ctx, true)
}

fn run_geofence(ctx: &mut Ctx) -> Result<Json, ToolError> {
    geofence(ctx, false)
}

/// Builds the fence and tests the waypoints. `check_only` needs waypoints
/// and reports the test without the fence's outline and size.
fn geofence(ctx: &mut Ctx, check_only: bool) -> Result<Json, ToolError> {
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
    if check_only && wps.is_empty() {
        return Err(ToolError::invalid(
            "/waypoints",
            "Give the waypoints to check against the fence.",
        ));
    }
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
    if check_only {
        let mut out = vec![
            ("outside_count", Json::Num(outside.len() as f64)),
            ("waypoints_checked", Json::Num(wps.len() as f64)),
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
        return Ok(Json::obj(out));
    }
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
/// A course change at a waypoint counts as a turn past this many degrees.
const TURN_THRESHOLD_DEG: f64 = 5.0;

pub static PATTERN_STATS: ToolDef = ToolDef {
    id: "drone.mission.pattern-stats",
    title: "Flight path statistics",
    summary: "The length of a waypoint path, its legs and turns, how far it goes from the first waypoint, and the flight time at a groundspeed.",
    aliases: &[
        "flight path length calculator",
        "waypoint mission distance",
        "mission statistics",
    ],
    keywords: &[
        "path length",
        "waypoints",
        "legs",
        "turns",
        "flight time",
        "farthest point",
        "mission summary",
    ],
    inputs: &[
        Field::new(
            "waypoints",
            "Waypoints",
            "The path in order, one waypoint per line, like 40.4406, -80.002",
            Kind::List {
                items: VERTEX,
                min: 2,
                max: 5_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "groundspeed",
            "Groundspeed",
            "For the flight time, like 10 m/s",
            Kind::Quantity {
                q: QT::Speed,
                unit: "m/s",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "path_length",
            "Path length",
            "The sum of the legs",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "legs",
            "Legs",
            "Waypoints less one",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "turns",
            "Turns",
            "Waypoints where the course changes by more than 5°",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "longest_leg",
            "Longest leg",
            "Between two consecutive waypoints",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "farthest_from_start",
            "Farthest from the first waypoint",
            "Straight-line ground distance to the most distant waypoint",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "total_turning",
            "Total turning",
            "The sum of the course changes at the waypoints",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "flight_time",
            "Flight time",
            "Path length ÷ groundspeed, without turns or climbs",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Each leg is the geodesic between consecutive waypoints on WGS 84 (Karney 2013); the course change at a waypoint is the departure azimuth of the next leg less the arrival azimuth of the last, and counts as a turn past 5°",
    accuracy: "Leg lengths to nanometers on the ellipsoid; the flight time is a planning figure at one steady groundspeed",
    when_to_use: "Use this to size up a mission you already have as a list of waypoints, from a flight app, a spreadsheet, or one of the pattern tools here. Enter the waypoints in the order flown. The tool returns the total path length, the number of legs and turns, the longest leg, and how far the path gets from its first waypoint, which matters for visual line of sight. Add a groundspeed for the flight time.",
    limitations: "The path is straight legs over the ground between waypoints, so climbs, descents, curved turns, and hovering to take a photo are not counted, and the flight time comes out short for a pattern with many turns. The first waypoint stands in for where the pilot is, so add the takeoff point first if it is somewhere else. It does not check terrain, airspace, or a geofence.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A four-leg box flown at 8 m/s",
        input: r#"{"waypoints":[{"lat":44.98,"lon":-93.27},{"lat":44.9818,"lon":-93.27},{"lat":44.9818,"lon":-93.2662},{"lat":44.98,"lon":-93.2662},{"lat":44.98,"lon":-93.27}],"groundspeed":"8 m/s"}"#,
        source: "Geodesic leg lengths from GeographicLib's GeodSolve",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.mission.survey-grid",
            reason: "parent",
        },
        Related {
            id: "drone.mission.sorties",
            reason: "next",
        },
        Related {
            id: "drone.mission.geofence-check",
            reason: "next",
        },
    ],
    sentence: "The path is {path_length} long in {legs} legs, with {turns} turns.",
    limits: &[("batchRows", 100)],
    run: run_pattern_stats,
    ..ToolDef::BLANK
};

fn run_pattern_stats(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let wps = read_points(ctx, "waypoints")?;
    if let Some(i) = (1..wps.len()).find(|&i| wps[i] == wps[i - 1]) {
        return Err(ToolError::invalid(
            &format!("/waypoints/{i}"),
            "This waypoint repeats the one before it: a leg needs two different points.",
        ));
    }
    let g = Geodesic::wgs84();
    let (mut total, mut longest, mut farthest) = (0.0f64, 0.0f64, 0.0f64);
    let (mut turns, mut turning) = (0usize, 0.0f64);
    let mut arriving: Option<f64> = None;
    for pair in wps.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (s, az1, az2, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
        total += s;
        longest = longest.max(s);
        if let Some(prev) = arriving {
            // The change of course at the waypoint, the short way round.
            let change = ((az1 - prev + 540.0).rem_euclid(360.0) - 180.0).abs();
            turning += change;
            if change > TURN_THRESHOLD_DEG {
                turns += 1;
            }
        }
        arriving = Some(az2);
        let from_start: f64 = g.inverse(wps[0].0, wps[0].1, b.0, b.1);
        farthest = farthest.max(from_start);
    }
    let m = unit(QT::Length, "m");
    let len = |v: f64| Q { value: v, unit: m };
    let mut out = vec![
        (
            "path_length",
            ctx.out(
                "path_length",
                Q {
                    value: total,
                    unit: unit(QT::Distance, "m"),
                },
            ),
        ),
        ("legs", Json::Num((wps.len() - 1) as f64)),
        ("turns", Json::Num(turns as f64)),
        ("longest_leg", ctx.out("longest_leg", len(longest))),
        (
            "farthest_from_start",
            ctx.out("farthest_from_start", len(farthest)),
        ),
        (
            "total_turning",
            ctx.out(
                "total_turning",
                Q {
                    value: turning,
                    unit: unit(QT::Angle, "deg"),
                },
            ),
        ),
    ];
    if let Some(v) = ctx.quantity("groundspeed")? {
        let v = v.to(unit(QT::Speed, "m/s"));
        if v <= 0.0 {
            return Err(ToolError::invalid(
                "/groundspeed",
                "The groundspeed must be positive.",
            ));
        }
        out.push((
            "flight_time",
            ctx.out(
                "flight_time",
                Q {
                    value: total / v,
                    unit: unit(QT::Time, "s"),
                },
            ),
        ));
    }
    Ok(Json::obj(out))
}

fn ring_area(g: &Geodesic, ring: &[(f64, f64)]) -> (f64, f64) {
    let mut p = geographiclib_rs::PolygonArea::new(g, geographiclib_rs::Winding::CounterClockwise);
    for &(lat, lon) in ring {
        p.add_point(lat, lon);
    }
    let (perimeter, area, _) = p.compute(true);
    (area, perimeter)
}
