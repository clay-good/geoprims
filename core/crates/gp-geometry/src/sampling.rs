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
    /// Meridian convergence there: the bearing of grid north from true north, degrees.
    pub convergence: f64,
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
        let plane = Plane::build(geo, true);
        let xs = plane.rings[0].iter().map(|p| p.0);
        let ys = plane.rings[0].iter().map(|p| p.1);
        let w = xs.clone().fold(f64::NEG_INFINITY, f64::max) - xs.fold(f64::INFINITY, f64::min);
        let h = ys.clone().fold(f64::NEG_INFINITY, f64::max) - ys.fold(f64::INFINITY, f64::min);
        if w.max(h) > MAX_EXTENT_M {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                format!("The polygon spans {} km; one flat grid is good to 500 km, past which its distortion grows too large. Split the area.", gp_base::display::number(w.max(h) / 1000.0, Precision::Decimals(0), ctx.options.format)),
            )
            .at("/polygon"));
        }
        Ok(plane)
    }

    /// The plane of the first ring's mean corner (longitudes unwrapped about
    /// its first corner), with every ring carried to it; `polygon` orients
    /// ring 0 counterclockwise and the rest clockwise.
    pub fn build(geo: Vec<Vec<(f64, f64)>>, polygon: bool) -> Plane {
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
        if polygon {
            for (k, r) in rings.iter_mut().enumerate() {
                let ccw = twice_area(r) > 0.0;
                if (k == 0) != ccw {
                    r.reverse();
                }
            }
        }
        let dlon_c = (lon_c - cm + 180.0).rem_euclid(360.0) - 180.0;
        let (_, _, convergence, scale) = tm.forward(lat_c, dlon_c);
        Plane {
            zone,
            north: lat_c >= 0.0,
            cm,
            tm,
            rings,
            geo,
            scale,
            convergence,
        }
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

const MAX_TRANSECTS: f64 = 10_000.0;

pub(crate) const BUCKLAND: Reference = Reference {
    title: "Distance Sampling: Methods and Applications",
    issuer: "Buckland, S. T., Rexstad, E. A., Marques, T. A., and Oedekoven, C. S., Springer",
    year: 2015,
    edition: "1st edition",
    locator: "Survey design: systematic parallel line transects with a random start",
    url: "https://doi.org/10.1007/978-3-319-19219-2",
};

const OUT_TRANSECT: &[Field] = &[
    Field::new(
        "transect",
        "Transect",
        "Numbered from the start side; a line the polygon cuts in two has parts a, b, …",
        Kind::Text { max_len: 12 },
    ),
    Field::new(
        "length",
        "Length",
        "Geodesic, on the ground",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "start_lat",
        "Start latitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "start_lon",
        "Start longitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "end_lat",
        "End latitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "end_lon",
        "End longitude",
        "",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];

const OUT_VERTEX: &[Field] = &[
    Field::new("line", "Line", "", Kind::Number { min: 0.0, max: 1e6 })
        .precision(Precision::Decimals(0)),
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

pub static TRANSECTS: ToolDef = ToolDef {
    id: "geometry.sampling.transects",
    title: "Parallel transects across a polygon",
    summary: "Survey lines at a set spacing and bearing across a polygon, clipped to it, with a random start from a seed, each line's length and the total, and a zigzag route joining them if asked.",
    aliases: &["line transects", "transect layout", "parallel survey lines"],
    keywords: &[
        "transect",
        "transects",
        "lines",
        "parallel",
        "survey",
        "spacing",
        "zigzag",
    ],
    inputs: &[
        POLYGON,
        Field::new(
            "spacing",
            "Spacing",
            "Between neighboring lines on the ground, like 100 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "bearing",
            "Bearing",
            "Direction of the lines from true north, like 90° for east-west lines",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "route",
            "Route",
            "Lines only, or a zigzag that joins each line's end to the next",
            Kind::Choice(&["lines", "zigzag"]),
        ),
        SEED,
    ],
    outputs: &[
        Field::new(
            "count",
            "Transects",
            "Lines that cross the polygon",
            Kind::Number {
                min: 0.0,
                max: MAX_TRANSECTS,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "total_length",
            "Total length",
            "Of every transect, on the ground",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "transects",
            "Transects",
            "From the start side",
            Kind::List {
                items: OUT_TRANSECT,
                min: 0,
                max: 20_000,
            },
        ),
        Field::new(
            "route_length",
            "Zigzag length",
            "Transects plus the joins between them",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3))
        .optional(),
        Field::new(
            "vertices",
            "Lines",
            "Each transect as a line, or the zigzag as one",
            Kind::List {
                items: OUT_VERTEX,
                min: 0,
                max: 100_000,
            },
        ),
        Field::new(
            "offset",
            "Start offset",
            "Of the first line from the polygon's edge",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new("seed", "Seed", "", Kind::Text { max_len: 20 }),
        Field::new(
            "crs",
            "Plane",
            "Where the lines were laid out",
            Kind::Text { max_len: 40 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
        ErrorCode::LimitExceeded,
        ErrorCode::NoSolution,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "On the UTM plane: lines at the grid bearing (true bearing less the meridian convergence at the polygon's center), spaced by the ground spacing times the zone's point scale there, the first at a random fraction of the spacing (one draw) from the polygon's edge on the left of the lines' direction; each line clipped to the polygon by its crossings with every ring; lengths are geodesic; a zigzag joins each part's end to the next part's start, reversing every other line (Buckland et al. 2015)",
    accuracy: "Lines are the spacing apart on the ground at the polygon's center, to within the change in UTM scale across it; lengths are geodesic to a millimeter",
    when_to_use: "Use this to plan line transects for a wildlife count, a vegetation survey, or a walk-over search: lines at an even spacing across the site with a random start, so the design is a probability sample. Choose the zigzag to get one continuous route.",
    limitations: "Lines are straight on the UTM plane, which over a few kilometers differs from a geodesic by far less than a field crew can walk. A line the polygon cuts in two (around a hole or a bay) becomes two parts. At most 10,000 transects.",
    references: &[BUCKLAND, VIGNA],
    examples: &[Example {
        id: "primary",
        title: "East-west lines 50 m apart across a field, seed 42",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.996},{"lat":40.003,"lon":-104.996},{"lat":40.003,"lon":-105.0}],"spacing":"50 m","bearing":"90 deg","seed":"42"}"#,
        source: "A 340 m × 333 m field",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("area", "total_length")],
        },
        Layer {
            kind: "polyline",
            map: &[("lines", "vertices")],
        },
    ],
    related: &[
        Related {
            id: "geometry.sampling.systematic-points",
            reason: "alternative",
        },
        Related {
            id: "geometry.sampling.points-along-line",
            reason: "next",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "parent",
        },
    ],
    sentence: "{count} transects, {total_length} in all.",
    limits: &[("batchRows", 1)],
    run: run_transects,
    ..ToolDef::BLANK
};

fn run_transects(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let plane = Plane::read(ctx)?;
    let m = gp_base::units::by_symbol(QT::Length, "m").expect("m");
    let deg = gp_base::units::by_symbol(QT::Angle, "deg").expect("deg");
    let d = ctx.req_quantity("spacing")?.to(m);
    if d.is_nan() || d <= 0.0 {
        return Err(ToolError::invalid(
            "/spacing",
            "The spacing must be more than zero.",
        ));
    }
    let bearing = ctx.quantity("bearing")?.map_or(0.0, |a| a.to(deg));
    let zigzag = ctx.choice("route")? == Some("zigzag");
    let seed = seed(ctx)?;
    let fmt = ctx.options.format;
    let dp = d * plane.scale;
    let az = (bearing - plane.convergence).to_radians();
    let dir = (libm::sin(az), libm::cos(az));
    // Across the lines, pointing right of their direction; lines sweep from
    // the polygon's left edge.
    let across = (dir.1, -dir.0);
    let s = |p: P| p.0 * across.0 + p.1 * across.1;
    let lo = plane.rings[0]
        .iter()
        .map(|&p| s(p))
        .fold(f64::INFINITY, f64::min);
    let hi = plane.rings[0]
        .iter()
        .map(|&p| s(p))
        .fold(f64::NEG_INFINITY, f64::max);
    let lines = ((hi - lo) / dp).ceil();
    if lines > MAX_TRANSECTS {
        return Err(too_many("/spacing", "transects", lines, MAX_TRANSECTS, fmt));
    }
    let mut g = Xoshiro256::new(seed);
    let u = g.next_f64() * dp;
    let along = |p: P| p.0 * dir.0 + p.1 * dir.1;
    let geod = Geodesic::wgs84();
    let mut rows = vec![];
    let mut verts = vec![];
    let mut route: Vec<P> = vec![];
    let (mut total, mut number) = (0.0, 0usize);
    let mut k = 0usize;
    loop {
        let c = lo + u + k as f64 * dp;
        if c >= hi {
            break;
        }
        k += 1;
        // Crossings of the line s = c with every ring, ordered along it.
        let mut hits: Vec<f64> = vec![];
        for r in &plane.rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                let (sa, sb) = (s(a) - c, s(b) - c);
                if (sa < 0.0) != (sb < 0.0) {
                    let f = sa / (sa - sb);
                    hits.push(along((a.0 + f * (b.0 - a.0), a.1 + f * (b.1 - a.1))));
                }
            }
        }
        hits.sort_by(f64::total_cmp);
        if hits.len() < 2 {
            continue;
        }
        number += 1;
        let at = |t: f64| (dir.0 * t + across.0 * c, dir.1 * t + across.1 * c);
        let mut parts: Vec<(P, P)> = hits
            .as_chunks::<2>()
            .0
            .iter()
            .map(|h| (at(h[0]), at(h[1])))
            .collect();
        if zigzag && number % 2 == 0 {
            parts.reverse();
            for p in &mut parts {
                *p = (p.1, p.0);
            }
        }
        let many = parts.len() > 1;
        for (j, &(a, b)) in parts.iter().enumerate() {
            let (la, lb) = (plane.to_geo(a), plane.to_geo(b));
            let len: f64 =
                geographiclib_rs::InverseGeodesic::inverse(&geod, la.0, la.1, lb.0, lb.1);
            total += len;
            let name = if many {
                format!("{number}{}", (b'a' + (j as u8 % 26)) as char)
            } else {
                number.to_string()
            };
            rows.push(Json::obj([
                ("transect", Json::str(name)),
                ("length", q(len, "m", QT::Length).to_json()),
                ("start_lat", q(la.0, "deg", QT::Angle).to_json()),
                ("start_lon", q(la.1, "deg", QT::Angle).to_json()),
                ("end_lat", q(lb.0, "deg", QT::Angle).to_json()),
                ("end_lon", q(lb.1, "deg", QT::Angle).to_json()),
            ]));
            if zigzag {
                route.push(a);
                route.push(b);
            } else {
                let line = rows.len() as f64;
                for (lat, lon) in [la, lb] {
                    verts.push(Json::obj([
                        ("line", Json::Num(line)),
                        ("lat", q(lat, "deg", QT::Angle).to_json()),
                        ("lon", q(lon, "deg", QT::Angle).to_json()),
                    ]));
                }
            }
        }
    }
    if number == 0 {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "No line crosses the polygon at this spacing and start; use a smaller spacing.",
        )
        .at("/spacing"));
    }
    let mut route_len = None;
    if zigzag {
        let mut len = 0.0;
        for w in route.windows(2) {
            let (a, b) = (plane.to_geo(w[0]), plane.to_geo(w[1]));
            let l: f64 = geographiclib_rs::InverseGeodesic::inverse(&geod, a.0, a.1, b.0, b.1);
            len += l;
        }
        route_len = Some(len);
        for p in &route {
            let (lat, lon) = plane.to_geo(*p);
            verts.push(Json::obj([
                ("line", Json::Num(1.0)),
                ("lat", q(lat, "deg", QT::Angle).to_json()),
                ("lon", q(lon, "deg", QT::Angle).to_json()),
            ]));
        }
    }
    if ctx.explaining() {
        ctx.step(
            "Lines",
            "parallel at the grid bearing, the spacing apart, from a random start",
            format!(
                "{} m apart at {}° true, start {} m in",
                gp_base::display::number(d, Precision::Significant(6), fmt),
                gp_base::display::number(bearing, Precision::Significant(6), fmt),
                gp_base::display::number(u / plane.scale, Precision::Decimals(3), fmt)
            ),
            format!("{} parts on {number} lines", rows.len()),
        );
        ctx.step(
            "Transects",
            "lines that cross the polygon",
            format!(
                "{} m in all",
                gp_base::display::number(total, Precision::Decimals(3), fmt)
            ),
            format!("{number}"),
        );
    }
    let mut out = vec![
        ("count", Json::Num(number as f64)),
        (
            "total_length",
            ctx.out("total_length", q(total, "m", QT::Length)),
        ),
        ("transects", Json::Arr(rows)),
    ];
    if let Some(l) = route_len {
        out.push((
            "route_length",
            ctx.out("route_length", q(l, "m", QT::Length)),
        ));
    }
    out.push(("vertices", Json::Arr(verts)));
    out.push((
        "offset",
        ctx.out("offset", q(u / plane.scale, "m", QT::Length)),
    ));
    out.push(("seed", Json::str(seed.to_string())));
    out.push(("crs", Json::str(plane.crs())));
    Ok(Json::obj(out))
}

const LINE_VERTEX: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.0",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -105.0",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
];

const OUT_ALONG: &[Field] = &[
    Field::new(
        "distance",
        "Distance along",
        "On the plane, from the line's start",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
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

pub(crate) const QGIS_ALONG: Reference = Reference {
    title: "QGIS Processing algorithm: Points along geometry",
    issuer: "QGIS project (vendor documentation)",
    year: 2025,
    edition: "QGIS 3.44 user manual",
    locator: "Vector geometry: Points along geometry (distance, start offset, end offset)",
    url: "https://docs.qgis.org/3.44/en/docs/user_manual/processing_algs/qgis/vectorgeometry.html#points-along-geometry",
};

pub static POINTS_ALONG_LINE: ToolDef = ToolDef {
    id: "geometry.sampling.points-along-line",
    title: "Points every set distance along a line",
    summary: "Points at a fixed interval along a line, from a start offset, measured on the UTM plane as GIS tools do, with the line's plane and geodesic lengths side by side.",
    aliases: &[
        "points along line",
        "stations along a line",
        "points along geometry",
    ],
    keywords: &[
        "points", "along", "line", "interval", "stations", "sampling", "chainage",
    ],
    inputs: &[
        Field::new(
            "line",
            "Line",
            "Corners in order, at least 2",
            Kind::List {
                items: LINE_VERTEX,
                min: 2,
                max: 10_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "interval",
            "Interval",
            "Between points, like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "start",
            "Start offset",
            "Distance to the first point, like 0 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "count",
            "Points",
            "",
            Kind::Number {
                min: 0.0,
                max: MAX_POINTS,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "points",
            "Points",
            "From the start",
            Kind::List {
                items: OUT_ALONG,
                min: 0,
                max: 100_000,
            },
        ),
        Field::new(
            "plane_length",
            "Plane length",
            "On the UTM plane, which the interval is measured on",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "geodesic_length",
            "Ground length",
            "Geodesic, corner to corner",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new("crs", "Plane", "", Kind::Text { max_len: 40 }),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "The line carried to its UTM zone's plane; points at start + i × interval of plane length, kept while within the line (QGIS Points along geometry); geodesic length from Karney's inverse on each segment",
    accuracy: "Exact along the plane line; the plane and ground lengths differ by the zone's scale, shown side by side",
    when_to_use: "Use this to set stations or sample points every so many meters along a route, a stream, a fence, or a transect, the way GIS tools do it. The last point is kept only if it falls on the line.",
    limitations: "Distances are measured on the UTM plane, as GIS tools do, so they run short or long of the ground by the zone's scale (up to about 0.1%); the ground length is shown beside it. At most 100,000 points; the line may span 500 km.",
    references: &[QGIS_ALONG, KARNEY_GEOD],
    examples: &[Example {
        id: "primary",
        title: "Every 30 m along a 100 m line",
        input: r#"{"line":[{"lat":40.0,"lon":-105.0},{"lat":40.0009,"lon":-105.0}],"interval":"30 m"}"#,
        source: "Points at 0, 30, 60, and 90 m; none at 120 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("points", "points")],
    }],
    related: &[
        Related {
            id: "geometry.shape.densify",
            reason: "alternative",
        },
        Related {
            id: "geometry.sampling.transects",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.waypoints",
            reason: "alternative",
        },
    ],
    sentence: "{count} points along a line of {plane_length} on the plane ({geodesic_length} on the ground).",
    limits: &[("batchRows", 100)],
    run: run_points_along,
    ..ToolDef::BLANK
};

pub(crate) const KARNEY_GEOD: Reference = crate::KARNEY;

fn run_points_along(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = gp_base::units::by_symbol(QT::Angle, "deg").expect("deg");
    let m = gp_base::units::by_symbol(QT::Length, "m").expect("m");
    let mut line = vec![];
    for (i, r) in ctx.rows("line")?.iter().enumerate() {
        let lat = ctx
            .row_quantity("line", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("line", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/line/{i}/lat")));
        }
        line.push((lat, lon));
    }
    let step = ctx.req_quantity("interval")?.to(m);
    if step.is_nan() || step <= 0.0 {
        return Err(ToolError::invalid(
            "/interval",
            "The interval must be more than zero.",
        ));
    }
    let start = ctx.quantity("start")?.map_or(0.0, |q| q.to(m));
    if start.is_nan() || start < 0.0 {
        return Err(ToolError::invalid(
            "/start",
            "The start offset cannot be negative.",
        ));
    }
    let fmt = ctx.options.format;
    let plane = Plane::build(vec![line.clone()], false);
    let pts = &plane.rings[0];
    let xs = pts.iter().map(|p| p.0);
    let ys = pts.iter().map(|p| p.1);
    let ext = (xs.clone().fold(f64::NEG_INFINITY, f64::max) - xs.fold(f64::INFINITY, f64::min))
        .max(ys.clone().fold(f64::NEG_INFINITY, f64::max) - ys.fold(f64::INFINITY, f64::min));
    if ext > MAX_EXTENT_M {
        return Err(ToolError::new(ErrorCode::LimitExceeded, "The line spans more than 500 km, past which one flat plane distorts too much. Split it.").at("/line"));
    }
    let mut cum = vec![0.0];
    for w in pts.windows(2) {
        cum.push(cum.last().unwrap() + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1));
    }
    let length = *cum.last().unwrap();
    let count = if start > length {
        0.0
    } else {
        ((length - start) / step).floor() + 1.0
    };
    if count > MAX_POINTS {
        return Err(too_many("/interval", "points", count, MAX_POINTS, fmt));
    }
    let geod = Geodesic::wgs84();
    let ground: f64 = line
        .windows(2)
        .map(|w| -> f64 {
            geographiclib_rs::InverseGeodesic::inverse(&geod, w[0].0, w[0].1, w[1].0, w[1].1)
        })
        .sum();
    let mut out = Vec::with_capacity(count as usize);
    let mut seg = 0;
    for i in 0..count as usize {
        let t = start + i as f64 * step;
        while seg + 1 < pts.len() - 1 && cum[seg + 1] < t {
            seg += 1;
        }
        let l = cum[seg + 1] - cum[seg];
        let f = if l > 0.0 {
            ((t - cum[seg]) / l).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (a, b) = (pts[seg], pts[seg + 1]);
        let (lat, lon) = plane.to_geo((a.0 + f * (b.0 - a.0), a.1 + f * (b.1 - a.1)));
        out.push(Json::obj([
            ("distance", q(t, "m", QT::Length).to_json()),
            ("lat", q(lat, "deg", QT::Angle).to_json()),
            ("lon", q(lon, "deg", QT::Angle).to_json()),
        ]));
    }
    if ctx.explaining() {
        let n3 = |v: f64| gp_base::display::number(v, Precision::Decimals(3), fmt);
        ctx.step(
            "Line length",
            "sum of segment lengths on the UTM plane",
            format!(
                "{} segments on {}; {} m on the ground",
                pts.len() - 1,
                plane.crs(),
                n3(ground)
            ),
            format!("{} m", n3(length)),
        );
        ctx.step(
            "Points",
            "at start + i × interval while within the line",
            format!(
                "{} m line on the plane, every {} m from {} m",
                n3(length),
                n3(step),
                n3(start)
            ),
            format!("{count}"),
        );
    }
    Ok(Json::obj([
        ("count", Json::Num(count)),
        ("points", Json::Arr(out)),
        (
            "plane_length",
            ctx.out("plane_length", q(length, "m", QT::Length)),
        ),
        (
            "geodesic_length",
            ctx.out("geodesic_length", q(ground, "m", QT::Length)),
        ),
        ("crs", Json::str(plane.crs())),
    ]))
}
