//! Computational geometry on the plane and the ellipsoid
//! (add-navigation-and-geometry, geometry/computational).

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
        "Decimal degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees",
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

/// Net longitude turned around the ring: ±360° when it circles a pole.
fn winding(ring: &[(f64, f64)]) -> f64 {
    (0..ring.len())
        .map(|i| (ring[(i + 1) % ring.len()].1 - ring[i].1 + 180.0).rem_euclid(360.0) - 180.0)
        .sum()
}

pub static POLYGON_AREA: ToolDef = ToolDef {
    id: "geometry.area.polygon",
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
    inputs: &[POLYGON, E[0], E[1], E[2]],
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
    errors: &[ErrorCode::Unsupported, ErrorCode::InvalidInput],
    warnings: &[
        "POLE_ENCLOSED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Karney (2013) geodesic polygon area on WGS 84",
    accuracy: "Matches GeographicLib Planimeter within 1e-8 relative (3e-9 observed on 500 polygons); edges are geodesics",
    references: &[KARNEY, PLANIMETER],
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
    let (outline, mut perimeter) = ring_area(&g, &rings[0]);
    let mut holes_area = 0.0;
    for hole in &rings[1..] {
        let (a, p) = ring_area(&g, hole);
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
    ctx.model = Some(format!(
        "Karney (2013) geodesic polygon area on {}",
        e.describe()
    ));
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

pub static TOOLS: &[&ToolDef] = &[&POLYGON_AREA];

pub static REGISTRY: Registry = Registry {
    module: "geometry",
    tools: TOOLS,
};

gp_base::export_module!("geometry", REGISTRY);
