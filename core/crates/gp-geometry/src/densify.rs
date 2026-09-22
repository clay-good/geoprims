//! Densification (add-navigation-and-geometry, geometry/computational,
//! "Densification, triangulation, Voronoi, and distances"): every edge of a
//! line or polygon is cut into equal geodesic pieces no longer than a maximum,
//! so software that draws straight lines between vertices follows the
//! geodesic closely.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (the direct and inverse problems)",
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
];

/// The most vertices a result may hold.
const MAX_OUT: usize = 100_000;

pub static DENSIFY: ToolDef = ToolDef {
    id: "geometry.shape.densify",
    title: "Add vertices along geodesic edges (densify)",
    summary: "Adds vertices along each edge of a line or polygon so no piece is longer than a set length, each on the true geodesic, so maps that draw straight segments still follow the Earth's curve.",
    aliases: &[
        "densify a line",
        "densify polygon",
        "add points along a line",
        "geodesic densify",
    ],
    keywords: &[
        "densify",
        "vertices",
        "segment length",
        "geodesic",
        "great circle",
        "line",
    ],
    inputs: &[
        Field::new(
            "points",
            "Vertices",
            "In order, like 40.0, -105.0 on each row",
            Kind::List {
                items: VERTEX,
                min: 2,
                max: 10_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "max_length",
            "Longest piece",
            "No piece of an edge will be longer, like 10 km",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .required()
        .core(),
        Field::new(
            "shape",
            "Shape",
            "line (default) or polygon, whose closing edge is densified too",
            Kind::Choice(&["line", "polygon"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "vertices_out",
            "Vertices now",
            "After densifying",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "vertices_in",
            "Vertices given",
            "Before densifying",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "longest_piece",
            "Longest piece",
            "Of the densified edges, along the geodesic",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "length",
            "Total length",
            "Along the geodesics, unchanged by densifying",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "densified",
            "Densified vertices",
            "In order; a polygon is not closed with a repeated corner",
            Kind::List {
                items: OUT_VERTEX,
                min: 0,
                max: MAX_OUT,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Each edge's geodesic on WGS 84 (Karney inverse) is cut into n = ⌈length / longest piece⌉ equal pieces, with the new vertices placed by the direct problem",
    accuracy: "Every new vertex lies on its edge's geodesic to a nanometer; the pieces of an edge are equal to a nanometer",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Denver to Chicago, in pieces of at most 200 km",
        input: r#"{"points":[{"lat":39.8561,"lon":-104.6737},{"lat":41.9742,"lon":-87.9073}],"max_length":"200 km"}"#,
        source: "Karney (2013) inverse and direct on WGS 84",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.waypoints",
            reason: "alternative",
        },
        Related {
            id: "geometry.simplify.rdp",
            reason: "alternative",
        },
    ],
    sentence: "The line now has {vertices_out} vertices, none more than {longest_piece} apart.",
    limits: &[("batchRows", 100)],
    run: run_densify,
    ..ToolDef::BLANK
};

fn run_densify(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("points")?;
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let m = units::by_symbol(QT::Distance, "m").expect("m");
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
        if !(-90.0..=90.0).contains(&lat) || !lon.is_finite() {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/points/{i}/lat")));
        }
        pts.push((lat, lon));
    }
    let ring = ctx.choice("shape")? == Some("polygon");
    if ring && pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < if ring { 3 } else { 2 } {
        return Err(ToolError::invalid(
            "/points",
            if ring {
                "A polygon needs at least 3 corners."
            } else {
                "A line needs at least 2 points."
            },
        ));
    }
    let max = ctx.req_quantity("max_length")?.base();
    if max.is_nan() || max <= 0.0 {
        return Err(ToolError::invalid(
            "/max_length",
            "The longest piece must be greater than 0.",
        ));
    }
    let g = Geodesic::wgs84();
    let edges = if ring { pts.len() } else { pts.len() - 1 };
    let mut out: Vec<(f64, f64)> = Vec::new();
    let (mut total, mut longest) = (0.0f64, 0.0f64);
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let (s, az, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
        let n = libm::ceil(s / max).max(1.0);
        if out.len() as f64 + n > MAX_OUT as f64 {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                format!("That would make more than {MAX_OUT} vertices; choose a longer piece."),
            )
            .at("/max_length"));
        }
        let n = n as usize;
        out.push(a);
        for k in 1..n {
            let (lat, lon): (f64, f64) = g.direct(a.0, a.1, az, s * k as f64 / n as f64);
            out.push((lat, gp_base::angle::wrap_lon(lon)));
        }
        total += s;
        longest = longest.max(s / n as f64);
    }
    if !ring {
        out.push(pts[pts.len() - 1]);
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Longest piece",
            "each edge's length / ⌈length / longest piece⌉",
            format!(
                "{} edges, {} km in all",
                edges,
                display::number(total / 1000.0, Precision::Decimals(3), fmt)
            ),
            display::quantity(longest / 1000.0, "km", Precision::Decimals(3), fmt),
        );
        ctx.step(
            "Vertices now",
            "the given vertices plus the pieces' new ends",
            format!("{} given", pts.len()),
            display::number(out.len() as f64, Precision::Decimals(0), fmt),
        );
    }
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    let rows: Vec<Json> = out
        .iter()
        .map(|&(lat, lon)| Json::obj([("lat", dq(lat)), ("lon", dq(lon))]))
        .collect();
    Ok(Json::obj(vec![
        ("vertices_out", Json::Num(out.len() as f64)),
        ("vertices_in", Json::Num(pts.len() as f64)),
        (
            "longest_piece",
            ctx.out(
                "longest_piece",
                Q {
                    value: longest,
                    unit: m,
                },
            ),
        ),
        (
            "length",
            ctx.out(
                "length",
                Q {
                    value: total,
                    unit: m,
                },
            ),
        ),
        ("densified", Json::Arr(rows)),
    ]))
}
