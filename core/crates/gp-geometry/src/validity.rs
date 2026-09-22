//! Validity and repair (add-navigation-and-geometry, geometry/computational,
//! "Validity and repair"): each problem with a polygon, where it is, and a
//! repaired version by the even-odd rule, which splits a bow-tie into two
//! triangles and drops spikes, with outlines counterclockwise per RFC 7946.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::{self, Aeqd};
use libm::ceil;

const OGC: Reference = Reference {
    title: "OpenGIS Implementation Specification for Geographic information - Simple feature access - Part 1: Common architecture",
    issuer: "Open Geospatial Consortium",
    year: 2011,
    edition: "OGC 06-103r4, version 1.2.1",
    locator: "Section 6.1.11.1 (polygon validity: closed simple rings, holes inside the exterior, interiors connected)",
    url: "https://www.ogc.org/standards/sfa",
};
const RFC7946: Reference = Reference {
    title: "The GeoJSON Format",
    issuer: "IETF RFC 7946",
    year: 2016,
    edition: "RFC 7946",
    locator: "Section 3.1.6 (exterior rings counterclockwise, holes clockwise)",
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
        "0 for the outline, 1, 2, … for holes",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];
const PROBLEM_ROW: &[Field] = &[
    Field::new(
        "problem",
        "Problem",
        "What is wrong",
        Kind::Text { max_len: 60 },
    ),
    Field::new(
        "ring",
        "Ring",
        "0 for the outline, 1, 2, … for holes",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "corner",
        "Corner or edge",
        "From 1; edge n runs from corner n to the next",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "lat",
        "Latitude",
        "Where",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Where",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];
const OUT_ROW: &[Field] = &[
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
        "0, 1, … for separate polygons",
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

pub static MAKE_VALID: ToolDef = ToolDef {
    id: "geometry.validity.make-valid",
    title: "Check and repair a polygon",
    summary: "Finds what is wrong with a polygon (crossed or touching edges, duplicate corners, spikes, holes outside the outline, clockwise rings), says where, and returns a repaired version: a bow-tie becomes two triangles.",
    aliases: &[
        "make valid",
        "fix polygon",
        "polygon validation",
        "self-intersection check",
        "bow-tie polygon repair",
    ],
    keywords: &[
        "valid",
        "invalid",
        "repair",
        "fix",
        "self-intersection",
        "bow-tie",
        "spike",
        "duplicate",
        "orientation",
        "topology",
    ],
    inputs: &[Field::new(
        "polygon",
        "Polygon",
        "Corners in order (ring 0), then any holes (ring 1, 2, …), like 40.4406, -80.002",
        Kind::List {
            items: VERTEX,
            min: 1,
            max: 20_000,
        },
    )
    .required()
    .core()],
    outputs: &[
        Field::new(
            "valid",
            "Valid",
            "yes, or no with the problems listed",
            Kind::Text { max_len: 3 },
        ),
        Field::new(
            "problem_count",
            "Problems",
            "Found, not counting orientation notes",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "problems",
            "Each problem",
            "What and where",
            Kind::List {
                items: PROBLEM_ROW,
                min: 0,
                max: 100_000,
            },
        ),
        Field::new(
            "parts",
            "Repaired parts",
            "Separate polygons after repair",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "area",
            "Repaired area",
            "Geodesic",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(8)),
        Field::new(
            "repaired",
            "Repaired polygon",
            "Outlines counterclockwise, holes clockwise",
            Kind::List {
                items: OUT_ROW,
                min: 0,
                max: 1_000_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["GEOMETRY_INVALID", "EXPERIMENTAL_TOOL"],
    model: "Geodesic edges cut into 5 km pieces on an azimuthal equidistant plane at the corners' mean. Checks per OGC Simple Features §6.1.11.1: every pair of edges meeting other than at a shared corner, repeated corners, spikes (a corner where the ring doubles back), holes whose corners lie outside the outline; orientation per RFC 7946 is noted, not counted. Repair keeps each piece of edge with the even-odd interior on one side only, split where edges cross, and joins the pieces into rings",
    accuracy: "Crossings located to well under 1 mm on the plane for shapes of a few hundred kilometers; the repaired area by Karney's geodesic polygon area",
    references: &[OGC, RFC7946],
    examples: &[Example {
        id: "primary",
        title: "A bow-tie field boundary",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.004,"lon":-104.995},{"lat":40.0,"lon":-104.995},{"lat":40.004,"lon":-105.0}]}"#,
        source: "add-navigation-and-geometry bow-tie scenario (two triangles, the crossing located)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("rings", "repaired")],
    }],
    related: &[
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
        Related {
            id: "geometry.predicate.point-in-polygon",
            reason: "alternative",
        },
    ],
    sentence: "{if problem_count > 0}Found {problem_count} {plural problem_count \"problem\" \"problems\"}; the repaired polygon has {parts} {plural parts \"part\" \"parts\"}.{/if}{if problem_count < 1}The polygon is valid.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_make_valid,
    ..ToolDef::BLANK
};

type P = (f64, f64);

/// The rings read in degrees, repeated corners reported and dropped.
fn read(
    ctx: &mut Ctx,
    problems: &mut Vec<(String, usize, usize, (f64, f64))>,
) -> Result<Vec<Vec<(f64, f64)>>, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows("polygon")?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut raw_index: Vec<usize> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("polygon", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("polygon", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/polygon/{i}/lat")));
        }
        let ring = r
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/polygon/{i}/ring"),
                "Ring must be a whole number from 0 (the outline) to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
            raw_index.resize(k + 1, 0);
        }
        raw_index[k] += 1;
        let lon = (lon + 540.0).rem_euclid(360.0) - 180.0;
        if rings[k].last() == Some(&(lat, lon)) {
            problems.push(("repeated corner".into(), k, raw_index[k], (lat, lon)));
        } else {
            rings[k].push((lat, lon));
        }
    }
    for (k, ring) in rings.iter_mut().enumerate() {
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if !ring.is_empty() && ring.len() < 3 {
            problems.push(("fewer than 3 corners".into(), k, 1, ring[0]));
        }
    }
    Ok(rings)
}

/// Rings cut into 5 km geodesic pieces on an azimuthal equidistant plane at
/// their corners' mean, each piece remembering the edge it came from.
pub(crate) struct Planar<'g> {
    pub map: Aeqd<'g>,
    pub plane: Vec<Vec<P>>,
    pub origin: Vec<Vec<usize>>,
    /// The farthest point from the plane's center, in meters.
    pub far: f64,
}

pub(crate) fn to_plane<'g>(
    g: &'g Geodesic,
    rings: &[&Vec<(f64, f64)>],
) -> Result<Planar<'g>, ToolError> {
    let all: Vec<(f64, f64)> = rings.iter().flat_map(|r| r.iter().copied()).collect();
    let (lat0, lon0) = buffer::center(&all);
    let map = Aeqd { g, lat0, lon0 };
    let mut plane: Vec<Vec<P>> = Vec::new();
    let mut origin: Vec<Vec<usize>> = Vec::new();
    let mut total = 0usize;
    for ring in rings {
        let n = ring.len();
        let (mut pts, mut from) = (Vec::new(), Vec::new());
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let (s, az, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
            let k = (ceil(s / 5_000.0) as usize).max(1);
            total += k;
            if total > 50_000 {
                return Err(ToolError::new(
                    ErrorCode::LimitExceeded,
                    "The polygon is too long to check here: over 50,000 edge pieces of 5 km.",
                )
                .at("/polygon"));
            }
            for j in 0..k {
                let p = if j == 0 {
                    a
                } else {
                    let (la, lo, _): (f64, f64, f64) =
                        g.direct(a.0, a.1, az, s * j as f64 / k as f64);
                    (la, lo)
                };
                pts.push(map.fwd(p));
                from.push(i);
            }
        }
        plane.push(pts);
        origin.push(from);
    }
    let far = plane
        .iter()
        .flatten()
        .map(|p| p.0.hypot(p.1))
        .fold(0.0, f64::max);
    Ok(Planar {
        map,
        plane,
        origin,
        far,
    })
}

/// Where a crossing is: (ring position, edge from 0, (lat, lon)).
pub(crate) type Found = (usize, usize, (f64, f64));

/// The first place a ring crosses or touches itself or another, as (ring
/// position in `rings`, edge from 0, lat, lon), if the rings fit within
/// 5,000 km of their center; farther out the plane cannot be trusted to say.
pub(crate) fn first_crossing(
    g: &Geodesic,
    rings: &[&Vec<(f64, f64)>],
) -> Result<Option<Found>, ToolError> {
    let pl = to_plane(g, rings)?;
    if pl.far > buffer::REACH * 5.0 {
        return Ok(None);
    }
    Ok(buffer::crossings(&pl.plane)
        .first()
        .map(|&(r, e, _, _, pt)| (r, pl.origin[r][e], pl.map.rev(pt))))
}

fn run_make_valid(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut problems: Vec<(String, usize, usize, (f64, f64))> = Vec::new();
    let rings = read(ctx, &mut problems)?;
    let used: Vec<(usize, &Vec<(f64, f64)>)> = rings
        .iter()
        .enumerate()
        .filter(|(_, r)| r.len() >= 3)
        .collect();
    if used.is_empty() {
        return Err(ToolError::invalid(
            "/polygon",
            "The polygon needs at least 3 distinct corners.",
        ));
    }
    let g = Geodesic::wgs84();
    let rs: Vec<&Vec<(f64, f64)>> = used.iter().map(|(_, r)| *r).collect();
    let Planar {
        map,
        plane,
        origin,
        far,
    } = to_plane(&g, &rs)?;
    if far > buffer::REACH * 5.0 {
        return Err(ToolError::new(ErrorCode::OutOfDomain, "The polygon is too large to check on one plane: keep it within 5,000 km of its center.").at("/polygon"));
    }
    // Spikes: a corner where the ring doubles straight back.
    for (k, ring) in &used {
        let n = ring.len();
        let pr: Vec<P> = ring.iter().map(|&p| map.fwd(p)).collect();
        for i in 0..n {
            let (a, b, c) = (pr[(i + n - 1) % n], pr[i], pr[(i + 1) % n]);
            let (d1, d2) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
            let (l1, l2) = (d1.0.hypot(d1.1), d2.0.hypot(d2.1));
            if (d1.0 * d2.1 - d1.1 * d2.0).abs() <= 1e-9 * l1 * l2
                && d1.0 * d2.0 + d1.1 * d2.1 < 0.0
            {
                problems.push(("spike".into(), *k, i + 1, ring[i]));
            }
        }
    }
    // Crossings and touches, located and named by ring and original edge.
    for (ra, ea, rb, eb, pt) in buffer::crossings(&plane) {
        let (ka, kb) = (used[ra].0, used[rb].0);
        let (oa, ob) = (origin[ra][ea], origin[rb][eb]);
        let what = if ka == kb {
            format!("edges {} and {} cross or touch", oa + 1, ob + 1)
        } else {
            format!("edge {} meets ring {} edge {}", oa + 1, kb, ob + 1)
        };
        let label = if ka == kb {
            format!("self-intersection: {what}")
        } else {
            format!("rings cross or touch: {what}")
        };
        let ll = map.rev(pt);
        if !problems
            .iter()
            .any(|p| p.0 == label && p.1 == ka && p.2 == oa + 1 && (p.3.0 - ll.0).abs() < 1e-9)
        {
            problems.push((label, ka, oa + 1, ll));
        }
    }
    // Holes must lie inside the outline.
    if used[0].0 == 0 {
        for (u, (k, ring)) in used.iter().enumerate().skip(1) {
            if !buffer::inside_rings(std::slice::from_ref(&plane[0]), plane[u][0]) {
                problems.push(("hole outside the outline".into(), *k, 1, ring[0]));
            }
        }
    }
    let counted = problems.len();
    // Orientation (RFC 7946): noted, not counted, since Simple Features allows either.
    for (u, (k, ring)) in used.iter().enumerate() {
        let a2: f64 = (0..plane[u].len())
            .map(|i| {
                let (p, q) = (plane[u][i], plane[u][(i + 1) % plane[u].len()]);
                p.0 * q.1 - q.0 * p.1
            })
            .sum();
        let hole = *k > 0;
        if (a2 < 0.0 && !hole) || (a2 > 0.0 && hole) {
            problems.push((
                if hole {
                    "note: hole runs counterclockwise (RFC 7946 wants clockwise)"
                } else {
                    "note: outline runs clockwise (RFC 7946 wants counterclockwise)"
                }
                .into(),
                *k,
                1,
                ring[0],
            ));
        }
    }
    let repaired = buffer::even_odd(&plane);
    let (mut outers, mut holes): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for (i, r) in repaired.iter().enumerate() {
        let a2: f64 = (0..r.len())
            .map(|j| r[j].0 * r[(j + 1) % r.len()].1 - r[(j + 1) % r.len()].0 * r[j].1)
            .sum();
        if a2 > 0.0 {
            outers.push(i)
        } else {
            holes.push(i)
        }
    }
    let mut parts: Vec<(usize, Vec<usize>)> = outers.iter().map(|&o| (o, Vec::new())).collect();
    for h in holes {
        if let Some(p) = parts.iter_mut().find(|(o, _)| {
            buffer::inside_rings(std::slice::from_ref(&repaired[*o]), repaired[h][0])
        }) {
            p.1.push(h);
        }
    }
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    let mut rows = Vec::new();
    let mut area = 0.0;
    for (pi, (o, hs)) in parts.iter().enumerate() {
        for (ri, &idx) in core::iter::once(o).chain(hs.iter()).enumerate() {
            let ring: Vec<(f64, f64)> = repaired[idx].iter().map(|&p| map.rev(p)).collect();
            let (a, _) = super::ring_area(&g, &ring);
            area += if ri == 0 { a.abs() } else { -a.abs() };
            for &(la, lo) in &ring {
                rows.push(Json::obj([
                    ("lat", dq(la)),
                    ("lon", dq(lo)),
                    ("part", Json::Num(pi as f64)),
                    ("ring", Json::Num(ri as f64)),
                ]));
            }
        }
    }
    if counted > 0 {
        let first = &problems[0];
        ctx.warnings.push(Warning::new(
            "GEOMETRY_INVALID",
            format!("{} problem{} found, the first a {} in ring {} near {:.6}°, {:.6}°; the repaired polygon is returned.", counted, if counted == 1 { "" } else { "s" }, first.0, first.1, first.3.0, first.3.1),
        ));
    }
    let list = problems
        .iter()
        .map(|(what, ring, corner, (la, lo))| {
            Json::obj([
                ("problem", Json::str(what.clone())),
                ("ring", Json::Num(*ring as f64)),
                ("corner", Json::Num(*corner as f64)),
                ("lat", dq(*la)),
                ("lon", dq(*lo)),
            ])
        })
        .collect();
    Ok(Json::obj([
        ("valid", Json::str(if counted == 0 { "yes" } else { "no" })),
        ("problem_count", Json::Num(counted as f64)),
        ("problems", Json::Arr(list)),
        ("parts", Json::Num(parts.len() as f64)),
        ("area", ctx.out("area", super::q(area, "m2", QT::Area))),
        ("repaired", Json::Arr(rows)),
    ]))
}
