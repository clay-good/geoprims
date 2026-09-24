//! Computational geometry on the plane and the ellipsoid
//! (add-navigation-and-geometry, geometry/computational).

pub mod buffer;
pub mod densify;
pub mod distance;
pub mod envelope;
pub mod mesh;
pub mod overlay;
pub mod predicate;
pub mod relate;
pub mod robust;
pub mod shape;
pub mod simplify;
pub mod validity;

use geographiclib_rs::{PolygonArea, Winding};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::ellipsoid::{self, Ellipsoid};

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics (area of geodesic polygons, section 6)",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55, section 6",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};
const KARNEY_RHUMB: Reference = Reference {
    title: "The area of rhumb polygons",
    issuer: "Karney, C. F. F., Studia Geophysica et Geodaetica",
    year: 2024,
    edition: "Vol. 68, No. 3-4",
    locator: "pp. 99-120 (area between a rhumb line and the equator)",
    url: "https://doi.org/10.1007/s11200-024-0709-z",
};
const PLANIMETER: Reference = Reference {
    title: "GeographicLib Planimeter: area of geodesic polygons",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2013,
    edition: "GeographicLib 2.x",
    locator: "Planimeter utility",
    url: "https://geographiclib.sourceforge.io/C++/doc/Planimeter.1.html",
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

const POLYGON: Field = Field::new(
    "polygon",
    "Polygon",
    "Corners in order (ring 0), then any holes (ring 1, 2, …); the first corner need not be repeated",
    Kind::List { items: VERTEX, min: 3, max: 10_000 },
)
.required()
.core();

const E: [Field; 3] = ellipsoid::FIELDS;

fn q(v: f64, s: &str, qt: QT) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
    }
}

/// The rings, in degrees, with a repeated closing corner dropped.
fn read_rings(ctx: &mut Ctx) -> Result<Vec<Vec<(f64, f64)>>, ToolError> {
    let rows = ctx.rows("polygon")?;
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
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
        let ring = match r.get("ring") {
            None | Some(serde_json::Value::Null) => 0,
            Some(v) => match v.as_f64() {
                Some(x) if x.fract() == 0.0 && (0.0..=100.0).contains(&x) => x as usize,
                _ => {
                    return Err(ToolError::invalid(
                        &format!("/polygon/{i}/ring"),
                        "Ring must be a whole number from 0 (the outline) to 100.",
                    ));
                }
            },
        };
        if rings.len() <= ring {
            rings.resize(ring + 1, Vec::new());
        }
        rings[ring].push((lat, lon));
    }
    for (k, ring) in rings.iter_mut().enumerate() {
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if ring.len() < 3 {
            return Err(ToolError::invalid(
                "/polygon",
                if k == 0 {
                    "The outline (ring 0) needs at least 3 corners.".to_owned()
                } else {
                    format!("Hole {k} needs at least 3 corners.")
                },
            ));
        }
    }
    Ok(rings)
}

/// Signed area (counterclockwise positive, the smaller of the two regions
/// the ring bounds) and perimeter of one ring, in m² and m.
fn ring_area(g: &geographiclib_rs::Geodesic, ring: &[(f64, f64)]) -> (f64, f64) {
    let mut p = PolygonArea::new(g, Winding::CounterClockwise);
    for &(lat, lon) in ring {
        p.add_point(lat, lon);
    }
    let (perimeter, area, _) = p.compute(true);
    (area, perimeter)
}

/// Shoelace area (m², counterclockwise positive) and perimeter (m) of a ring
/// on a flat equirectangular map centered on the outline `base`, using the
/// mean Earth radius: the naive calculation, for comparison only.
fn planar_area(base: &[(f64, f64)], ring: &[(f64, f64)]) -> (f64, f64) {
    const R: f64 = 6_371_008.8;
    let lat0 = base.iter().map(|p| p.0).sum::<f64>() / base.len() as f64;
    let lon0 = base[0].1;
    let k = libm::cos(lat0.to_radians());
    let xy: Vec<(f64, f64)> = ring
        .iter()
        .map(|p| {
            let dlon = (p.1 - lon0 + 180.0).rem_euclid(360.0) - 180.0;
            (R * k * dlon.to_radians(), R * (p.0 - lat0).to_radians())
        })
        .collect();
    let n = xy.len();
    let (mut a, mut per) = (0.0, 0.0);
    for i in 0..n {
        let (p, q) = (xy[i], xy[(i + 1) % n]);
        a += p.0 * q.1 - q.0 * p.1;
        per += (q.0 - p.0).hypot(q.1 - p.1);
    }
    (a / 2.0, per)
}

/// Net longitude turned around the ring: ±360° when it circles a pole.
fn winding(ring: &[(f64, f64)]) -> f64 {
    (0..ring.len())
        .map(|i| (ring[(i + 1) % ring.len()].1 - ring[i].1 + 180.0).rem_euclid(360.0) - 180.0)
        .sum()
}

pub static POLYGON_AREA: ToolDef = ToolDef {
    id: "geometry.area.polygon",
    stability: gp_base::tool::Stability::Stable,
    title: "Polygon area and perimeter on the ellipsoid",
    summary: "The area and perimeter of a polygon with geodesic edges on WGS 84 (or any ellipsoid), with holes, across the antimeridian or around a pole, and its ring orientation.",
    aliases: &[
        "polygon area",
        "area calculator",
        "field area",
        "acreage from coordinates",
        "geodesic area",
    ],
    keywords: &[
        "area",
        "perimeter",
        "polygon",
        "acres",
        "hectares",
        "planimeter",
        "geodesic",
        "holes",
    ],
    inputs: &[
        POLYGON,
        Field::new(
            "edges",
            "Edges are",
            "geodesic (shortest paths, default), rhumb (constant course), or planar (flat shoelace, a rough check only)",
            Kind::Choice(&["geodesic", "rhumb", "planar"]),
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        Field::new(
            "area",
            "Area",
            "Outline minus holes",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(10)),
        Field::new(
            "perimeter",
            "Perimeter",
            "The outline and every hole",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "orientation",
            "Outline orientation",
            "clockwise or counterclockwise, seen from above",
            Kind::Text { max_len: 20 },
        ),
        Field::new(
            "pole",
            "Pole enclosed",
            "none, north, or south",
            Kind::Text { max_len: 8 },
        ),
        Field::new(
            "holes",
            "Holes",
            "Rings subtracted from the outline",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "outline_area",
            "Outline area",
            "Before holes",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(10)),
    ],
    errors: &[
        ErrorCode::Unsupported,
        ErrorCode::InvalidInput,
        ErrorCode::DegenerateGeometry,
    ],
    warnings: &[
        "POLE_ENCLOSED",
        "PLANAR_ON_GEOGRAPHIC",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    when_to_use: "Use this for the area and perimeter of ground: a parcel, a burn scar, a survey boundary, a service area, or any ring of latitudes and longitudes, measured on the ellipsoid rather than on a flat picture of it. It also says which way the ring is wound and whether it takes in a pole, both of which change what the number means.",
    limitations: "The edges are geodesics, which is what a boundary on the ground is, and not straight lines on a projection or arcs along a parallel: a ring whose corners all sit on one parallel still encloses real area, because every edge bows towards the pole and the longer ones bow further. A ring that crosses or touches itself has no single area and is refused rather than answered, with make-valid named as the way out. The area comes back positive whichever way the ring is wound, with the winding reported beside it, so a hole has to be given as a hole rather than as a ring walked backwards.",
    model: "Karney (2013) geodesic polygon area on WGS 84; rhumb edges by Karney (2024); planar by the shoelace formula on a local flat map",
    accuracy: "Geodesic and rhumb edges match GeographicLib Planimeter (with -R for rhumbs) within 1e-8 relative. Planar mode is a rough check that grows wrong with size",
    references: &[KARNEY, KARNEY_RHUMB, PLANIMETER],
    examples: &[Example {
        id: "primary",
        title: "A Colorado-shaped rectangle",
        input: r#"{"polygon":[{"lat":37,"lon":-109.05},{"lat":41,"lon":-109.05},{"lat":41,"lon":-102.05},{"lat":37,"lon":-102.05}]}"#,
        source: "add-navigation-and-geometry area scenario: 269,154.55 km², perimeter 2,099.854 km, clockwise",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("area", "area")],
    }],
    related: &[
        Related {
            id: "survey.cogo.area-by-coordinates",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "next",
        },
        Related {
            id: "geometry.validity.make-valid",
            reason: "next",
        },
    ],
    sentence: "The area is {area}, with a perimeter of {perimeter}.",
    limits: &[("batchRows", 1_000)],
    run: run_polygon_area,
    ..ToolDef::BLANK
};

fn run_polygon_area(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rings = read_rings(ctx)?;
    let e = Ellipsoid::from_ctx(ctx)?;
    let g = e.geodesic()?;
    // A ring that crosses itself has no single area: send it to be repaired.
    let refs: Vec<&Vec<(f64, f64)>> = rings.iter().collect();
    if let Some((r, edge, (lat, lon))) =
        validity::first_crossing(&geographiclib_rs::Geodesic::wgs84(), &refs)?
    {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            format!("Ring {r} crosses or touches itself at edge {} (from corner {}), near {lat:.6}°, {lon:.6}°, so it has no single area.", edge + 1, edge + 1),
        )
        .at("/polygon")
        .hint("Repair it with geometry.validity.make-valid, which splits a crossing ring into valid parts."));
    }
    let edges = ctx.choice("edges")?.unwrap_or("geodesic");
    let rhumb = gp_geo::rhumb::Rhumb::new(e.a, e.f);
    if edges == "rhumb" && rings.iter().flatten().any(|p| p.0.abs() >= 90.0) {
        return Err(ToolError::invalid(
            "/polygon",
            "A rhumb line cannot pass through a pole; move the corner off it or use geodesic edges.",
        ));
    }
    let measure = |ring: &[(f64, f64)]| match edges {
        "rhumb" => {
            let n = ring.len();
            let p: f64 = (0..n)
                .map(|i| {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    rhumb.inverse(a.0, a.1, b.0, b.1).0
                })
                .sum();
            (rhumb.ring_area(ring), p)
        }
        "planar" => planar_area(&rings[0], ring),
        _ => ring_area(&g, ring),
    };
    let (outline, mut perimeter) = measure(&rings[0]);
    let mut holes_area = 0.0;
    for hole in &rings[1..] {
        let (a, p) = measure(hole);
        holes_area += a.abs();
        perimeter += p;
    }
    let area = outline.abs() - holes_area;
    if area < 0.0 {
        return Err(ToolError::invalid(
            "/polygon",
            "The holes cover more than the outline; check the ring numbers.",
        ));
    }
    let turn = winding(&rings[0]);
    let pole = if turn.abs() < 180.0 {
        "none"
    } else if rings[0].iter().map(|p| p.0).sum::<f64>() >= 0.0 {
        "north"
    } else {
        "south"
    };
    if pole != "none" {
        ctx.warnings.push(Warning::new(
            "POLE_ENCLOSED",
            format!("The outline circles the {pole} pole, so the area is the cap on that side."),
        ));
    }
    ctx.model = Some(match edges {
        "rhumb" => format!("Karney (2024) rhumb polygon area on {}", e.describe()),
        "planar" => "Shoelace formula on a flat map of the polygon (equirectangular at its mean latitude, mean Earth radius)".to_owned(),
        _ => format!("Karney (2013) geodesic polygon area on {}", e.describe()),
    });
    if edges == "planar" {
        let geodesic: f64 = ring_area(&g, &rings[0]).0.abs()
            - rings[1..]
                .iter()
                .map(|h| ring_area(&g, h).0.abs())
                .sum::<f64>();
        let off = if geodesic > 0.0 {
            (area - geodesic) / geodesic * 100.0
        } else {
            0.0
        };
        ctx.warnings.push(Warning::new(
            "PLANAR_ON_GEOGRAPHIC",
            format!(
                "Planar math on latitude and longitude: this area is {}% off the geodesic one. Use geodesic edges (the default) for a real measurement.",
                gp_base::display::number(off, gp_base::tool::Precision::Significant(3), ctx.options.format)
            ),
        ));
    }
    Ok(Json::obj([
        ("area", ctx.out("area", q(area, "m2", QT::Area))),
        (
            "perimeter",
            ctx.out("perimeter", q(perimeter, "m", QT::Distance)),
        ),
        (
            "orientation",
            Json::str(if outline >= 0.0 {
                "counterclockwise"
            } else {
                "clockwise"
            }),
        ),
        ("pole", Json::str(pole)),
        ("holes", Json::Num((rings.len() - 1) as f64)),
        (
            "outline_area",
            ctx.out("outline_area", q(outline.abs(), "m2", QT::Area)),
        ),
    ]))
}

/// A ring with straight edges in longitude and latitude, cut into pieces of
/// at most 0.01° so its geodesic area is the area those edges enclose (the
/// pieces bow from the straight line by well under a millimeter).
pub(crate) fn densify_straight(ring: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let n = (((b.0 - a.0).abs().max((b.1 - a.1).abs())) / 0.01)
            .ceil()
            .max(1.0) as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            out.push((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)));
        }
    }
    out
}

pub static TOOLS: &[&ToolDef] = &[
    &POLYGON_AREA,
    &buffer::BUFFER,
    &shape::CENTROID,
    &envelope::BBOX,
    &envelope::ENCLOSING,
    &predicate::POINT_IN_POLYGON,
    &relate::RELATE,
    &validity::MAKE_VALID,
    &overlay::BOOLEAN,
    &distance::TRACKS,
    &simplify::RDP,
    &simplify::VISVALINGAM_WHYATT,
    &densify::DENSIFY,
    &mesh::DELAUNAY,
    &mesh::VORONOI,
];

pub static REGISTRY: Registry = Registry {
    module: "geometry",
    tools: TOOLS,
};

gp_base::export_module!("geometry", REGISTRY);
