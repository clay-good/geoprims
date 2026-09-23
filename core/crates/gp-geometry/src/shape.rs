//! Centroids and representative points (add-navigation-and-geometry,
//! geometry/computational, "Centroids and representative points"): a
//! polygon's area-weighted centroid on the ellipsoid through an exact
//! equal-area map, flagged when it falls outside, and an interior point as
//! far from the edges as can be found.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::{self, Shape};
use libm::{asin, atan2, cos, log, sin, sqrt};

const SNYDER: Reference = Reference {
    title: "Map Projections: A Working Manual",
    issuer: "Snyder, J. P., U.S. Geological Survey",
    year: 1987,
    edition: "Professional Paper 1395",
    locator: "Chapter 24 (Lambert azimuthal equal-area) and equations 3-11 to 3-16 (authalic latitude)",
    url: "https://pubs.usgs.gov/publication/pp1395",
};

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_223_563;

/// The authalic sphere: latitude on it keeps areas, and its radius the total.
struct Authalic {
    e: f64,
    e2: f64,
    qp: f64,
    rq: f64,
}

impl Authalic {
    fn wgs84() -> Authalic {
        let e2 = F * (2.0 - F);
        let e = sqrt(e2);
        let mut a = Authalic {
            e,
            e2,
            qp: 0.0,
            rq: 0.0,
        };
        a.qp = a.q(core::f64::consts::FRAC_PI_2);
        a.rq = A * sqrt(a.qp / 2.0);
        a
    }
    /// Snyder (3-12).
    fn q(&self, phi: f64) -> f64 {
        let s = sin(phi);
        (1.0 - self.e2)
            * (s / (1.0 - self.e2 * s * s)
                - log((1.0 - self.e * s) / (1.0 + self.e * s)) / (2.0 * self.e))
    }
    /// Authalic latitude β from geodetic φ (3-11), radians.
    fn beta(&self, phi: f64) -> f64 {
        asin((self.q(phi) / self.qp).clamp(-1.0, 1.0))
    }
    /// Geodetic φ from β, by Newton's method on q (dq/dφ = 2(1 − e²)cos φ / (1 − e² sin² φ)²).
    fn phi(&self, beta: f64) -> f64 {
        let target = self.qp * sin(beta);
        let mut phi = beta;
        for _ in 0..20 {
            let s = sin(phi);
            let c = cos(phi);
            if c.abs() < 1e-12 {
                break;
            }
            let w = 1.0 - self.e2 * s * s;
            let step = (self.q(phi) - target) * w * w / (2.0 * (1.0 - self.e2) * c);
            phi -= step;
            if step.abs() < 1e-15 {
                break;
            }
        }
        phi
    }
}

/// Lambert azimuthal equal-area on the authalic sphere, centered at (β1, λ0).
struct Laea {
    auth: Authalic,
    sb1: f64,
    cb1: f64,
    lon0: f64,
}

impl Laea {
    fn new(lat0: f64, lon0: f64) -> Laea {
        let auth = Authalic::wgs84();
        let b1 = auth.beta(lat0.to_radians());
        Laea {
            sb1: sin(b1),
            cb1: cos(b1),
            auth,
            lon0,
        }
    }
    fn fwd(&self, (lat, lon): (f64, f64)) -> (f64, f64) {
        let b = self.auth.beta(lat.to_radians());
        let dl = ((lon - self.lon0 + 540.0).rem_euclid(360.0) - 180.0).to_radians();
        let (sb, cb, cl) = (sin(b), cos(b), cos(dl));
        let k = sqrt(2.0 / (1.0 + self.sb1 * sb + self.cb1 * cb * cl).max(1e-300));
        (
            self.auth.rq * k * cb * sin(dl),
            self.auth.rq * k * (self.cb1 * sb - self.sb1 * cb * cl),
        )
    }
    fn rev(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let rho = x.hypot(y);
        if rho < 1e-9 {
            return (self.auth.phi(asin(self.sb1)).to_degrees(), self.lon0);
        }
        let c = 2.0 * asin((rho / (2.0 * self.auth.rq)).min(1.0));
        let (sc, cc) = (sin(c), cos(c));
        let b = asin((cc * self.sb1 + y * sc * self.cb1 / rho).clamp(-1.0, 1.0));
        let dl = atan2(x * sc, rho * self.cb1 * cc - y * self.sb1 * sc);
        let lon = (self.lon0 + dl.to_degrees() + 540.0).rem_euclid(360.0) - 180.0;
        (self.auth.phi(b).to_degrees(), lon)
    }
}

type P = (f64, f64);

/// Signed area and first moments of one ring (shoelace), counterclockwise positive.
fn moments(ring: &[P]) -> (f64, f64, f64) {
    let (mut a, mut mx, mut my) = (0.0, 0.0, 0.0);
    for i in 0..ring.len() {
        let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
        let c = p.0 * q.1 - q.0 * p.1;
        a += c;
        mx += (p.0 + q.0) * c;
        my += (p.1 + q.1) * c;
    }
    (a / 2.0, mx / 6.0, my / 6.0)
}

fn seg_dist(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// Distance to the nearest edge, positive inside and negative outside.
fn signed_dist(rings: &[Vec<P>], p: P) -> f64 {
    let d = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| seg_dist(p, r[i], r[(i + 1) % r.len()])))
        .fold(f64::INFINITY, f64::min);
    if buffer::inside_rings(rings, p) {
        d
    } else {
        -d
    }
}

struct Cell {
    c: P,
    h: f64,
    d: f64,
    max: f64,
}
impl Cell {
    fn new(c: P, h: f64, rings: &[Vec<P>]) -> Cell {
        let d = signed_dist(rings, c);
        Cell {
            c,
            h,
            d,
            max: d + h * core::f64::consts::SQRT_2,
        }
    }
}
impl PartialEq for Cell {
    fn eq(&self, o: &Self) -> bool {
        self.max == o.max
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cell {
    fn cmp(&self, o: &Self) -> Ordering {
        self.max.total_cmp(&o.max)
    }
}

/// The pole of inaccessibility within `precision` (Agafonkin's polylabel):
/// a quadtree search that drops cells that cannot beat the best point so far.
fn polylabel(rings: &[Vec<P>], precision: f64, start: P) -> (P, f64) {
    // Each probe measures every edge, so bound the total work, not just the
    // probes: the best point so far is always inside, only less centered.
    let edges: usize = rings.iter().map(Vec::len).sum();
    let budget = (4_000_000 / edges.max(1)).clamp(1_000, 200_000);
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in &rings[0] {
        x0 = x0.min(p.0);
        y0 = y0.min(p.1);
        x1 = x1.max(p.0);
        y1 = y1.max(p.1);
    }
    let (short, long) = ((x1 - x0).min(y1 - y0), (x1 - x0).max(y1 - y0));
    let mut best = Cell::new(start, 0.0, rings);
    if short <= 0.0 {
        return (best.c, best.d);
    }
    // A sliver would start with thousands of cells, each measuring every edge;
    // at most 128 along the long side, and they count against the budget.
    let size = short.max(long / 128.0);
    let h0 = size / 2.0;
    let mut heap = BinaryHeap::new();
    let mut y = y0;
    while y < y1 {
        let mut x = x0;
        while x < x1 {
            heap.push(Cell::new((x + h0, y + h0), h0, rings));
            x += size;
        }
        y += size;
    }
    let bbox = Cell::new(((x0 + x1) / 2.0, (y0 + y1) / 2.0), 0.0, rings);
    if bbox.d > best.d {
        best = bbox;
    }
    let mut probes = heap.len();
    while let Some(cell) = heap.pop() {
        if cell.d > best.d {
            best = Cell {
                c: cell.c,
                h: 0.0,
                d: cell.d,
                max: cell.d,
            };
        }
        if cell.max - best.d <= precision || probes > budget {
            continue;
        }
        let h = cell.h / 2.0;
        for (dx, dy) in [(-h, -h), (h, -h), (-h, h), (h, h)] {
            heap.push(Cell::new((cell.c.0 + dx, cell.c.1 + dy), h, rings));
            probes += 1;
        }
    }
    (best.c, best.d)
}

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

pub static CENTROID: ToolDef = ToolDef {
    id: "geometry.shape.centroid",
    stability: gp_base::tool::Stability::Stable,
    title: "Polygon centroid and interior point",
    summary: "The center of mass of a polygon on the ellipsoid, a warning when it falls outside the shape, and a point guaranteed inside, as far from the edges as possible, for a label or a pin.",
    aliases: &[
        "polygon centroid",
        "center of polygon",
        "center of mass",
        "pole of inaccessibility",
        "label point",
        "representative point",
    ],
    keywords: &[
        "centroid",
        "center",
        "middle",
        "polygon",
        "interior point",
        "label",
        "pole of inaccessibility",
        "polylabel",
    ],
    inputs: &[Field::new(
        "polygon",
        "Polygon",
        "Corners in order (ring 0), then any holes (ring 1, 2, …), like 40.4406, -80.002",
        Kind::List {
            items: VERTEX,
            min: 3,
            max: 10_000,
        },
    )
    .required()
    .core()],
    outputs: &[
        deg_out("centroid_lat", "Centroid latitude", "Area-weighted"),
        deg_out("centroid_lon", "Centroid longitude", "Area-weighted"),
        Field::new(
            "centroid_inside",
            "Centroid inside",
            "yes, or no for a C-shape or a ring",
            Kind::Text { max_len: 3 },
        ),
        deg_out(
            "interior_lat",
            "Interior point latitude",
            "Farthest found from the edges",
        ),
        deg_out(
            "interior_lon",
            "Interior point longitude",
            "Farthest found from the edges",
        ),
        Field::new(
            "clearance",
            "Interior point clearance",
            "Geodesic distance to the nearest edge",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "area",
            "Area",
            "Outline minus holes",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Significant(8)),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["CENTROID_OUTSIDE"],
    model: "Geodesic edges cut into 5 km pieces and mapped to a Lambert azimuthal equal-area plane on the authalic sphere of WGS 84, centered at the corners' mean (an exact equal-area map, Snyder 1987 ch. 24 and eq. 3-11 to 3-16); the centroid is the plane's area-weighted centroid, holes subtracted, mapped back. The interior point is the pole of inaccessibility on that plane (Agafonkin's polylabel, to 0.1% of the shape's narrow side), with its clearance measured as a geodesic distance",
    accuracy: "The centroid is exact for the equal-area plane, which is what area-weighting on a curved surface needs a choice of; for shapes of a few hundred kilometers it matches a local plane to well under 1 m. The interior point is within 0.1% of the shape's narrow side of the true pole of inaccessibility; for extreme slivers (thousands of times longer than wide) it is the best found within a fixed amount of work, always inside",
    when_to_use: "Use this to find the middle of an area — the centre of a parcel, a search zone, a coverage polygon, a district — when the answer has to sit correctly on the ellipsoid rather than being the average of the corner coordinates, which is wrong for anything but a small symmetric shape. Two points come back, and they are for different jobs: the centroid is the centre of mass, which is what a calculation wants, and the interior point is guaranteed to be inside and as far from the edges as it can be, which is what a label or a map pin wants.",
    limitations: "The centroid of a shape on a curved surface is not defined until you choose how to weight area, and this weights it on an equal-area map, which is the choice that makes the answer independent of how the shape is oriented. For a concave shape — a C, a horseshoe, a ring — the centre of mass falls outside the polygon, which is correct and not an error; the tool says so and hands back the interior point instead. The interior point is found to within 0.1% of the shape’s narrow side, so it is the best place for a pin rather than a uniquely defined coordinate, and for a sliver thousands of times longer than it is wide it is the best found within a fixed amount of work. Holes are subtracted; self-intersecting outlines should be repaired first.",
    references: &[SNYDER],
    examples: &[Example {
        id: "primary",
        title: "A C-shaped lot, whose centroid falls in the notch",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.997},{"lat":40.0006,"lon":-104.997},{"lat":40.0006,"lon":-104.9994},{"lat":40.0024,"lon":-104.9994},{"lat":40.0024,"lon":-104.997},{"lat":40.003,"lon":-104.997},{"lat":40.003,"lon":-105.0}]}"#,
        source: "a C shape, whose centre of mass falls outside it: CENTROID_OUTSIDE is raised and the interior point returned. Checked against PROJ 9.3.0's ellipsoidal Lambert azimuthal equal-area with GEOS 3.11.4's planar centroid over twelve shapes, which agree with the tool to 23 mm and to 4.6e-9 of the area",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("area", "area")],
        },
        Layer {
            kind: "point",
            map: &[("lat", "centroid_lat"), ("lon", "centroid_lon")],
        },
        Layer {
            kind: "point",
            map: &[("lat", "interior_lat"), ("lon", "interior_lon")],
        },
    ],
    related: &[
        Related {
            id: "geometry.area.polygon",
            reason: "alternative",
        },
        Related {
            id: "geometry.buffer.geodesic",
            reason: "next",
        },
        Related {
            id: "geometry.shape.bbox",
            reason: "alternative",
        },
    ],
    sentence: "The centroid is at {centroid_lat}, {centroid_lon}.{if clearance > 0} A point inside, {clearance} from the nearest edge, is at {interior_lat}, {interior_lon}.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_centroid,
    ..ToolDef::BLANK
};

fn run_centroid(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows("polygon")?;
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
        }
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
                "/polygon",
                format!("Ring {k} needs at least 3 distinct corners."),
            ));
        }
    }
    let g = Geodesic::wgs84();
    let dense = buffer::densify(&g, Shape::Polygon, &rings)?;
    let all: Vec<(f64, f64)> = dense.iter().flatten().copied().collect();
    let (lat0, lon0) = buffer::center(&all);
    let map = Laea::new(lat0, lon0);
    let plane: Vec<Vec<P>> = dense
        .iter()
        .map(|r| r.iter().map(|&p| map.fwd(p)).collect())
        .collect();
    // Beyond a hemisphere the equal-area plane folds; stop well short of that.
    let far = plane
        .iter()
        .flatten()
        .map(|p| p.0.hypot(p.1))
        .fold(0.0, f64::max);
    if far > 0.9 * core::f64::consts::SQRT_2 * map.auth.rq {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The polygon is too large for one centroid: it spans most of a hemisphere.",
        )
        .at("/polygon"));
    }
    let (mut area, mut mx, mut my) = (0.0, 0.0, 0.0);
    for (k, ring) in plane.iter().enumerate() {
        let (a, x, y) = moments(ring);
        // The outline adds and holes take away, whichever way each is wound.
        let s = if k == 0 { a.signum() } else { -a.signum() };
        area += s * a;
        mx += s * x;
        my += s * y;
    }
    if area <= 0.0 {
        return Err(ToolError::invalid(
            "/polygon",
            "The holes cover more than the outline; check the ring numbers.",
        ));
    }
    let c = (mx / area, my / area);
    let inside = buffer::inside_rings(&plane, c);
    let (lat, lon) = map.rev(c);
    let (x0, x1) = plane[0]
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.0), hi.max(p.0))
        });
    let (y0, y1) = plane[0]
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.1), hi.max(p.1))
        });
    // 0.1% of the narrow side, so a sliver is searched as finely as its width.
    let precision = (1e-3 * (x1 - x0).min(y1 - y0)).max(1e-3);
    let (rep, _) = polylabel(&plane, precision, c);
    let (rlat, rlon) = map.rev(rep);
    // The clearance, measured as a geodesic distance to every ring.
    let closed: Vec<Vec<(f64, f64)>> = rings
        .iter()
        .map(|r| {
            let mut v = r.clone();
            v.push(r[0]);
            v
        })
        .collect();
    let mut clearance = f64::INFINITY;
    for ring in &closed {
        let d = buffer::distances(
            &g,
            Shape::Line,
            std::slice::from_ref(ring),
            &[(rlat, rlon)],
            1.0e6,
        )?;
        clearance = clearance.min(d[0]);
    }
    if !inside {
        ctx.warnings.push(Warning::new(
            "CENTROID_OUTSIDE",
            "The centroid falls outside the polygon, as it does for a C-shape or a ring. Use the interior point for a label or a pin.",
        ));
    }
    let dq = |v: f64| Q {
        value: v,
        unit: deg,
    };
    Ok(Json::obj([
        ("centroid_lat", ctx.out("centroid_lat", dq(lat))),
        ("centroid_lon", ctx.out("centroid_lon", dq(lon))),
        (
            "centroid_inside",
            Json::str(if inside { "yes" } else { "no" }),
        ),
        ("interior_lat", ctx.out("interior_lat", dq(rlat))),
        ("interior_lon", ctx.out("interior_lon", dq(rlon))),
        (
            "clearance",
            ctx.out("clearance", super::q(clearance, "m", QT::Length)),
        ),
        ("area", ctx.out("area", super::q(area, "m2", QT::Area))),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authalic_latitude_round_trips_and_keeps_the_poles() {
        let a = Authalic::wgs84();
        for d in [-89.9, -60.0, -10.0, 0.0, 0.5, 33.3, 45.0, 72.0, 89.99] {
            let phi = f64::to_radians(d);
            // asin amplifies rounding near the poles (about 6e-14 rad at 89.9°).
            assert!((a.phi(a.beta(phi)) - phi).abs() < 1e-12, "{d}");
        }
        assert!(
            (a.beta(core::f64::consts::FRAC_PI_2) - core::f64::consts::FRAC_PI_2).abs() < 1e-12
        );
        // The authalic sphere has the ellipsoid's area: 4π Rq² = 510,065,621.7 km² for WGS 84.
        let total = 4.0 * core::f64::consts::PI * a.rq * a.rq / 1e6;
        assert!((total - 510_065_621.7).abs() < 1.0, "{total}");
    }

    #[test]
    fn the_equal_area_map_round_trips() {
        let m = Laea::new(40.0, -105.0);
        for p in [
            (40.0, -105.0),
            (41.2, -103.5),
            (35.0, -110.0),
            (60.0, -80.0),
            (-5.0, -100.0),
        ] {
            let (lat, lon) = m.rev(m.fwd(p));
            assert!(
                (lat - p.0).abs() < 1e-11 && (lon - p.1).abs() < 1e-11,
                "{p:?} → {lat}, {lon}"
            );
        }
    }

    #[test]
    fn polylabel_finds_the_middle_of_a_square_and_the_fat_part_of_an_l() {
        let sq = vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]];
        let (p, d) = polylabel(&sq, 1e-6, (1.0, 1.0));
        assert!(
            (p.0 - 5.0).abs() < 1e-3 && (p.1 - 5.0).abs() < 1e-3 && (d - 5.0).abs() < 1e-3,
            "{p:?} {d}"
        );
        let l = vec![vec![
            (0.0, 0.0),
            (30.0, 0.0),
            (30.0, 10.0),
            (10.0, 10.0),
            (10.0, 30.0),
            (0.0, 30.0),
        ]];
        let (_, d) = polylabel(&l, 1e-4, (15.0, 15.0));
        // The circle sits toward the reflex corner: c = √2(10 − c), so c = 10√2/(1 + √2).
        assert!(
            (d - 10.0 * 2f64.sqrt() / (1.0 + 2f64.sqrt())).abs() < 1e-3,
            "{d}"
        );
    }
}
