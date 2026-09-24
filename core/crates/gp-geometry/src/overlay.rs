//! Boolean operations (add-navigation-and-geometry, geometry/computational,
//! "Boolean operations"): the intersection, union, difference, or symmetric
//! difference of two polygons with geodesic edges, as valid polygons with
//! their geodesic area.

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::{self, Op};

use crate::validity::to_plane;

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (direct and inverse problems; area of geodesic polygons, section 6)",
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
        "0 for the outline, 1, 2, … for holes",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
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
const fn km2(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Area,
            unit: "km2",
        },
    )
    .precision(Precision::Significant(8))
}

pub static BOOLEAN: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geometry.overlay.boolean",
    version: "1.1.1",
    title: "Overlap, union, or difference of two polygons",
    summary: "Where two areas overlap, their combined outline, what one has that the other lacks, or both, as valid polygons with geodesic areas, like the overlap of two geofences.",
    aliases: &["polygon intersection", "polygon union", "polygon difference", "overlap of two areas", "clip polygon", "boolean operation"],
    keywords: &["intersection", "union", "difference", "overlap", "clip", "merge", "subtract", "xor", "geofence", "overlay"],
    inputs: &[
        Field::new("polygon_a", "First polygon", "Corners in order (ring 0), then any holes, like 40.4406, -80.002", Kind::List { items: VERTEX, min: 3, max: 10_000 }).required().core(),
        Field::new("polygon_b", "Second polygon", "Corners in order (ring 0), then any holes, like 40.4406, -80.002", Kind::List { items: VERTEX, min: 3, max: 10_000 }).required().core(),
        Field::new("operation", "Operation", "intersection (the default), union, difference (first minus second), or symmetric-difference", Kind::Choice(&["intersection", "union", "difference", "symmetric-difference"])).core(),
    ],
    outputs: &[
        km2("area", "Result area", "Geodesic"),
        Field::new("parts", "Parts", "Separate polygons in the result; 0 when it is empty", Kind::Number { min: 0.0, max: 1e6 }).precision(Precision::Decimals(0)),
        km2("area_a", "First polygon's area", "Geodesic"),
        km2("area_b", "Second polygon's area", "Geodesic"),
        Field::new("result", "Result", "Outlines counterclockwise, holes clockwise", Kind::List { items: OUT_ROW, min: 0, max: 1_000_000 }),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &[],
    model: "Both polygons' geodesic edges cut into 5 km pieces on one azimuthal equidistant plane at their corners' mean, each read by the even-odd rule; corners within a millionth of the shapes' size (at most 1 mm) of another ring are first put on it, so boundaries either meet exactly or stay clearly apart; every piece of either boundary is kept exactly when the result's inside differs on its two sides, and the pieces are joined into rings, taking the sharpest left turn where rings meet at a corner, so pieces that touch at a point come out as separate parts. Areas by Karney's geodesic polygon area (Karney 2013)",
    accuracy: "Edges follow the geodesics to about 1 mm for shapes of a few hundred kilometers; areas exact for the returned corners. Inputs within 5,000 km of their shared center",
    when_to_use: "Use this to ask how two areas relate as areas rather than as outlines: how much of a flight restriction falls inside a planned survey block, what a parcel keeps after a right of way is taken out of it, the combined footprint of two coverage zones, the part of a search area nobody has swept yet. It answers with the polygon itself and with its area on the ellipsoid, so the result can be drawn, measured, or fed straight back in.",
    limitations: "Both polygons and their result must sit within 5,000 km of their shared centre, because the overlay is done on one plane placed there and a plane cannot hold more of the Earth than that faithfully; further apart and the tool refuses rather than distorting. Edges are cut into 5 km pieces before the overlay, so a result boundary follows the geodesic to about a millimetre rather than exactly. A result can be empty, or break into several pieces, or acquire a hole, all of which are reported rather than treated as failure — a difference that leaves nothing is a correct answer. Self-intersecting inputs have no well-defined inside and should be repaired first.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Where two geofences overlap",
        input: r#"{"polygon_a":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.008,"lon":-104.99},{"lat":40.008,"lon":-105.0}],"polygon_b":[{"lat":40.004,"lon":-104.995},{"lat":40.004,"lon":-104.985},{"lat":40.012,"lon":-104.985},{"lat":40.012,"lon":-104.995}],"operation":"intersection"}"#,
        source: "GEOS 3.11.4 through shapely, the reference implementation for this operation, run on the same azimuthal equidistant plane and measured back on the ellipsoid by geographiclib: over ten cases across all four operations the areas agree to 2.2e-11 relative and the part counts exactly",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "polygon", map: &[("rings", "result")] }],
    related: &[
        Related { id: "geometry.area.polygon", reason: "next" },
        Related { id: "geometry.validity.make-valid", reason: "alternative" },
        Related { id: "geometry.buffer.geodesic", reason: "alternative" },
    ],
    sentence: "{if parts > 0}The result covers {area} in {parts} {plural parts \"part\" \"parts\"}.{/if}{if parts < 1}The result is empty: the polygons do not overlap that way.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_boolean,
    ..ToolDef::BLANK
};

fn read(ctx: &mut Ctx, list: &str) -> Result<Vec<Vec<(f64, f64)>>, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows(list)?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity(list, i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity(list, i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/{list}/{i}/lat")));
        }
        let ring = r
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/{list}/{i}/ring"),
                "Ring must be a whole number from 0 (the outline) to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        let lon = (lon + 540.0).rem_euclid(360.0) - 180.0;
        if rings[k].last() != Some(&(lat, lon)) {
            rings[k].push((lat, lon));
        }
    }
    rings.retain(|r| !r.is_empty());
    for (k, ring) in rings.iter_mut().enumerate() {
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if ring.len() < 3 {
            return Err(ToolError::invalid(
                &format!("/{list}"),
                format!("Ring {k} needs at least 3 distinct corners."),
            ));
        }
        gp_geo::point::refuse_repeated_corner(ring, &format!("/{list}"))?;
    }
    Ok(rings)
}

fn geodesic_area(g: &Geodesic, rings: &[Vec<(f64, f64)>]) -> f64 {
    rings
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let a = super::ring_area(g, r).0.abs();
            if k == 0 { a } else { -a }
        })
        .sum()
}

fn run_boolean(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let a = read(ctx, "polygon_a")?;
    let b = read(ctx, "polygon_b")?;
    let op = match ctx.choice("operation")? {
        Some("union") => Op::Union,
        Some("difference") => Op::Difference,
        Some("symmetric-difference") => Op::SymmetricDifference,
        _ => Op::Intersection,
    };
    let g = Geodesic::wgs84();
    let all: Vec<&Vec<(f64, f64)>> = a.iter().chain(b.iter()).collect();
    let pl = to_plane(&g, &all)?;
    if pl.far > buffer::REACH * 5.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The polygons must lie within 5,000 km of their shared center.",
        )
        .at("/polygon_b"));
    }
    let (pa, pb) = pl.plane.split_at(a.len());
    let out = buffer::boolean(pa, pb, op);
    let (mut outers, mut holes): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for (i, r) in out.iter().enumerate() {
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
    // A hole belongs to the outline holding most of its corners: one corner
    // may be where it touches that outline, and there the test could go
    // either way.
    for h in holes {
        let held = |o: usize| {
            out[h]
                .iter()
                .filter(|&&c| buffer::inside_rings(std::slice::from_ref(&out[o]), c))
                .count()
        };
        if let Some(p) = parts
            .iter_mut()
            .filter(|(o, _)| held(*o) > 0)
            .max_by_key(|(o, _)| held(*o))
        {
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
    let (mut rows, mut area) = (Vec::new(), 0.0);
    for (pi, (o, hs)) in parts.iter().enumerate() {
        for (ri, &idx) in core::iter::once(o).chain(hs.iter()).enumerate() {
            let ring: Vec<(f64, f64)> = out[idx].iter().map(|&p| pl.map.rev(p)).collect();
            let r_area = super::ring_area(&g, &ring).0.abs();
            area += if ri == 0 { r_area } else { -r_area };
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
    Ok(Json::obj([
        ("area", ctx.out("area", super::q(area, "m2", QT::Area))),
        ("parts", Json::Num(parts.len() as f64)),
        (
            "area_a",
            ctx.out("area_a", super::q(geodesic_area(&g, &a), "m2", QT::Area)),
        ),
        (
            "area_b",
            ctx.out("area_b", super::q(geodesic_area(&g, &b), "m2", QT::Area)),
        ),
        ("result", Json::Arr(rows)),
    ]))
}
