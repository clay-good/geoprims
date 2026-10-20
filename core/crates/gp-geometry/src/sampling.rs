//! Sampling layouts in a polygon (add-sampling-and-grids): the polygon is
//! carried to the transverse Mercator plane of its UTM zone, worked there,
//! and the answer returned as latitude and longitude. Randomness comes only
//! from `gp_base::rng`, seeded by the reader, so a seed fixes every point.

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::rng::Xoshiro256;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::tm::Tm;
use gp_geo::utmups::{UTM_K0, central_meridian, standard_zone};
use serde_json::Value;

use crate::{POLYGON, q, read_rings, ring_area, validity};

pub(crate) const OSADA: Reference = Reference {
    title: "Shape distributions",
    issuer: "Osada, R., Funkhouser, T., Chazelle, B., and Dobkin, D., ACM Transactions on Graphics",
    year: 2002,
    edition: "Vol. 21, No. 4",
    locator: "Section 4.2, a random point in a triangle: P = (1 − √r₁)A + √r₁(1 − r₂)B + √r₁r₂C",
    url: "https://doi.org/10.1145/571647.571648",
};
pub(crate) const VIGNA: Reference = Reference {
    title: "Scrambled linear pseudorandom number generators",
    issuer: "Blackman, D., and Vigna, S., ACM Transactions on Mathematical Software",
    year: 2021,
    edition: "Vol. 47, No. 4",
    locator: "xoshiro256**, seeded by splitmix64 as the authors suggest; doubles from the top 53 bits",
    url: "https://doi.org/10.1145/3460772",
};
pub(crate) const EPA_G5S: Reference = Reference {
    title: "Guidance on Choosing a Sampling Design for Environmental Data Collection (EPA QA/G-5S)",
    issuer: "US Environmental Protection Agency",
    year: 2002,
    edition: "EPA/240/R-02/005, December 2002",
    locator: "Chapter 5, simple random sampling (p. 33)",
    url: "https://www.epa.gov/sites/default/files/2015-06/documents/g5s-final.pdf",
};

/// Above this extent one plane distorts too much (design S3 and Risks).
const MAX_EXTENT_M: f64 = 500_000.0;
const MAX_POINTS: f64 = 100_000.0;
/// Corners the triangulation takes (ear clipping is quadratic).
const MAX_CORNERS: usize = 2_000;

type P = (f64, f64);

fn orient(a: P, b: P, c: P) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn twice_area(r: &[P]) -> f64 {
    (0..r.len())
        .map(|i| {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum()
}

/// The polygon in its UTM zone's transverse Mercator plane (no false origin,
/// so it stays continuous across the equator), with the way back.
pub(crate) struct Plane {
    pub zone: u8,
    pub north: bool,
    cm: f64,
    tm: Tm,
    /// Ring 0 counterclockwise, holes clockwise, in meters.
    pub rings: Vec<Vec<P>>,
    /// Geographic rings as read, (lat, lon).
    pub geo: Vec<Vec<(f64, f64)>>,
    /// Point scale at the outline's mean corner: plane meters per ground meter.
    pub scale: f64,
}

impl Plane {
    pub fn read(ctx: &mut Ctx) -> Result<Plane, ToolError> {
        let geo = read_rings(ctx)?;
        if geo.iter().map(Vec::len).sum::<usize>() > MAX_CORNERS {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                format!("At most {MAX_CORNERS} corners in all; simplify the polygon first."),
            )
            .at("/polygon"));
        }
        let refs: Vec<&Vec<(f64, f64)>> = geo.iter().collect();
        if let Some((r, edge, _)) = validity::first_crossing(&Geodesic::wgs84(), &refs)? {
            return Err(ToolError::new(
                ErrorCode::DegenerateGeometry,
                format!(
                    "Ring {r} crosses or touches itself at edge {}, so it does not bound one area.",
                    edge + 1
                ),
            )
            .at("/polygon")
            .hint("Repair it with geometry.validity.make-valid first."));
        }
        // The zone of the outline's mean corner, longitudes unwrapped about the first.
        let lon0 = geo[0][0].1;
        let unwrap = |lon: f64| lon0 + (lon - lon0 + 180.0).rem_euclid(360.0) - 180.0;
        let n0 = geo[0].len() as f64;
        let lat_c = geo[0].iter().map(|p| p.0).sum::<f64>() / n0;
        let lon_c = gp_base::angle::wrap_lon(geo[0].iter().map(|p| unwrap(p.1)).sum::<f64>() / n0);
        let zone = standard_zone(lat_c.clamp(-80.0, 84.0), lon_c);
        let cm = central_meridian(zone);
        let g = Geodesic::wgs84();
        let tm = Tm::new(g.a, g.f, UTM_K0);
        let mut rings: Vec<Vec<P>> = geo
            .iter()
            .map(|r| {
                r.iter()
                    .map(|&(lat, lon)| {
                        let dlon = (lon - cm + 180.0).rem_euclid(360.0) - 180.0;
                        let (x, y, ..) = tm.forward(lat, dlon);
                        (x, y)
                    })
                    .collect()
            })
            .collect();
        for (k, r) in rings.iter_mut().enumerate() {
            let ccw = twice_area(r) > 0.0;
            if (k == 0) != ccw {
                r.reverse();
            }
        }
        let xs = rings[0].iter().map(|p| p.0);
        let ys = rings[0].iter().map(|p| p.1);
        let w = xs.clone().fold(f64::NEG_INFINITY, f64::max) - xs.fold(f64::INFINITY, f64::min);
        let h = ys.clone().fold(f64::NEG_INFINITY, f64::max) - ys.fold(f64::INFINITY, f64::min);
        if w.max(h) > MAX_EXTENT_M {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                format!("The polygon spans {} km; one flat grid is good to 500 km, past which its distortion grows too large. Split the area.", gp_base::display::number(w.max(h) / 1000.0, Precision::Decimals(0), ctx.options.format)),
            )
            .at("/polygon"));
        }
        let dlon_c = (lon_c - cm + 180.0).rem_euclid(360.0) - 180.0;
        let scale = tm.forward(lat_c, dlon_c).3;
        Ok(Plane {
            zone,
            north: lat_c >= 0.0,
            cm,
            tm,
            rings,
            geo,
            scale,
        })
    }

    /// (lat, lon) of a plane point.
    pub fn to_geo(&self, p: P) -> (f64, f64) {
        let (lat, dlon) = self.tm.inverse(p.0, p.1);
        (lat, gp_base::angle::wrap_lon(dlon + self.cm))
    }

    /// Inside the outline and outside every hole (even-odd over all rings).
    pub fn inside(&self, p: P) -> bool {
        let mut c = false;
        for r in &self.rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1)
                {
                    c = !c;
                }
            }
        }
        c
    }

    /// Plane area of the polygon, holes out, in plane m².
    pub fn plane_area(&self) -> f64 {
        self.rings.iter().map(|r| twice_area(r) / 2.0).sum()
    }

    pub fn crs(&self) -> String {
        format!(
            "UTM zone {}{} (WGS 84)",
            self.zone,
            if self.north { "N" } else { "S" }
        )
    }

    /// Geodesic area of the polygon, holes out, in m².
    pub fn area(&self) -> f64 {
        let g = Geodesic::wgs84();
        ring_area(&g, &self.geo[0]).0.abs()
            - self.geo[1..]
                .iter()
                .map(|h| ring_area(&g, h).0.abs())
                .sum::<f64>()
    }
}

/// Whether segment ab properly crosses cd, or touches it at a point other
/// than a shared end.
fn blocks(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    let within = |p: P, q: P, r: P| {
        r != p
            && r != q
            && r.0 >= p.0.min(q.0)
            && r.0 <= p.0.max(q.0)
            && r.1 >= p.1.min(q.1)
            && r.1 <= p.1.max(q.1)
    };
    (o1 == 0.0 && within(a, b, c)) || (o2 == 0.0 && within(a, b, d))
}

/// Triangles of a polygon with holes, by ear clipping (documented to the step
/// in the derivation note so it can be reimplemented): each hole, in order of
/// its largest x, is joined to the nearest outline corner the join can reach
/// without crossing a side, then ears are cut from the joined ring.
pub(crate) fn triangulate(rings: &[Vec<P>]) -> Option<Vec<[P; 3]>> {
    let mut outer: Vec<P> = rings[0].clone();
    let mut holes: Vec<(usize, &Vec<P>)> = rings[1..].iter().enumerate().collect();
    holes.sort_by(|a, b| {
        let mx = |h: &Vec<P>| h.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        mx(b.1).total_cmp(&mx(a.1)).then(a.0.cmp(&b.0))
    });
    for (k, &(_, hole)) in holes.iter().enumerate() {
        let m = (0..hole.len()).fold(0, |best, i| if hole[i].0 > hole[best].0 { i } else { best });
        let mp = hole[m];
        let mut order: Vec<usize> = (0..outer.len()).collect();
        let d2 = |p: P| (p.0 - mp.0).powi(2) + (p.1 - mp.1).powi(2);
        order.sort_by(|&a, &b| d2(outer[a]).total_cmp(&d2(outer[b])).then(a.cmp(&b)));
        let sides = |r: &Vec<P>| {
            (0..r.len())
                .map(|i| (r[i], r[(i + 1) % r.len()]))
                .collect::<Vec<_>>()
        };
        let mut walls = sides(&outer);
        for (_, h) in &holes[k..] {
            walls.extend(sides(h));
        }
        let p = order.into_iter().find(|&i| {
            let v = outer[i];
            v != mp
                && walls
                    .iter()
                    .all(|&(c, d)| !blocks(mp, v, c, d) && !blocks(c, d, mp, v))
        })?;
        let mut joined = outer[..=p].to_vec();
        joined.extend((0..=hole.len()).map(|j| hole[(m + j) % hole.len()]));
        joined.extend_from_slice(&outer[p..]);
        outer = joined;
    }
    let mut v = outer;
    let mut tris = Vec::with_capacity(v.len().saturating_sub(2));
    let mut i = 0;
    while v.len() > 3 {
        let n = v.len();
        let mut cut = false;
        for _ in 0..n {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            if orient(a, b, c) > 0.0 && is_ear(&v, a, b, c) {
                tris.push([a, b, c]);
                v.remove(i);
                i %= v.len();
                cut = true;
                break;
            }
            i = (i + 1) % n;
        }
        if !cut {
            return None;
        }
    }
    tris.push([v[0], v[1], v[2]]);
    Some(tris)
}

/// No reflex corner of the ring other than a, b, c lies in or on abc.
fn is_ear(v: &[P], a: P, b: P, c: P) -> bool {
    let n = v.len();
    (0..n).all(|j| {
        let p = v[j];
        if p == a || p == b || p == c {
            return true;
        }
        let reflex = orient(v[(j + n - 1) % n], p, v[(j + 1) % n]) <= 0.0;
        !(reflex && orient(a, b, p) >= 0.0 && orient(b, c, p) >= 0.0 && orient(c, a, p) >= 0.0)
    })
}

/// Triangles with their running areas, for choosing one by area.
pub(crate) struct Pieces {
    tris: Vec<[P; 3]>,
    cum: Vec<f64>,
}

impl Pieces {
    pub fn new(tris: Vec<[P; 3]>) -> Pieces {
        let mut cum = Vec::with_capacity(tris.len());
        let mut total = 0.0;
        for t in &tris {
            total += orient(t[0], t[1], t[2]) / 2.0;
            cum.push(total);
        }
        Pieces { tris, cum }
    }

    pub fn area(&self) -> f64 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    /// One uniform point, from exactly two draws: the first picks the
    /// triangle by area and, rescaled within its share, is r₁; the second is
    /// r₂ (Osada et al. 2002 §4.2).
    pub fn draw(&self, g: &mut Xoshiro256) -> P {
        let u = g.next_f64() * self.area();
        let k = self
            .cum
            .partition_point(|&c| c <= u)
            .min(self.tris.len() - 1);
        let lo = if k == 0 { 0.0 } else { self.cum[k - 1] };
        let r1 = ((u - lo) / (self.cum[k] - lo)).clamp(0.0, 1.0);
        let r2 = g.next_f64();
        let s = libm::sqrt(r1);
        let [a, b, c] = self.tris[k];
        (
            (1.0 - s) * a.0 + s * (1.0 - r2) * b.0 + s * r2 * c.0,
            (1.0 - s) * a.1 + s * (1.0 - r2) * b.1 + s * r2 * c.1,
        )
    }
}

/// The seed: a whole number from 0 to 2⁶⁴ − 1, as a JSON integer or text.
pub(crate) fn seed(ctx: &Ctx) -> Result<u64, ToolError> {
    let bad = || {
        ToolError::invalid(
            "/seed",
            "The seed is a whole number from 0 to 18446744073709551615, like 42.",
        )
    };
    match ctx.raw("seed") {
        None | Some(Value::Null) => Ok(1),
        Some(Value::Number(n)) => n.as_u64().ok_or_else(bad),
        Some(Value::String(s)) => s.trim().parse::<u64>().map_err(|_| bad()),
        Some(_) => Err(bad()),
    }
}

pub(crate) const SEED: Field = Field::new(
    "seed",
    "Seed",
    "A whole number that fixes the layout, like 42; the same seed gives the same points",
    Kind::Text { max_len: 20 },
);

const OUT_POINT: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];

pub static RANDOM_POINTS: ToolDef = ToolDef {
    id: "geometry.sampling.random-points",
    title: "Random sample points in a polygon",
    summary: "A simple random sample: N points spread uniformly over a polygon, holes left out, reproducible from a seed, with the area each point stands for.",
    aliases: &[
        "random points in polygon",
        "simple random sample",
        "random sampling locations",
    ],
    keywords: &[
        "random", "sample", "sampling", "points", "polygon", "seed", "plots",
    ],
    inputs: &[
        POLYGON,
        Field::new(
            "count",
            "Number of points",
            "How many sample points, like 30",
            Kind::Number {
                min: 1.0,
                max: MAX_POINTS,
            },
        )
        .required()
        .core(),
        SEED.core(),
    ],
    outputs: &[
        Field::new(
            "points",
            "Points",
            "In the order drawn",
            Kind::List {
                items: OUT_POINT,
                min: 1,
                max: 100_000,
            },
        ),
        Field::new(
            "area_per_point",
            "Area per point",
            "The polygon's area divided by the number of points",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "area",
            "Polygon area",
            "Geodesic, holes left out",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Significant(8)),
        Field::new(
            "count",
            "Points placed",
            "",
            Kind::Number {
                min: 1.0,
                max: MAX_POINTS,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new("seed", "Seed", "", Kind::Text { max_len: 20 }),
        Field::new(
            "crs",
            "Plane",
            "Where the points were placed",
            Kind::Text { max_len: 40 },
        ),
        Field::new("design", "Design", "", Kind::Text { max_len: 120 }),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
        ErrorCode::LimitExceeded,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "The polygon is carried to its UTM zone's transverse Mercator plane and cut into triangles by ear clipping; each point takes two draws from xoshiro256**: the first picks a triangle by its share of the area and, rescaled within that share, is r₁; the second is r₂; P = (1 − √r₁)A + √r₁(1 − r₂)B + √r₁r₂C (Osada et al. 2002 §4.2); then back to latitude and longitude",
    accuracy: "Every point is inside the polygon as drawn on the plane; uniform by plane area, which differs from ground area by the zone's scale (up to about 0.3% inside a zone)",
    when_to_use: "Use this to pick sample sites, plots, or check points at random inside a field, a stand, or a study area. Enter the boundary, how many points, and a seed; the same seed always gives the same points, so the layout can be shared and checked.",
    limitations: "Points are uniform over the area on the UTM plane, not over the ground surface, a difference far below field accuracy within one zone. The polygon may span at most 500 km and 2,000 corners. It does not keep points apart; for an even spread use a systematic design.",
    references: &[OSADA, VIGNA, EPA_G5S],
    examples: &[Example {
        id: "primary",
        title: "Ten points in a field with a pond, seed 42",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.996},{"lat":40.003,"lon":-104.996},{"lat":40.003,"lon":-105.0},{"lat":40.001,"lon":-104.999,"ring":1},{"lat":40.002,"lon":-104.999,"ring":1},{"lat":40.002,"lon":-104.998,"ring":1},{"lat":40.001,"lon":-104.998,"ring":1}],"count":10,"seed":"42"}"#,
        source: "A 340 m × 333 m field with a pond as a hole",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("area", "area")],
        },
        Layer {
            kind: "point",
            map: &[("points", "points")],
        },
    ],
    related: &[
        Related {
            id: "geometry.area.polygon",
            reason: "parent",
        },
        Related {
            id: "geometry.predicate.point-in-polygon",
            reason: "next",
        },
        Related {
            id: "geometry.mesh.delaunay",
            reason: "alternative",
        },
    ],
    sentence: "{count} random points, one for every {area_per_point}, from seed {seed}.",
    limits: &[("batchRows", 1)],
    run: run_random_points,
    ..ToolDef::BLANK
};

fn run_random_points(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let plane = Plane::read(ctx)?;
    let n = ctx.number("count")?.expect("required");
    if n.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/count",
            "The number of points is a whole number.",
        ));
    }
    let n = n as usize;
    let seed = seed(ctx)?;
    let tris = triangulate(&plane.rings).ok_or_else(|| {
        ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The polygon could not be cut into triangles; check that the holes lie inside the outline and do not touch it.",
        )
        .at("/polygon")
    })?;
    let n_tris = tris.len();
    let pieces = Pieces::new(tris);
    let mut g = Xoshiro256::new(seed);
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        let (lat, lon) = plane.to_geo(pieces.draw(&mut g));
        points.push(Json::obj([
            ("lat", q(lat, "deg", QT::Angle).to_json()),
            ("lon", q(lon, "deg", QT::Angle).to_json()),
        ]));
    }
    let area = plane.area();
    if ctx.explaining() {
        ctx.step(
            "Triangles",
            "ear clipping on the UTM plane",
            format!(
                "{} corners, {} holes",
                plane.rings.iter().map(Vec::len).sum::<usize>(),
                plane.rings.len() - 1
            ),
            format!("{n_tris} triangles"),
        );
        ctx.step(
            "Points",
            "P = (1 − √r₁)A + √r₁(1 − r₂)B + √r₁r₂C",
            format!("2 draws per point from xoshiro256**, seed {seed}"),
            format!("{n} points"),
        );
        let fmt = ctx.options.format;
        let m2 = |v: f64| gp_base::display::quantity(v, "m2", Precision::Significant(6), fmt);
        ctx.step(
            "Area per point",
            "a = A / N",
            format!("{} / {n}", m2(area)),
            m2(area / n as f64),
        );
    }
    Ok(Json::obj([
        ("points", Json::Arr(points)),
        ("count", Json::Num(n as f64)),
        (
            "area_per_point",
            ctx.out("area_per_point", q(area / n as f64, "m2", QT::Area)),
        ),
        ("area", ctx.out("area", q(area, "m2", QT::Area))),
        ("seed", Json::str(seed.to_string())),
        ("crs", Json::str(plane.crs())),
        (
            "design",
            Json::str("Simple random sampling (EPA QA/G-5S ch. 5)"),
        ),
    ]))
}

const MAX_CELLS: f64 = 50_000.0;

fn too_many(
    at: &str,
    what: &str,
    n: f64,
    limit: f64,
    fmt: gp_base::parse::NumberFormat,
) -> ToolError {
    let show = |v: f64| gp_base::display::number(v.round(), Precision::Decimals(0), fmt);
    ToolError::new(
        ErrorCode::LimitExceeded,
        format!("These inputs make about {} {what}, over the limit of {}. Use a wider spacing or a smaller area.", show(n), show(limit)),
    )
    .at(at)
}

fn point_row(plane: &Plane, p: P) -> Json {
    let (lat, lon) = plane.to_geo(p);
    Json::obj([
        ("lat", q(lat, "deg", QT::Angle).to_json()),
        ("lon", q(lon, "deg", QT::Angle).to_json()),
    ])
}

pub static SYSTEMATIC_POINTS: ToolDef = ToolDef {
    id: "geometry.sampling.systematic-points",
    title: "Systematic grid sample points in a polygon",
    summary: "Sample points on a square or triangular grid of a set spacing across a polygon, with a random start from a seed or a start at the corner, and the area each point stands for.",
    aliases: &[
        "systematic sampling grid",
        "grid sampling points",
        "regular points in polygon",
    ],
    keywords: &[
        "systematic",
        "grid",
        "sample",
        "sampling",
        "points",
        "triangular",
        "spacing",
    ],
    inputs: &[
        POLYGON,
        Field::new(
            "spacing",
            "Spacing",
            "Ground distance between neighbors, like 50 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "pattern",
            "Pattern",
            "Square rows and columns, or triangular (offset rows, six neighbors each)",
            Kind::Choice(&["square", "triangular"]),
        )
        .core(),
        Field::new(
            "start",
            "Start",
            "A random start from the seed, or the grid's first point at the corner",
            Kind::Choice(&["random", "corner"]),
        ),
        Field::new(
            "rotation",
            "Rotation",
            "Grid turned clockwise from north, like 30°",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
        SEED,
    ],
    outputs: &[
        Field::new(
            "points",
            "Points",
            "Row by row, south to north, west to east in the grid's frame",
            Kind::List {
                items: OUT_POINT,
                min: 0,
                max: 100_000,
            },
        ),
        Field::new(
            "count",
            "Points placed",
            "",
            Kind::Number {
                min: 0.0,
                max: MAX_POINTS,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "area_per_point",
            "Area per point",
            "One grid cell: d² for square, d²·√3/2 for triangular",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "offset",
            "Start offset",
            "Of the first grid point from the corner, east and north in the grid's frame",
            Kind::Text { max_len: 60 },
        ),
        Field::new("seed", "Seed", "", Kind::Text { max_len: 20 }),
        Field::new(
            "crs",
            "Plane",
            "Where the grid was laid out",
            Kind::Text { max_len: 40 },
        ),
        Field::new("design", "Design", "", Kind::Text { max_len: 120 }),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
        ErrorCode::LimitExceeded,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "A lattice on the UTM plane from the lower-left corner of the polygon's bounding box (in the grid's turned frame): square, or triangular with rows d·√3/2 apart and every other row shifted d/2; spacing scaled by the zone's point scale at the polygon's center so neighbors are d apart on the ground; a random start moves the lattice by (r₁·d, r₂·row spacing) from two draws (EPA QA/G-5S ch. 7); points kept inside the outline and outside holes",
    accuracy: "Neighbors are the spacing apart on the ground to within the change in UTM scale across the polygon (under 1 mm over 1 km near the center)",
    when_to_use: "Use this to lay out sample points evenly over a site: soil cores, plots, or check points on a grid. A triangular grid covers ground most evenly; a random start keeps the design a probability sample. The same seed gives the same grid.",
    limitations: "A grid can line up with patterns in the ground, such as crop rows or drains, and miss or over-sample them; rotate it or use a random design if that is a risk. At most 100,000 points; the polygon may span 500 km and 2,000 corners.",
    references: &[EPA_G5S_SYSTEMATIC, VIGNA],
    examples: &[Example {
        id: "primary",
        title: "A 50 m triangular grid with a random start, seed 42",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.996},{"lat":40.003,"lon":-104.996},{"lat":40.003,"lon":-105.0}],"spacing":"50 m","pattern":"triangular","seed":"42"}"#,
        source: "A 340 m × 333 m field",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("area", "area_per_point")],
        },
        Layer {
            kind: "point",
            map: &[("points", "points")],
        },
    ],
    related: &[
        Related {
            id: "geometry.sampling.random-points",
            reason: "alternative",
        },
        Related {
            id: "geometry.sampling.stratified-points",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "parent",
        },
    ],
    sentence: "{count} points on a grid, one for every {area_per_point}.",
    limits: &[("batchRows", 1)],
    run: run_systematic,
    ..ToolDef::BLANK
};

pub(crate) const EPA_G5S_SYSTEMATIC: Reference = Reference {
    locator: "Chapter 7, systematic/grid sampling (p. 63)",
    ..EPA_G5S
};
pub(crate) const EPA_G5S_STRATIFIED: Reference = Reference {
    locator: "Chapter 6, stratified sampling (p. 51)",
    ..EPA_G5S
};

/// Rotation about `o` by `t` radians clockwise (a bearing turn) and back.
fn turn(p: P, o: P, t: f64) -> P {
    let (s, c) = (libm::sin(t), libm::cos(t));
    let (x, y) = (p.0 - o.0, p.1 - o.1);
    (o.0 + x * c + y * s, o.1 - x * s + y * c)
}

fn run_systematic(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let plane = Plane::read(ctx)?;
    let m = gp_base::units::by_symbol(QT::Length, "m").expect("m");
    let d = ctx.req_quantity("spacing")?.to(m);
    if d.is_nan() || d <= 0.0 {
        return Err(ToolError::invalid(
            "/spacing",
            "The spacing must be more than zero.",
        ));
    }
    let tri = ctx.choice("pattern")? == Some("triangular");
    let random = ctx.choice("start")? != Some("corner");
    let deg = gp_base::units::by_symbol(QT::Angle, "deg").expect("deg");
    let rot = ctx.quantity("rotation")?.map_or(0.0, |a| a.to(deg));
    let seed = seed(ctx)?;
    let fmt = ctx.options.format;
    // Plane spacing: ground meters times the point scale at the center.
    let dp = d * plane.scale;
    let row = if tri { dp * libm::sqrt(3.0) / 2.0 } else { dp };
    let cell = dp * row;
    let expected = plane.plane_area() / cell;
    if expected > MAX_POINTS {
        return Err(too_many("/spacing", "points", expected, MAX_POINTS, fmt));
    }
    // The grid's frame: the polygon turned back by the rotation about the
    // bounding box's lower-left corner.
    let xs = plane.rings[0].iter().map(|p| p.0);
    let ys = plane.rings[0].iter().map(|p| p.1);
    let o = (
        xs.fold(f64::INFINITY, f64::min),
        ys.fold(f64::INFINITY, f64::min),
    );
    let t = rot.to_radians();
    let framed: Vec<P> = plane.rings[0].iter().map(|&p| turn(p, o, -t)).collect();
    let fx0 = framed.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let fy0 = framed.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let fx1 = framed.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let fy1 = framed.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let (ux, uy) = if random {
        let mut g = Xoshiro256::new(seed);
        (g.next_f64() * dp, g.next_f64() * row)
    } else {
        (0.0, 0.0)
    };
    let (cols, rows) = (
        ((fx1 - fx0) / dp).ceil() + 2.0,
        ((fy1 - fy0) / row).ceil() + 2.0,
    );
    if cols * rows > 100.0 * MAX_POINTS {
        return Err(too_many(
            "/spacing",
            "grid positions over the polygon's bounding box",
            cols * rows,
            100.0 * MAX_POINTS,
            fmt,
        ));
    }
    let mut points = vec![];
    for j in 0..rows as usize {
        let y = fy0 + uy + j as f64 * row;
        let shift = if tri && j % 2 == 1 { dp / 2.0 } else { 0.0 };
        for i in 0..cols as usize {
            let x = fx0 + ux + shift + i as f64 * dp
                - if shift > 0.0 && ux + shift >= dp {
                    dp
                } else {
                    0.0
                };
            let p = turn((x, y), o, t);
            if plane.inside(p) {
                points.push(point_row(&plane, p));
            }
        }
    }
    let n = points.len();
    if n == 0 {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "No grid point falls inside the polygon at this spacing; use a smaller spacing.",
        )
        .at("/spacing"));
    }
    let ground_cell = d * d * if tri { libm::sqrt(3.0) / 2.0 } else { 1.0 };
    let n2 = |v: f64| gp_base::display::number(v / plane.scale, Precision::Decimals(2), fmt);
    if ctx.explaining() {
        ctx.step(
            "Area per point",
            if tri { "a = d²·√3/2" } else { "a = d²" },
            format!(
                "d = {} m",
                gp_base::display::number(d, Precision::Significant(6), fmt)
            ),
            gp_base::display::quantity(ground_cell, "m2", Precision::Significant(6), fmt),
        );
        ctx.step(
            "Points inside",
            if tri {
                "rows d·√3/2 apart, every other row shifted d/2"
            } else {
                "rows and columns d apart"
            },
            format!(
                "d = {} m on the ground, {} m on the plane",
                gp_base::display::number(d, Precision::Significant(6), fmt),
                gp_base::display::number(dp, Precision::Significant(9), fmt)
            ),
            format!("{n}"),
        );
    }
    Ok(Json::obj([
        ("points", Json::Arr(points)),
        ("count", Json::Num(n as f64)),
        (
            "area_per_point",
            ctx.out("area_per_point", q(ground_cell, "m2", QT::Area)),
        ),
        (
            "offset",
            Json::str(format!("{} m east, {} m north", n2(ux), n2(uy))),
        ),
        (
            "seed",
            Json::str(if random {
                seed.to_string()
            } else {
                "none (corner start)".to_owned()
            }),
        ),
        ("crs", Json::str(plane.crs())),
        (
            "design",
            Json::str(format!(
                "Systematic {} grid (EPA QA/G-5S ch. 7)",
                if tri { "triangular" } else { "square" }
            )),
        ),
    ]))
}

/// The part of convex polygon `poly` inside the axis-aligned box.
fn clip_box(poly: &[P], x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<P> {
    let mut out = poly.to_vec();
    type Side = fn(P, f64) -> f64;
    let edges: [(Side, f64); 4] = [
        (|p, v| p.0 - v, x0),
        (|p, v| v - p.0, x1),
        (|p, v| p.1 - v, y0),
        (|p, v| v - p.1, y1),
    ];
    for (side, v) in edges {
        let inp = out;
        out = Vec::with_capacity(inp.len() + 2);
        for i in 0..inp.len() {
            let (a, b) = (inp[i], inp[(i + 1) % inp.len()]);
            let (sa, sb) = (side(a, v), side(b, v));
            if sa >= 0.0 {
                out.push(a);
            }
            if (sa >= 0.0) != (sb >= 0.0) {
                let f = sa / (sa - sb);
                out.push((a.0 + f * (b.0 - a.0), a.1 + f * (b.1 - a.1)));
            }
        }
        if out.len() < 3 {
            return vec![];
        }
    }
    out
}

const OUT_STRAT_POINT: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "stratum",
        "Stratum",
        "Row and column of its cell, from the south-west",
        Kind::Text { max_len: 16 },
    ),
];

pub static STRATIFIED_POINTS: ToolDef = ToolDef {
    id: "geometry.sampling.stratified-points",
    title: "Stratified random sample points in a polygon",
    summary: "A stratified random sample: the polygon cut into square cells of a set size, and a set number of random points placed in each cell's part of the polygon, reproducible from a seed.",
    aliases: &[
        "stratified random sampling",
        "random points per grid cell",
        "stratified sample grid",
    ],
    keywords: &[
        "stratified",
        "strata",
        "random",
        "sample",
        "sampling",
        "points",
        "cells",
    ],
    inputs: &[
        POLYGON,
        Field::new(
            "cell",
            "Cell size",
            "Side of each square stratum on the ground, like 100 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "per_stratum",
            "Points per stratum",
            "Random points in each cell, like 2",
            Kind::Number {
                min: 1.0,
                max: 1_000.0,
            },
        )
        .core(),
        SEED.core(),
    ],
    outputs: &[
        Field::new(
            "points",
            "Points",
            "Stratum by stratum, rows south to north, cells west to east",
            Kind::List {
                items: OUT_STRAT_POINT,
                min: 0,
                max: 100_000,
            },
        ),
        Field::new(
            "count",
            "Points placed",
            "",
            Kind::Number {
                min: 0.0,
                max: MAX_POINTS,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "strata",
            "Strata sampled",
            "Cells with 1 m² or more of the polygon",
            Kind::Number {
                min: 0.0,
                max: MAX_CELLS,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "skipped",
            "Strata skipped",
            "Cells holding less than 1 m² of the polygon",
            Kind::Text { max_len: 2_000 },
        ),
        Field::new("seed", "Seed", "", Kind::Text { max_len: 20 }),
        Field::new(
            "crs",
            "Plane",
            "Where the cells were laid out",
            Kind::Text { max_len: 40 },
        ),
        Field::new("design", "Design", "", Kind::Text { max_len: 120 }),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
        ErrorCode::LimitExceeded,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Square cells from the lower-left corner of the polygon's bounding box on the UTM plane, side scaled by the zone's point scale at the center; each cell's part of the polygon is the polygon's ear-clipped triangles clipped to the cell; strata taken row by row, south to north and west to east; in each, k points by the random-points method, two draws each, from one xoshiro256** stream (EPA QA/G-5S ch. 6)",
    accuracy: "Every point is inside its cell and the polygon as drawn on the plane; uniform within each stratum by plane area",
    when_to_use: "Use this when the sample must cover the whole site but stay random: each grid cell gets its own random points, so no part of the area is left out. Enter the boundary, the cell size, and how many points per cell.",
    limitations: "Each cell gets the same number of points however much of the polygon it holds, so a sliver cell at the edge is sampled as heavily as a full one; weigh results by stratum area. Cells with less than 1 m² of the polygon are skipped and listed. At most 50,000 cells and 100,000 points.",
    references: &[EPA_G5S_STRATIFIED, OSADA, VIGNA],
    examples: &[Example {
        id: "primary",
        title: "Two points in each 100 m cell of a field, seed 42",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.996},{"lat":40.003,"lon":-104.996},{"lat":40.003,"lon":-105.0}],"cell":"100 m","per_stratum":2,"seed":"42"}"#,
        source: "A 340 m × 333 m field in 16 cells",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("area", "strata")],
        },
        Layer {
            kind: "point",
            map: &[("points", "points")],
        },
    ],
    related: &[
        Related {
            id: "geometry.sampling.random-points",
            reason: "alternative",
        },
        Related {
            id: "geometry.sampling.systematic-points",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "parent",
        },
    ],
    sentence: "{count} points in {strata} cells, from seed {seed}.",
    limits: &[("batchRows", 1)],
    run: run_stratified,
    ..ToolDef::BLANK
};

fn run_stratified(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let plane = Plane::read(ctx)?;
    let m = gp_base::units::by_symbol(QT::Length, "m").expect("m");
    let s = ctx.req_quantity("cell")?.to(m);
    if s.is_nan() || s <= 0.0 {
        return Err(ToolError::invalid(
            "/cell",
            "The cell size must be more than zero.",
        ));
    }
    let k = ctx.number("per_stratum")?.unwrap_or(1.0);
    if k.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/per_stratum",
            "Points per stratum is a whole number.",
        ));
    }
    let k = k as usize;
    let seed = seed(ctx)?;
    let fmt = ctx.options.format;
    let sp = s * plane.scale;
    let xs = plane.rings[0].iter().map(|p| p.0);
    let ys = plane.rings[0].iter().map(|p| p.1);
    let (x0, y0) = (
        xs.clone().fold(f64::INFINITY, f64::min),
        ys.clone().fold(f64::INFINITY, f64::min),
    );
    let (x1, y1) = (
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.fold(f64::NEG_INFINITY, f64::max),
    );
    let (cols, rows) = (
        ((x1 - x0) / sp).ceil().max(1.0),
        ((y1 - y0) / sp).ceil().max(1.0),
    );
    if cols * rows > MAX_CELLS {
        return Err(too_many("/cell", "cells", cols * rows, MAX_CELLS, fmt));
    }
    let estimate = (plane.plane_area() / (sp * sp)).ceil().min(cols * rows) * k as f64;
    if estimate > MAX_POINTS {
        return Err(too_many(
            "/per_stratum",
            "points",
            estimate,
            MAX_POINTS,
            fmt,
        ));
    }
    let tris = triangulate(&plane.rings).ok_or_else(|| {
        ToolError::new(ErrorCode::DegenerateGeometry, "The polygon could not be cut into triangles; check that the holes lie inside the outline and do not touch it.").at("/polygon")
    })?;
    let (cols, rows) = (cols as usize, rows as usize);
    // Each triangle listed under every cell its bounding box reaches.
    let mut bucket: Vec<Vec<usize>> = vec![vec![]; cols * rows];
    for (ti, t) in tris.iter().enumerate() {
        let span =
            |v: f64, lo: f64, n: usize| (((v - lo) / sp).floor().max(0.0) as usize).min(n - 1);
        let (ca, cb) = (
            span(
                t.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
                x0,
                cols,
            ),
            span(
                t.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                x0,
                cols,
            ),
        );
        let (ra, rb) = (
            span(
                t.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                y0,
                rows,
            ),
            span(
                t.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
                y0,
                rows,
            ),
        );
        for r in ra..=rb {
            for c in ca..=cb {
                bucket[r * cols + c].push(ti);
            }
        }
    }
    let one_m2 = plane.scale * plane.scale;
    let mut g = Xoshiro256::new(seed);
    let (mut points, mut skipped, mut strata) = (vec![], vec![], 0usize);
    for r in 0..rows {
        for c in 0..cols {
            let (bx0, by0) = (x0 + c as f64 * sp, y0 + r as f64 * sp);
            let mut parts: Vec<[P; 3]> = vec![];
            for &ti in &bucket[r * cols + c] {
                let piece = clip_box(&tris[ti], bx0, by0, bx0 + sp, by0 + sp);
                for i in 1..piece.len().saturating_sub(1) {
                    if orient(piece[0], piece[i], piece[i + 1]) > 0.0 {
                        parts.push([piece[0], piece[i], piece[i + 1]]);
                    }
                }
            }
            let label = format!("r{}c{}", r + 1, c + 1);
            let pieces = Pieces::new(parts);
            if pieces.area() < one_m2 {
                if pieces.area() > 0.0 {
                    skipped.push(label);
                }
                continue;
            }
            strata += 1;
            for _ in 0..k {
                let (lat, lon) = plane.to_geo(pieces.draw(&mut g));
                points.push(Json::obj([
                    ("lat", q(lat, "deg", QT::Angle).to_json()),
                    ("lon", q(lon, "deg", QT::Angle).to_json()),
                    ("stratum", Json::str(label.clone())),
                ]));
            }
        }
    }
    let n = points.len();
    if ctx.explaining() {
        ctx.step(
            "Strata",
            "square cells from the south-west corner, clipped to the polygon",
            format!(
                "{} m cells, {cols} × {rows}",
                gp_base::display::number(s, Precision::Significant(6), fmt)
            ),
            format!("{strata} sampled, {} skipped", skipped.len()),
        );
        ctx.step(
            "Points",
            "k random points in each stratum, 2 draws each",
            format!("k = {k}, seed {seed}"),
            format!("{n}"),
        );
    }
    Ok(Json::obj([
        ("points", Json::Arr(points)),
        ("count", Json::Num(n as f64)),
        ("strata", Json::Num(strata as f64)),
        (
            "skipped",
            Json::str(if skipped.is_empty() {
                "none".to_owned()
            } else {
                skipped.join(", ")
            }),
        ),
        ("seed", Json::str(seed.to_string())),
        ("crs", Json::str(plane.crs())),
        (
            "design",
            Json::str(format!(
                "Stratified random, {k} per {} m cell (EPA QA/G-5S ch. 6)",
                gp_base::display::number(s, Precision::Significant(6), fmt)
            )),
        ),
    ]))
}
