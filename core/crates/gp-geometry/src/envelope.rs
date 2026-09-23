//! Envelopes (add-navigation-and-geometry, geometry/computational,
//! "Envelopes"): the bounding box of points, a line, or a polygon, aware of
//! the antimeridian (west greater than east when the box crosses it) and of
//! geodesic edges that bow poleward between their corners.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use libm::cos;

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (direct and inverse problems; the vertex of a geodesic, where its azimuth is 90°)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};
const RFC7946: Reference = Reference {
    title: "The GeoJSON Format",
    issuer: "IETF RFC 7946",
    year: 2016,
    edition: "RFC 7946",
    locator: "Section 5.2 (the antimeridian: a bounding box with west greater than east)",
    url: "https://www.rfc-editor.org/rfc/rfc7946",
};

const VERTEX: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.4406",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -80.002",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "ring",
        "Ring",
        "For a polygon: 0 for the outline, 1, 2, … for holes",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];

const fn deg_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7))
}

pub static BBOX: ToolDef = ToolDef {
    id: "geometry.shape.bbox",
    stability: gp_base::tool::Stability::Stable,
    title: "Bounding box, antimeridian-aware",
    summary: "The smallest latitude and longitude box around points, a line, or a polygon, written west-south-east-north, crossing the antimeridian when that is shorter and reaching a pole a polygon circles.",
    aliases: &[
        "bounding box",
        "bbox",
        "extent",
        "envelope",
        "min max lat lon",
    ],
    keywords: &[
        "bbox",
        "bounding box",
        "extent",
        "envelope",
        "antimeridian",
        "dateline",
        "west",
        "east",
    ],
    inputs: &[
        Field::new(
            "points",
            "Points",
            "One per line, like 40.4406, -80.002",
            Kind::List {
                items: VERTEX,
                min: 1,
                max: 20_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "shape",
            "Treat as",
            "points (the default), line, or polygon; a line or polygon adds its geodesic edges",
            Kind::Choice(&["points", "line", "polygon"]),
        )
        .core(),
    ],
    outputs: &[
        deg_out("west", "West", "Longitude of the west edge"),
        deg_out("south", "South", "Latitude of the south edge"),
        deg_out(
            "east",
            "East",
            "Longitude of the east edge; less than west when the box crosses the antimeridian",
        ),
        deg_out("north", "North", "Latitude of the north edge"),
        Field::new(
            "lon_span",
            "Longitude span",
            "East of west, going east",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7)),
        Field::new(
            "crosses_antimeridian",
            "Crosses the antimeridian",
            "yes or no",
            Kind::Text { max_len: 3 },
        ),
        Field::new(
            "pole",
            "Pole enclosed",
            "none, north, or south",
            Kind::Text { max_len: 5 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["CROSSES_ANTIMERIDIAN", "POLE_ENCLOSED"],
    model: "Longitudes: every point, and for a line or polygon each geodesic edge's shorter arc of longitude, marked on the circle; the box is the complement of the largest gap (RFC 7946 §5.2 writes a crossing box with west > east). Latitudes: the points, plus each edge's vertex where its azimuth passes 90° or 270°, found by bisection on the geodesic direct problem (Karney 2013). A polygon whose outline winds around a pole spans every longitude and reaches that pole",
    accuracy: "Edge extremes to 1e-9° or better; the box is the smallest in longitude span",
    when_to_use: "Use this to get the extent of something — the area a set of points covers, the window a map should open at, the bounds to index a shape by, the box to hand to a tile or data service. Two things make it more than taking the minimum and maximum of the coordinates, and both matter: a line or polygon is bounded by its geodesic edges rather than its corners, and those bow poleward, sometimes by degrees; and a shape either side of the antimeridian has to produce the short box across it rather than one wrapping the whole world.",
    limitations: "The box is in latitude and longitude, so it is a region on the graticule and not a rectangle on the ground: it is widest in kilometres at its equatorward edge and its corners are not equidistant from anything. Treating a set of points as points rather than as a line or polygon is a different question and gives a different answer — the edges are only bounded when you say there are edges. Where a shape spreads so widely that no gap in longitude is clearly the largest, two correct implementations of the rule can return different boxes of the same width, so the tie is not something to depend on. A polygon whose outline winds around a pole spans every longitude and reaches that pole, which is right but is a very large box.",
    references: &[KARNEY, RFC7946],
    examples: &[Example {
        id: "primary",
        title: "Two points either side of the antimeridian",
        input: r#"{"points":[{"lat":-17.0,"lon":170.0},{"lat":-15.0,"lon":-170.0}]}"#,
        source: "RFC 7946 §5.2 writes a box crossing the antimeridian with west > east, so 170 to -170 is the 20° box and not the 340° one. Checked over ten shapes against Karney's geographiclib bisected for where each edge's azimuth passes due east or west, which agrees with the tool exactly in every coordinate",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "bbox",
        map: &[
            ("south", "south"),
            ("west", "west"),
            ("north", "north"),
            ("east", "east"),
        ],
    }],
    related: &[
        Related {
            id: "geometry.shape.centroid",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
        Related {
            id: "geometry.shape.enclosing",
            reason: "alternative",
        },
    ],
    sentence: "The box runs from {west} to {east} in longitude, a {lon_span} span, and from {south} to {north} in latitude.",
    limits: &[("batchRows", 1_000)],
    run: run_bbox,
    ..ToolDef::BLANK
};

fn norm180(x: f64) -> f64 {
    let v = (x + 180.0).rem_euclid(360.0) - 180.0;
    if v == -180.0 && x > 0.0 { 180.0 } else { v }
}

/// The latitude where edge a→b turns (its azimuth passes 90° or 270°), if inside the edge.
fn edge_vertex(g: &Geodesic, a: (f64, f64), b: (f64, f64)) -> Option<f64> {
    let (s12, azi1, azi2, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
    let (c1, c2) = (cos(azi1.to_radians()), cos(azi2.to_radians()));
    if s12 <= 0.0 || c1 * c2 >= 0.0 {
        return None;
    }
    // Northbound turning south (or the reverse): bisect on the sign of cos(azimuth).
    let (mut lo, mut hi) = (0.0, s12);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        let (_, _, az): (f64, f64, f64) = g.direct(a.0, a.1, azi1, mid);
        if cos(az.to_radians()) * c1 > 0.0 {
            lo = mid
        } else {
            hi = mid
        }
    }
    let (lat, _, _): (f64, f64, f64) = g.direct(a.0, a.1, azi1, 0.5 * (lo + hi));
    Some(lat)
}

fn run_bbox(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows("points")?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("points", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("points", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/points/{i}/lat")));
        }
        let ring = r
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/points/{i}/ring"),
                "Ring must be a whole number from 0 to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        rings[k].push((lat, norm180(lon)));
    }
    rings.retain(|r| !r.is_empty());
    let shape = ctx.choice("shape")?.unwrap_or("points");
    let g = Geodesic::wgs84();
    let (mut south, mut north) = (f64::INFINITY, f64::NEG_INFINITY);
    // Arcs of longitude covered: (start, eastward length).
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    for ring in &rings {
        for &(lat, lon) in ring {
            south = south.min(lat);
            north = north.max(lat);
            arcs.push((lon, 0.0));
        }
        let n = ring.len();
        let edges = match shape {
            "line" => n.saturating_sub(1),
            "polygon" if n >= 3 => n,
            _ => 0,
        };
        for i in 0..edges {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if let Some(lat) = edge_vertex(&g, a, b) {
                south = south.min(lat);
                north = north.max(lat);
            }
            let d = norm180(b.1 - a.1);
            arcs.push(if d >= 0.0 { (a.1, d) } else { (b.1, -d) });
        }
    }
    // A polygon outline that winds once around a pole covers every longitude up to it.
    let mut pole = "none";
    if shape == "polygon" && rings[0].len() >= 3 {
        let r = &rings[0];
        let turn: f64 = (0..r.len())
            .map(|i| norm180(r[(i + 1) % r.len()].1 - r[i].1))
            .sum();
        if turn.abs() > 180.0 {
            pole = if r.iter().map(|p| p.0).sum::<f64>() >= 0.0 {
                "north"
            } else {
                "south"
            };
        }
    }
    let (west, east, span) = if pole != "none" {
        if pole == "north" {
            north = 90.0
        } else {
            south = -90.0
        }
        (-180.0, 180.0, 360.0)
    } else {
        // The largest gap between covered arcs, going east around the circle.
        arcs.sort_by(|x, y| x.0.total_cmp(&y.0));
        let (mut best_gap, mut best_start) = (-1.0, 0.0);
        let mut reach = arcs[0].0 + arcs[0].1;
        for &(s, l) in &arcs[1..] {
            if s > reach && s - reach > best_gap {
                best_gap = s - reach;
                best_start = s;
            }
            reach = reach.max(s + l);
        }
        // The wrap-around gap, from the farthest reach back to the first start.
        let wrap = arcs[0].0 + 360.0 - reach;
        if wrap > best_gap {
            best_gap = wrap;
            best_start = arcs[0].0;
        }
        if best_gap <= 0.0 {
            (-180.0, 180.0, 360.0)
        } else {
            let span = 360.0 - best_gap;
            (best_start, norm180(best_start + span), span)
        }
    };
    let crosses = pole == "none" && span < 360.0 && east < west;
    if crosses {
        ctx.warnings.push(Warning::new(
            "CROSSES_ANTIMERIDIAN",
            format!("The box crosses the antimeridian, so west ({west}°) is greater than east ({east}°), as RFC 7946 writes it."),
        ));
    }
    if pole != "none" {
        ctx.warnings.push(Warning::new("POLE_ENCLOSED", format!("The outline circles the {pole} pole, so the box spans every longitude and reaches it.")));
    }
    let dq = |v: f64| Q {
        value: v,
        unit: deg,
    };
    Ok(Json::obj([
        ("west", ctx.out("west", dq(west))),
        ("south", ctx.out("south", dq(south))),
        ("east", ctx.out("east", dq(east))),
        ("north", ctx.out("north", dq(north))),
        ("lon_span", ctx.out("lon_span", dq(span))),
        (
            "crosses_antimeridian",
            Json::str(if crosses { "yes" } else { "no" }),
        ),
        ("pole", Json::str(pole)),
    ]))
}

// ---------------------------------------------------------------- hull, rectangle, circle

use gp_geo::buffer::{Aeqd, center};
use libm::{atan2, hypot, sin};

type P = (f64, f64);

fn cross3(o: P, a: P, b: P) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Convex hull indices, counterclockwise (Andrew's monotone chain).
fn hull(pts: &[P]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    idx.sort_by(|&a, &b| {
        pts[a]
            .0
            .total_cmp(&pts[b].0)
            .then(pts[a].1.total_cmp(&pts[b].1))
    });
    idx.dedup_by(|a, b| pts[*a] == pts[*b]);
    if idx.len() < 3 {
        return idx;
    }
    let mut h: Vec<usize> = Vec::with_capacity(2 * idx.len());
    for pass in 0..2 {
        let start = h.len();
        let seq: Vec<usize> = if pass == 0 {
            idx.clone()
        } else {
            idx.iter().rev().copied().collect()
        };
        for &i in &seq {
            while h.len() >= start + 2
                && cross3(pts[h[h.len() - 2]], pts[h[h.len() - 1]], pts[i]) <= 0.0
            {
                h.pop();
            }
            h.push(i);
        }
        h.pop();
    }
    h
}

/// The smallest circle holding every point (Welzl's algorithm, iterative,
/// over a fixed shuffle so the answer is the same every run).
fn min_circle(pts: &[P]) -> (P, f64) {
    let mut p: Vec<P> = pts.to_vec();
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..p.len()).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        p.swap(i, (s % (i as u64 + 1)) as usize);
    }
    let circle2 = |a: P, b: P| {
        (
            ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
            hypot(a.0 - b.0, a.1 - b.1) / 2.0,
        )
    };
    let circle3 = |a: P, b: P, c: P| {
        let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
        if d.abs() < 1e-300 {
            return None;
        }
        let (a2, b2, c2) = (
            a.0 * a.0 + a.1 * a.1,
            b.0 * b.0 + b.1 * b.1,
            c.0 * c.0 + c.1 * c.1,
        );
        let ux = (a2 * (b.1 - c.1) + b2 * (c.1 - a.1) + c2 * (a.1 - b.1)) / d;
        let uy = (a2 * (c.0 - b.0) + b2 * (a.0 - c.0) + c2 * (b.0 - a.0)) / d;
        Some(((ux, uy), hypot(a.0 - ux, a.1 - uy)))
    };
    let inside = |c: (P, f64), q: P| hypot(q.0 - c.0.0, q.1 - c.0.1) <= c.1 * (1.0 + 1e-12) + 1e-9;
    let mut c = (p[0], 0.0);
    for i in 1..p.len() {
        if inside(c, p[i]) {
            continue;
        }
        c = (p[i], 0.0);
        for j in 0..i {
            if inside(c, p[j]) {
                continue;
            }
            c = circle2(p[i], p[j]);
            for k in 0..j {
                if !inside(c, p[k]) {
                    c = circle3(p[i], p[j], p[k]).unwrap_or(c);
                }
            }
        }
    }
    c
}

const OUTLINE_ROW: &[Field] = &[
    deg_out("lat", "Latitude", "Degrees"),
    deg_out("lon", "Longitude", "Degrees"),
    Field::new(
        "shape",
        "Shape",
        "hull, rectangle, or circle",
        Kind::Text { max_len: 10 },
    ),
    Field::new(
        "part",
        "Part",
        "0 hull, 1 rectangle, 2 circle",
        Kind::Number { min: 0.0, max: 2.0 },
    )
    .precision(Precision::Decimals(0)),
];

pub static ENCLOSING: ToolDef = ToolDef {
    id: "geometry.shape.enclosing",
    stability: gp_base::tool::Stability::Stable,
    title: "Hull, bounding rectangle, and enclosing circle",
    summary: "Around a set of points: the convex hull, the smallest rotated rectangle, and the smallest circle that holds them all, with its center and geodesic radius.",
    aliases: &[
        "convex hull",
        "minimum enclosing circle",
        "smallest enclosing circle",
        "minimum bounding rectangle",
        "oriented bounding box",
    ],
    keywords: &[
        "hull",
        "convex hull",
        "enclosing circle",
        "bounding rectangle",
        "MBR",
        "oriented",
        "smallest circle",
        "points",
    ],
    inputs: &[Field::new(
        "points",
        "Points",
        "One per line, like 40.4406, -80.002",
        Kind::List {
            items: VERTEX,
            min: 1,
            max: 5_000,
        },
    )
    .required()
    .core()],
    outputs: &[
        deg_out(
            "circle_lat",
            "Circle center latitude",
            "Of the smallest enclosing circle",
        ),
        deg_out(
            "circle_lon",
            "Circle center longitude",
            "Of the smallest enclosing circle",
        ),
        Field::new(
            "circle_radius",
            "Circle radius",
            "Geodesic, to the farthest point",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "hull_count",
            "Hull corners",
            "Points on the convex hull",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "hull_area",
            "Hull area",
            "Geodesic",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(8)),
        Field::new(
            "rect_length",
            "Rectangle length",
            "The long side",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "rect_width",
            "Rectangle width",
            "The short side",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "rect_azimuth",
            "Rectangle direction",
            "Of the long side, from north, 0° to 180°",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "outlines",
            "Outlines",
            "Hull corners, rectangle corners, and the circle",
            Kind::List {
                items: OUTLINE_ROW,
                min: 0,
                max: 100_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[],
    model: "Circle: Welzl's smallest circle on an azimuthal equidistant plane, re-centered on its result until the circle's center is the plane's own; that plane keeps distances and directions from its center, so the fixed point is the smallest geodesic circle (Karney 2013). Hull: great-circle hull of the points on the sphere, by monotone chain in a gnomonic projection from their mean. Rectangle: the smallest-area rectangle on an edge of the hull, on the equidistant plane at the points' center",
    accuracy: "The circle's radius is an exact geodesic distance, the center converged to 1 mm; the rectangle is planar on the equidistant map, true for spans of tens of kilometers to about 1 part in 10⁶",
    when_to_use: "Use this to put a shape around a set of positions: the coverage circle for a set of sightings, the smallest area holding a survey's control points, the footprint of a swarm or a fleet, the block a site occupies. Three answers come back because they suit different jobs — a circle is what a range or a broadcast covers, the convex hull is the tightest area that holds everything, and the smallest rotated rectangle is how a field, a runway or a site plan is usually described.",
    limitations: "All three are computed on one plane placed at the points, so they are meant for spreads of tens of kilometres rather than continental ones; the circle's radius is then an exact geodesic distance while the rectangle stays planar. The convex hull holds every point and says nothing about how they are distributed inside it: one outlier stretches all three answers, and none is a summary of where the points mostly are. Collinear or nearly collinear points give a hull and a rectangle that degenerate to a line, with zero area and zero width, which is correct. Where the hull is a triangle the smallest rectangle is not unique — all three edge-flush rectangles have exactly the same area — so the sides returned are one valid choice among equals.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Seven survey points",
        input: r#"{"points":[{"lat":40.0,"lon":-105.0},{"lat":40.004,"lon":-104.996},{"lat":40.001,"lon":-104.99},{"lat":39.997,"lon":-104.993},{"lat":40.002,"lon":-104.994},{"lat":39.999,"lon":-104.998},{"lat":40.006,"lon":-104.992}]}"#,
        source: "checked against GEOS 3.11.4 through shapely — its own minimum_bounding_circle, convex_hull and minimum_rotated_rectangle — over twelve point sets, agreeing on every hull corner count, on the circle radius to 2.5e-8 relative, on the hull area to 5.7e-7, and placing the circle's centre within 0.8 m over a 20 km scatter",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("rings", "outlines")],
        },
        Layer {
            kind: "point",
            map: &[("lat", "circle_lat"), ("lon", "circle_lon")],
        },
    ],
    related: &[
        Related {
            id: "geometry.shape.bbox",
            reason: "alternative",
        },
        Related {
            id: "geometry.shape.centroid",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
    ],
    sentence: "The smallest circle around the points has a radius of {circle_radius}. The smallest rectangle is {rect_length} by {rect_width}.",
    limits: &[("batchRows", 1_000)],
    run: run_enclosing,
    ..ToolDef::BLANK
};

fn run_enclosing(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows("points")?;
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("points", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("points", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/points/{i}/lat")));
        }
        pts.push((lat, norm180(lon)));
    }
    let g = Geodesic::wgs84();
    let (lat0, lon0) = center(&pts);
    // Everything must sit well inside the hemisphere around the points' center.
    let unit = |(la, lo): (f64, f64)| {
        let (a, o) = (la.to_radians(), lo.to_radians());
        (cos(a) * cos(o), cos(a) * sin(o), sin(a))
    };
    let c3 = unit((lat0, lon0));
    if pts.iter().any(|&p| {
        let v = unit(p);
        v.0 * c3.0 + v.1 * c3.1 + v.2 * c3.2 < 0.2
    }) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The points spread over most of a hemisphere; split them into smaller groups.",
        )
        .at("/points"));
    }
    // The smallest circle: re-center the equidistant plane until the circle sits at its origin.
    let (mut clat, mut clon) = (lat0, lon0);
    for _ in 0..30 {
        let map = Aeqd {
            g: &g,
            lat0: clat,
            lon0: clon,
        };
        let plane: Vec<P> = pts.iter().map(|&p| map.fwd(p)).collect();
        let (c, _) = min_circle(&plane);
        if hypot(c.0, c.1) < 1e-4 {
            break;
        }
        (clat, clon) = map.rev(c);
    }
    let radius = pts
        .iter()
        .map(|p| {
            let d: f64 = g.inverse(clat, clon, p.0, p.1);
            d
        })
        .fold(0.0, f64::max);
    // The great-circle hull, by a gnomonic projection from the points' center.
    let (sl, cl) = (sin(lon0.to_radians()), cos(lon0.to_radians()));
    let e = (-sl, cl, 0.0);
    let n = (
        c3.1 * e.2 - c3.2 * e.1,
        c3.2 * e.0 - c3.0 * e.2,
        c3.0 * e.1 - c3.1 * e.0,
    );
    let gno: Vec<P> = pts
        .iter()
        .map(|&p| {
            let v = unit(p);
            let k = v.0 * c3.0 + v.1 * c3.1 + v.2 * c3.2;
            (
                (v.0 * e.0 + v.1 * e.1 + v.2 * e.2) / k,
                (v.0 * n.0 + v.1 * n.1 + v.2 * n.2) / k,
            )
        })
        .collect();
    let h = hull(&gno);
    let hull_ll: Vec<(f64, f64)> = h.iter().map(|&i| pts[i]).collect();
    let hull_area = if hull_ll.len() >= 3 {
        super::ring_area(&g, &hull_ll).0.abs()
    } else {
        0.0
    };
    // The smallest rectangle, on the equidistant plane at the points' center.
    let map = Aeqd { g: &g, lat0, lon0 };
    let plane: Vec<P> = pts.iter().map(|&p| map.fwd(p)).collect();
    let ph = hull(&plane);
    let hp: Vec<P> = ph.iter().map(|&i| plane[i]).collect();
    let mut best: Option<(f64, P, f64, f64, f64, f64)> = None; // area, u, lo_u, hi_u, lo_v, hi_v
    let dirs: Vec<P> = if hp.len() >= 2 {
        (0..hp.len())
            .map(|i| {
                let (a, b) = (hp[i], hp[(i + 1) % hp.len()]);
                let l = hypot(b.0 - a.0, b.1 - a.1);
                ((b.0 - a.0) / l, (b.1 - a.1) / l)
            })
            .filter(|u| u.0.is_finite())
            .collect()
    } else {
        vec![(0.0, 1.0)]
    };
    for u in dirs {
        let v = (-u.1, u.0);
        let (mut lu, mut hu, mut lv, mut hv) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in if hp.is_empty() { &plane } else { &hp } {
            let (a, b) = (p.0 * u.0 + p.1 * u.1, p.0 * v.0 + p.1 * v.1);
            lu = lu.min(a);
            hu = hu.max(a);
            lv = lv.min(b);
            hv = hv.max(b);
        }
        let area = (hu - lu) * (hv - lv);
        if best.is_none_or(|b| area < b.0 - 1e-9 * area.abs()) {
            best = Some((area, u, lu, hu, lv, hv));
        }
    }
    let (_, u, lu, hu, lv, hv) = best.expect("at least one direction");
    let v = (-u.1, u.0);
    let (len_u, len_v) = (hu - lu, hv - lv);
    let corner = |a: f64, b: f64| map.rev((a * u.0 + b * v.0, a * u.1 + b * v.1));
    let rect = [
        corner(lu, lv),
        corner(hu, lv),
        corner(hu, hv),
        corner(lu, hv),
    ];
    let long = if len_u >= len_v { u } else { v };
    let azimuth = atan2(long.0, long.1).to_degrees().rem_euclid(180.0);
    let dq = |v: f64| Q {
        value: v,
        unit: deg,
    };
    let mut outlines = Vec::new();
    let mut row = |ll: (f64, f64), shape: &str, part: f64| {
        outlines.push(Json::obj([
            ("lat", dq(ll.0).to_json()),
            ("lon", dq(ll.1).to_json()),
            ("shape", Json::str(shape)),
            ("part", Json::Num(part)),
        ]));
    };
    for &p in &hull_ll {
        row(p, "hull", 0.0);
    }
    for &p in &rect {
        row(p, "rectangle", 1.0);
    }
    if radius > 0.0 {
        for k in 0..72 {
            let (la, lo, _): (f64, f64, f64) = g.direct(clat, clon, 5.0 * k as f64, radius);
            row((la, norm180(lo)), "circle", 2.0);
        }
    }
    let m = units::by_symbol(QT::Length, "m").expect("m");
    Ok(Json::obj([
        ("circle_lat", ctx.out("circle_lat", dq(clat))),
        ("circle_lon", ctx.out("circle_lon", dq(norm180(clon)))),
        (
            "circle_radius",
            ctx.out(
                "circle_radius",
                Q {
                    value: radius,
                    unit: m,
                },
            ),
        ),
        ("hull_count", Json::Num(hull_ll.len() as f64)),
        (
            "hull_area",
            ctx.out("hull_area", super::q(hull_area, "m2", QT::Area)),
        ),
        (
            "rect_length",
            ctx.out(
                "rect_length",
                Q {
                    value: len_u.max(len_v),
                    unit: m,
                },
            ),
        ),
        (
            "rect_width",
            ctx.out(
                "rect_width",
                Q {
                    value: len_u.min(len_v),
                    unit: m,
                },
            ),
        ),
        ("rect_azimuth", ctx.out("rect_azimuth", dq(azimuth))),
        ("outlines", Json::Arr(outlines)),
    ]))
}
