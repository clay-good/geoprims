//! The geodesic buffer tool (add-navigation-and-geometry, geometry/computational,
//! "Geodesic buffers"); the algorithm lives in `gp_geo::buffer`, shared with
//! the drone geofence.

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_geo::buffer::{Cap, Join, Shape, Style, geodesic};

use gp_base::error::Warning;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (direct and inverse problems; azimuthal equidistant projection, section 8)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
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
const OUT_VERTEX: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "part",
        "Part",
        "0, 1, … when the buffer is in pieces",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "ring",
        "Ring",
        "0 for a part's outline, 1, 2, … for its holes",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
];

pub static BUFFER: ToolDef = ToolDef {
    id: "geometry.buffer.geodesic",
    stability: gp_base::tool::Stability::Stable,
    title: "Buffer a point, line, or polygon",
    summary: "The area within a distance of a point, line, or polygon on the ellipsoid, or a polygon shrunk inward, with round, mitre, or bevel corners, measured back against the input.",
    aliases: &["buffer", "geodesic buffer", "offset polygon", "setback", "zone around a line", "radius around a point"],
    keywords: &["buffer", "offset", "setback", "radius", "zone", "corridor", "geofence", "inset", "shrink", "grow"],
    inputs: &[
        Field::new("vertices", "Vertices", "A point, a line in order, or polygon corners (holes by ring), one per line, like 40.4406, -80.002", Kind::List { items: VERTEX, min: 1, max: 5_000 }).required().core(),
        Field::new("distance", "Distance", "How far out, like 500 m; negative shrinks a polygon, like -20 m", Kind::Quantity { q: QT::Length, unit: "m" }).required().core(),
        Field::new("shape", "Treat as", "point, line, or polygon; default by count: 1 point, 2 a line, 3 or more a polygon", Kind::Choice(&["polygon", "line", "point"])).core(),
        Field::new("join", "Corners", "round (default), mitre (sharp), or bevel (cut off)", Kind::Choice(&["round", "mitre", "bevel"])),
        Field::new("cap", "Line ends", "round (default), flat, or square", Kind::Choice(&["round", "flat", "square"])),
        Field::new("mitre_limit", "Mitre limit", "Longest sharp corner, in distances, like 5 (the default); longer ones are beveled", Kind::Number { min: 1.0, max: 100.0 }),
    ],
    outputs: &[
        Field::new("area", "Area", "Parts minus holes", Kind::Quantity { q: QT::Area, unit: "km2" }).precision(Precision::Significant(8)),
        Field::new("perimeter", "Perimeter", "Every outline and hole", Kind::Quantity { q: QT::Distance, unit: "km" }).precision(Precision::Decimals(3)),
        Field::new("parts", "Parts", "Separate pieces; 0 when the buffer collapsed", Kind::Number { min: 0.0, max: 1e6 }).precision(Precision::Decimals(0)),
        Field::new("vertex_count", "Vertices", "In the boundary", Kind::Number { min: 0.0, max: 1e9 }).precision(Precision::Decimals(0)),
        Field::new("max_deviation", "Largest measured error", "Geodesic distance from the input, minus the buffer distance, at every vertex and edge midpoint (sharp corners and flat ends skipped)", Kind::Quantity { q: QT::Length, unit: "m" }).precision(Precision::Decimals(3)),
        Field::new("tolerance", "Tolerance", "0.1% of the distance or 0.5 m, whichever is larger", Kind::Quantity { q: QT::Length, unit: "m" }).precision(Precision::Decimals(3)),
        Field::new("boundary", "Boundary", "Each part's outline counterclockwise, then its holes", Kind::List { items: OUT_VERTEX, min: 0, max: 1_000_000 }),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["BUFFER_COLLAPSED", "BUFFER_ACCURACY"],
    model: "On WGS 84: convex pieces (a rectangle per edge, a disk, wedge, or mitre per corner, a disk or square per line end) on an azimuthal equidistant plane at the input's center, unioned by keeping the edges no other piece covers; vertices on round and straight parts are then placed exactly at the distance from the nearest input point by the geodesic direct problem, and the distance is measured again at every vertex and edge midpoint (Karney 2013)",
    accuracy: "Round parts within 0.1% of the distance or 0.5 m, whichever is larger, as measured on every result; mitre and square corners sit at d / cos(θ/2) by construction. Input and buffer must fit within 1,000 km of their center",
    when_to_use: "Use this to draw the zone within a distance of something on the ground: a geofence around a field, a setback from a property line, a corridor either side of a route, a noise or buffer zone around a runway, an exclusion radius around a point. A negative distance shrinks a polygon instead, which is how a setback inside a parcel is drawn. The corner style matters for a legal or regulatory setback — round follows the true distance, mitre carries the corner out to the intersection the way a surveyor draws it — and the result reports the area, the perimeter, and how far any measured point strayed from the distance asked for.",
    limitations: "The buffer and its input must fit within 1,000 km of their common center, because the construction runs on an azimuthal equidistant plane placed there and the plane stops being faithful beyond that; a larger shape is refused rather than quietly distorted. Round parts are drawn as a polygon, so the boundary is a chord inside the true curve between vertices — the vertices themselves sit on the distance, and the reported deviation is the worst sag in between, not an error in placing them. Mitre and square corners sit at the distance over the cosine of half the corner angle, which is farther out than the distance asked for; that is what a mitre is, and the mitre limit bevels a corner too sharp for it. A shrink can collapse a polygon to nothing or split it into pieces, which is reported rather than treated as failure. Shrinking a round buffer back by its own distance is one such collapse and is not a mistake: a round corner is drawn as a polygon inscribed in its arc, so it is short of the distance by the reported deviation, and eroding by more than that empties it. Use mitre corners if a buffer needs to be undone exactly. Areas and perimeters are on the ellipsoid, but the union is computed in the plane, so a self-touching or repeated input should be made valid first.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 500 m geofence around a field",
        input: r#"{"vertices":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.006,"lon":-104.99},{"lat":40.006,"lon":-105.0}],"distance":"500 m"}"#,
        source: "GeographicLib 2.7 GeodSolve, against 8,004 points sampled along the input rectangle: all 157 boundary vertices lie 500 m from it within 61 nanometres, and the worst chord midpoint sags 0.124103 m inside, which is the max_deviation of 0.1241033677 m the tool reports",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "polygon", map: &[("rings", "boundary")] }],
    related: &[
        Related { id: "geometry.area.polygon", reason: "next" },
        Related { id: "navigation.geodesic.inverse", reason: "alternative" },
        Related { id: "geometry.shape.densify", reason: "alternative" },
    ],
    sentence: "{if parts > 0}The buffer covers {area} in {parts} {plural parts \"part\" \"parts\"}, every checked point within {max_deviation} of the distance.{/if}{if parts < 1}Nothing is left: the buffer collapsed.{/if}",
    limits: &[("batchRows", 20)],
    run: run_buffer,
    ..ToolDef::BLANK
};

fn run_buffer(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let rows = ctx.rows("vertices")?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("vertices", i, row, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("vertices", i, row, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/vertices/{i}/lat")));
        }
        let ring = row
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/vertices/{i}/ring"),
                "Ring must be a whole number from 0 to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        // Repeated vertices add nothing and would make zero-length edges.
        if rings[k].last() != Some(&(lat, lon)) {
            rings[k].push((lat, lon));
        }
    }
    rings.retain(|r| !r.is_empty());
    let count: usize = rings.iter().map(Vec::len).sum();
    let kind = match ctx.choice("shape")? {
        Some("point") => Shape::Point,
        Some("line") => Shape::Line,
        Some(_) => Shape::Polygon,
        None if count == 1 => Shape::Point,
        None if count == 2 => Shape::Line,
        None => Shape::Polygon,
    };
    match kind {
        Shape::Point if count != 1 => {
            return Err(ToolError::invalid("/vertices", "A point is one vertex."));
        }
        Shape::Line if rings.len() != 1 || count < 2 => {
            return Err(ToolError::invalid(
                "/vertices",
                "A line is two or more vertices, all in ring 0.",
            ));
        }
        Shape::Polygon => {
            for (k, ring) in rings.iter_mut().enumerate() {
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() < 3 {
                    return Err(ToolError::invalid(
                        "/vertices",
                        format!("Ring {k} needs at least 3 distinct corners."),
                    ));
                }
                gp_geo::point::refuse_repeated_corner(ring, "/vertices")?;
            }
        }
        _ => {}
    }
    let d = ctx.req_quantity("distance")?.to(m);
    if d == 0.0 || !d.is_finite() {
        return Err(ToolError::invalid(
            "/distance",
            "The distance must be a nonzero length, like 500 m.",
        ));
    }
    let join = match ctx.choice("join")? {
        Some("mitre") => Join::Mitre,
        Some("bevel") => Join::Bevel,
        _ => Join::Round,
    };
    let cap = match ctx.choice("cap")? {
        Some("flat") => Cap::Flat,
        Some("square") => Cap::Square,
        _ => Cap::Round,
    };
    if kind == Shape::Point && cap == Cap::Flat {
        return Err(ToolError::invalid(
            "/cap",
            "A point has no direction for a flat end: use round or square.",
        ));
    }
    let st = Style {
        join,
        cap,
        mitre_limit: ctx.number("mitre_limit")?.unwrap_or(5.0),
    };
    let g = Geodesic::wgs84();
    let out = geodesic(&g, kind, &rings, d, &st)?;
    if out.parts.is_empty() {
        ctx.warnings.push(Warning::new(
            "BUFFER_COLLAPSED",
            if kind == Shape::Polygon {
                "The polygon is narrower than twice the inward distance, so nothing is left."
            } else {
                "Only a polygon can shrink: a point or line buffered inward is empty."
            },
        ));
    } else if out.max_deviation > out.tolerance {
        ctx.warnings.push(Warning::new(
            "BUFFER_ACCURACY",
            format!(
                "The boundary strays {:.3} m from the distance, over the {:.3} m tolerance.",
                out.max_deviation, out.tolerance
            ),
        ));
    }
    let (mut area, mut perimeter) = (0.0, 0.0);
    let mut boundary = Vec::new();
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    for (pi, (outline, holes)) in out.parts.iter().enumerate() {
        for (ri, ring) in core::iter::once(outline).chain(holes.iter()).enumerate() {
            let (a, p) = super::ring_area(&g, ring);
            area += if ri == 0 { a.abs() } else { -a.abs() };
            perimeter += p;
            for &(lat, lon) in ring {
                boundary.push(Json::obj([
                    ("lat", dq(lat)),
                    ("lon", dq(lon)),
                    ("part", Json::Num(pi as f64)),
                    ("ring", Json::Num(ri as f64)),
                ]));
            }
        }
    }
    let n = boundary.len();
    Ok(Json::obj([
        ("area", ctx.out("area", super::q(area, "m2", QT::Area))),
        (
            "perimeter",
            ctx.out("perimeter", super::q(perimeter, "m", QT::Distance)),
        ),
        ("parts", Json::Num(out.parts.len() as f64)),
        ("vertex_count", Json::Num(n as f64)),
        (
            "max_deviation",
            ctx.out(
                "max_deviation",
                Q {
                    value: out.max_deviation,
                    unit: m,
                },
            ),
        ),
        (
            "tolerance",
            ctx.out(
                "tolerance",
                Q {
                    value: out.tolerance,
                    unit: m,
                },
            ),
        ),
        ("boundary", Json::Arr(boundary)),
    ]))
}
