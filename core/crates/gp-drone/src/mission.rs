//! Mission patterns (drone/mission-patterns spec): the serpentine survey grid
//! over a polygon with holes, the image count that uses the same sweep
//! (photogrammetry "Image count and survey size"), corridor lines that follow
//! a centerline, and orbits with camera heading and gimbal pitch.
//!
//! Geometry is planned on a transverse Mercator plane centered on the area
//! (Krüger series, k0 = 1): conformal, with scale error under 1e-7 within a few
//! kilometers of the center, and mapped back to latitude and longitude.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::tm::Tm;
use libm::{atan, atan2, cos, hypot, sin};

const WGS84_A: f64 = 6_378_137.0;
const WGS84_F: f64 = 1.0 / 298.257_223_563;

const PIX4D: Reference = Reference {
    title: "Elements of Photogrammetry with Applications in GIS",
    issuer: "Wolf, P. R., Dewitt, B. A., and Wilkinson, B. E., McGraw-Hill",
    year: 2014,
    edition: "4th edition",
    locator: "Chapter 18 (flight planning: flight lines, photo spacing, number of photos)",
    url: "https://www.mheducation.com/highered/product/elements-photogrammetry-applications-gis-wolf-dewitt/9780071761123.html",
};
pub(crate) const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (direct and inverse geodesic problems)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};

/// The local plane: TM centered at (lat0, lon0).
pub struct Plane {
    tm: Tm,
    lat0: f64,
    lon0: f64,
    y0: f64,
}

impl Plane {
    pub fn new(lat0: f64, lon0: f64) -> Plane {
        let tm = Tm::new(WGS84_A, WGS84_F, 1.0);
        let (_, y0, _, _) = tm.forward(lat0, 0.0);
        Plane { tm, lat0, lon0, y0 }
    }

    /// (x east, y north) in meters.
    pub fn fwd(&self, lat: f64, lon: f64) -> (f64, f64) {
        let dl = (lon - self.lon0 + 540.0).rem_euclid(360.0) - 180.0;
        let (x, y, _, _) = self.tm.forward(lat, dl);
        (x, y - self.y0)
    }

    pub fn inv(&self, x: f64, y: f64) -> (f64, f64) {
        let (lat, dl) = self.tm.inverse(x, y + self.y0);
        let lon = (self.lon0 + dl + 540.0).rem_euclid(360.0) - 180.0;
        (lat, lon)
    }

    pub fn center(&self) -> (f64, f64) {
        (self.lat0, self.lon0)
    }
}

type P = (f64, f64);
/// The plane, the outer ring, and the holes.
type Polygon = (Plane, Vec<P>, Vec<Vec<P>>);

fn rot(p: P, a: f64) -> P {
    (p.0 * cos(a) - p.1 * sin(a), p.0 * sin(a) + p.1 * cos(a))
}

fn cross(o: P, a: P, b: P) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Convex hull (Andrew's monotone chain), counterclockwise.
pub fn hull(pts: &[P]) -> Vec<P> {
    let mut p: Vec<P> = pts.to_vec();
    p.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let mut lower: Vec<P> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<P> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The line direction (radians, math angle from +x) that gives the fewest
/// lines: parallel to the hull edge with the smallest width.
pub fn min_width_angle(outer: &[P]) -> f64 {
    let h = hull(outer);
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..h.len() {
        let (a, b) = (h[i], h[(i + 1) % h.len()]);
        let len = hypot(b.0 - a.0, b.1 - a.1);
        if len == 0.0 {
            continue;
        }
        let w = h
            .iter()
            .map(|&q| cross(a, b, q).abs() / len)
            .fold(0.0, f64::max);
        if w < best.0 - 1e-9 {
            best = (w, atan2(b.1 - a.1, b.0 - a.0));
        }
    }
    best.1
}

/// Buffered convex hull of a hole: the hole's points pushed out by `buf` in 16 directions.
fn buffered(hole: &[P], buf: f64) -> Vec<P> {
    let mut pts = Vec::with_capacity(hole.len() * 16);
    for &p in hole {
        for k in 0..16 {
            let a = k as f64 * core::f64::consts::PI / 8.0;
            // Circumscribe the circle so the polygon contains every point within buf.
            let r = buf / cos(core::f64::consts::PI / 16.0);
            pts.push((p.0 + r * cos(a), p.1 + r * sin(a)));
        }
    }
    hull(&pts)
}

/// x-intervals where the horizontal line y = `y` is inside `rings` (even-odd).
fn intervals(rings: &[Vec<P>], y: f64) -> Vec<(f64, f64)> {
    let mut xs = Vec::new();
    for r in rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            if (a.1 > y) != (b.1 > y) {
                xs.push(a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1));
            }
        }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    xs.chunks(2)
        .filter(|c| c.len() == 2 && c[1] > c[0])
        .map(|c| (c[0], c[1]))
        .collect()
}

/// The swept lines: for each line (in the rotated frame), its y and inside intervals.
pub struct Sweep {
    pub angle: f64,
    pub lines: Vec<(f64, Vec<(f64, f64)>)>,
    pub obstacles: Vec<Vec<P>>,
}

/// Sweeps lines at `spacing` over the outer ring minus buffered holes, lines at
/// `angle` (radians from +x). The first line is half a spacing inside the area.
pub fn sweep(outer: &[P], holes: &[Vec<P>], angle: f64, spacing: f64, buffer: f64) -> Sweep {
    let obstacles: Vec<Vec<P>> = holes.iter().map(|h| buffered(h, buffer)).collect();
    let r = |p: &P| rot(*p, -angle);
    let mut rings: Vec<Vec<P>> = vec![outer.iter().map(r).collect()];
    rings.extend(
        obstacles
            .iter()
            .map(|h| h.iter().map(r).collect::<Vec<_>>()),
    );
    let (ymin, ymax) = rings[0]
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.1), hi.max(p.1))
        });
    // Slack for round-off: a 1,000 m extent at 100 m spacing is 10 lines, not 11.
    let n = ((ymax - ymin) / spacing - 1e-6).ceil().max(1.0) as usize;
    // Center the lines in the extent: n lines cover n·spacing.
    let first = ymin + ((ymax - ymin) - (n as f64 - 1.0) * spacing) / 2.0;
    let lines = (0..n)
        .map(|k| {
            let y = first + k as f64 * spacing;
            (y, intervals(&rings, y))
        })
        .filter(|(_, iv)| !iv.is_empty())
        .collect();
    Sweep {
        angle,
        lines,
        obstacles,
    }
}

/// Photos along an interval of length `l` at `p` spacing: one at each end and every p between.
pub fn photos(l: f64, p: f64) -> usize {
    (l / p).floor() as usize + 1
}

fn seg_hits(a: P, b: P, c: P, d: P) -> Option<f64> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / den;
    let u = ((c.0 - a.0) * r.1 - (c.1 - a.1) * r.0) / den;
    // Endpoints count: a line split by a hole ends exactly on its boundary.
    ((-1e-9..=1.0 + 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u))
        .then_some(t.clamp(0.0, 1.0))
}

/// Strictly inside a counterclockwise convex polygon.
fn inside_convex(h: &[P], p: P) -> bool {
    (0..h.len()).all(|i| cross(h[i], h[(i + 1) % h.len()], p) > 1e-7)
}

/// A transit from a to b that goes around any obstacle along its boundary (shorter side).
fn route(a: P, b: P, obstacles: &[Vec<P>]) -> Vec<P> {
    for h in obstacles {
        let mut hits: Vec<(f64, usize)> = Vec::new();
        for i in 0..h.len() {
            if let Some(t) = seg_hits(a, b, h[i], h[(i + 1) % h.len()]) {
                hits.push((t, i));
            }
        }
        if hits.len() < 2 {
            continue;
        }
        hits.sort_by(|x, y| x.0.partial_cmp(&y.0).expect("finite"));
        let (t_in, e_in) = hits[0];
        let (t_out, e_out) = hits[hits.len() - 1];
        let tm = (t_in + t_out) / 2.0;
        if t_out - t_in < 1e-9
            || !inside_convex(h, (a.0 + tm * (b.0 - a.0), a.1 + tm * (b.1 - a.1)))
        {
            continue; // touches the obstacle without crossing it
        }
        let pin = (a.0 + t_in * (b.0 - a.0), a.1 + t_in * (b.1 - a.1));
        let pout = (a.0 + t_out * (b.0 - a.0), a.1 + t_out * (b.1 - a.1));
        let n = h.len();
        // Walk forward (edge e_in+1 .. e_out) or backward, whichever is shorter.
        let walk = |fwd: bool| -> Vec<P> {
            let mut v = vec![pin];
            let mut i = if fwd { (e_in + 1) % n } else { e_in };
            let stop = if fwd { (e_out + 1) % n } else { e_out };
            let mut guard = 0;
            while i != stop && guard <= n {
                v.push(h[i]);
                i = if fwd { (i + 1) % n } else { (i + n - 1) % n };
                guard += 1;
            }
            v.push(pout);
            v
        };
        let len = |v: &Vec<P>| {
            v.windows(2)
                .map(|w| hypot(w[1].0 - w[0].0, w[1].1 - w[0].1))
                .sum::<f64>()
        };
        let (f, bk) = (walk(true), walk(false));
        let path = if len(&f) <= len(&bk) { f } else { bk };
        let mut out = route(a, pin, obstacles);
        out.extend(path.into_iter().skip(1));
        out.extend(route(pout, b, obstacles).into_iter().skip(1));
        return out;
    }
    vec![a, b]
}

/// A serpentine path: waypoints (plane coordinates) with a kind, and totals.
pub struct Path {
    pub points: Vec<(P, &'static str)>,
    pub lines: usize,
    pub survey_length: f64,
    pub photos: usize,
}

pub fn serpentine(s: &Sweep, overshoot: f64, photo_spacing: f64) -> Path {
    let mut pts: Vec<(P, &'static str)> = Vec::new();
    let (mut survey, mut n_photos) = (0.0, 0);
    for (k, (y, iv)) in s.lines.iter().enumerate() {
        let mut segs: Vec<(f64, f64)> = iv.clone();
        if k % 2 == 1 {
            segs.reverse();
            segs = segs.into_iter().map(|(a, b)| (b, a)).collect();
        }
        for (j, &(x0, x1)) in segs.iter().enumerate() {
            let dir = (x1 - x0).signum();
            let (xs, xe) = (
                if j == 0 { x0 - dir * overshoot } else { x0 },
                if j + 1 == segs.len() {
                    x1 + dir * overshoot
                } else {
                    x1
                },
            );
            let a = rot((xs, *y), s.angle);
            let b = rot((xe, *y), s.angle);
            if let Some(&(last, _)) = pts.last() {
                for p in route(last, a, &s.obstacles).into_iter().skip(1) {
                    pts.push((p, "transit"));
                }
                pts.pop();
            }
            pts.push((a, "line_start"));
            pts.push((b, "line_end"));
            survey += (x1 - x0).abs();
            n_photos += photos((x1 - x0).abs(), photo_spacing);
        }
    }
    Path {
        points: pts,
        lines: s.lines.len(),
        survey_length: survey,
        photos: n_photos,
    }
}

pub fn path_length(pts: &[(P, &str)]) -> f64 {
    pts.windows(2)
        .map(|w| hypot(w[1].0.0 - w[0].0.0, w[1].0.1 - w[0].0.1))
        .sum()
}

// ---------------------------------------------------------------- inputs

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
        "0 for the area, 1, 2, … for holes (no-fly areas)",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("m"),
    }
}

/// Reads the polygon rows into the plane: outer ring and holes.
fn read_polygon(ctx: &mut Ctx) -> Result<Polygon, ToolError> {
    let rows = ctx.rows("area")?;
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("area", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("area", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-85.0..=85.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Mission areas must be between 85° S and 85° N.",
            )
            .at(&format!("/area/{i}/lat")));
        }
        let ring = match r.get("ring") {
            None | Some(serde_json::Value::Null) => 0,
            Some(v) => match v.as_f64() {
                Some(x) if x.fract() == 0.0 && (0.0..=100.0).contains(&x) => x as usize,
                _ => {
                    return Err(ToolError::invalid(
                        &format!("/area/{i}/ring"),
                        "Ring must be a whole number from 0 (the area) to 100.",
                    ));
                }
            },
        };
        if rings.len() <= ring {
            rings.resize(ring + 1, Vec::new());
        }
        rings[ring].push((lat, lon));
    }
    if rings.first().is_none_or(|r| r.len() < 3) {
        return Err(ToolError::invalid(
            "/area",
            "The area needs at least 3 corners (ring 0).",
        ));
    }
    let n = rings[0].len() as f64;
    let lat0 = rings[0].iter().map(|p| p.0).sum::<f64>() / n;
    // Mean longitude about the first vertex (safe across ±180°).
    let lon_ref = rings[0][0].1;
    let lon0 = lon_ref
        + rings[0]
            .iter()
            .map(|p| (p.1 - lon_ref + 540.0).rem_euclid(360.0) - 180.0)
            .sum::<f64>()
            / n;
    let plane = Plane::new(lat0, lon0);
    let proj = |r: &Vec<(f64, f64)>| r.iter().map(|&(a, b)| plane.fwd(a, b)).collect::<Vec<P>>();
    let outer = proj(&rings[0]);
    if outer.iter().any(|p| hypot(p.0, p.1) > 50_000.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Mission areas must fit within 50 km of their center.",
        )
        .at("/area"));
    }
    let holes = rings[1..]
        .iter()
        .filter(|r| r.len() >= 3)
        .map(proj)
        .collect();
    Ok((plane, outer, holes))
}

fn read_angle(ctx: &mut Ctx, outer: &[P]) -> Result<(f64, f64), ToolError> {
    // Returns (plane angle from +x in radians, azimuth in degrees).
    match ctx.raw("direction").and_then(|v| v.as_str()).map(str::trim) {
        None | Some("auto") => {
            let a = min_width_angle(outer);
            Ok((a, (90.0 - a.to_degrees()).rem_euclid(180.0)))
        }
        Some(_) => {
            let az = ctx
                .quantity("direction")?
                .expect("set")
                .to(units::by_symbol(QT::Angle, "deg").expect("deg"));
            Ok(((90.0 - az).to_radians(), az.rem_euclid(360.0)))
        }
    }
}

fn positive(ctx: &mut Ctx, name: &str, what: &str) -> Result<f64, ToolError> {
    let v = ctx.req_quantity(name)?.base();
    if v <= 0.0 {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            format!("{what} must be positive."),
        ));
    }
    Ok(v)
}

const AREA: Field = Field::new(
    "area",
    "Area",
    "Corners of the area (ring 0) and any no-fly holes (ring 1, 2, …), in order",
    Kind::List {
        items: VERTEX,
        min: 3,
        max: 2000,
    },
);
const SPACING: Field = qty(
    "line_spacing",
    "Line spacing",
    "From the photogrammetry trigger tool, like 52.5 m",
    QT::Length,
    "m",
);
const PHOTO: Field = qty(
    "photo_spacing",
    "Photo spacing",
    "Along-track distance between photos, like 30 m",
    QT::Length,
    "m",
);
const DIRECTION: Field = Field::new(
    "direction",
    "Line direction",
    "auto (fewest lines, default) or an azimuth like 45 deg",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
);
const OVERSHOOT: Field = qty(
    "overshoot",
    "Overshoot",
    "Extra length past each line end, default 0 m",
    QT::Length,
    "m",
);

const WAYPOINT: &[Field] = &[
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
        "kind",
        "Kind",
        "line_start, line_end, or transit",
        Kind::Text { max_len: 12 },
    ),
];

fn waypoints(plane: &Plane, pts: &[(P, &str)]) -> Json {
    let deg = |v: f64| {
        Q {
            value: v,
            unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
        }
        .to_json()
    };
    Json::Arr(
        pts.iter()
            .map(|((x, y), k)| {
                let (la, lo) = plane.inv(*x, *y);
                Json::obj([("lat", deg(la)), ("lon", deg(lo)), ("kind", Json::str(*k))])
            })
            .collect(),
    )
}

pub static SURVEY_GRID: ToolDef = ToolDef {
    id: "drone.mission.survey-grid",
    title: "Survey grid (lawnmower pattern)",
    summary: "A serpentine mapping pattern over an area, with no-fly holes routed around, the fewest lines by default, overshoot, and an optional crosshatch: waypoints, line count, path length, turns, photo count, and flight time.",
    aliases: &[
        "drone survey grid",
        "lawnmower pattern",
        "mapping flight plan",
        "crosshatch grid",
    ],
    keywords: &[
        "survey grid",
        "lawnmower",
        "serpentine",
        "flight lines",
        "crosshatch",
        "mapping mission",
        "waypoints",
        "photogrammetry",
    ],
    inputs: &[
        AREA.required().core(),
        SPACING.required().core(),
        PHOTO.required().core(),
        DIRECTION.core(),
        OVERSHOOT,
        qty(
            "hole_buffer",
            "Hole buffer",
            "Distance kept from no-fly holes, default 10 m",
            QT::Length,
            "m",
        ),
        Field::new(
            "crosshatch",
            "Crosshatch",
            "no (default) or yes for a second, perpendicular grid",
            Kind::Choice(&["no", "yes"]),
        ),
        qty(
            "groundspeed",
            "Groundspeed",
            "For flight time, like 10 m/s",
            QT::Speed,
            "m/s",
        )
        .core(),
        qty(
            "height",
            "Height above ground",
            "Like 120 m AGL (constant)",
            QT::Length,
            "m",
        ),
    ],
    outputs: &[
        Field::new(
            "lines",
            "Flight lines",
            "Number of lines",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "photos",
            "Photos",
            "At the photo spacing, one at each line end",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        qty(
            "path_length",
            "Path length",
            "Lines, overshoot, and transits",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "survey_length",
            "Line length over the area",
            "Inside the area only",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "turns",
            "Turns",
            "Changes of direction",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        qty(
            "flight_time",
            "Flight time",
            "Path length / groundspeed, without turns or climbs",
            QT::Time,
            "min",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "direction",
            "Line direction",
            "Azimuth of the lines",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "height_reference",
            "Height",
            "Every waypoint's height and reference",
            Kind::Text { max_len: 60 },
        )
        .optional(),
        Field::new(
            "waypoints",
            "Waypoints",
            "In flight order",
            Kind::List {
                items: WAYPOINT,
                min: 0,
                max: 100_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Lines swept on a local transverse Mercator plane, clipped to the area minus buffered holes (even-odd), joined in serpentine order; transits that would cross a hole follow its buffered boundary",
    accuracy: "Line spacing is true to within 1e-7 across a few kilometers (checked geodesically). Flight time ignores turns, climbs, and wind.",
    references: &[PIX4D, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 600 m by 150 m field at 52.5 m line spacing",
        input: r#"{"area":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99295},{"lat":40.00135,"lon":-104.99295},{"lat":40.00135,"lon":-105.0}],"line_spacing":"52.5 m","photo_spacing":"30 m","groundspeed":"10 m/s","height":"120 m"}"#,
        source: "add-drone-suite survey-grid scenario: lines run along the long axis, ⌈150 / 52.5⌉ = 3 lines",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("path", "waypoints")],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.image-count",
            reason: "alternative",
        },
    ],
    sentence: "Fly {lines} {plural lines \"line\" \"lines\"} for {path_length} and about {photos} photos.{if flight_time > 0} That takes about {flight_time}.{/if}",
    limits: &[("batchRows", 100)],
    run: run_grid,
    ..ToolDef::BLANK
};

fn grid_paths(ctx: &mut Ctx) -> Result<(Plane, Vec<Path>, f64, f64), ToolError> {
    let (plane, outer, holes) = read_polygon(ctx)?;
    let s = positive(ctx, "line_spacing", "Line spacing")?;
    let p = positive(ctx, "photo_spacing", "Photo spacing")?;
    if s < 0.5 || p < 0.1 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Line spacing must be at least 0.5 m and photo spacing at least 0.1 m.",
        )
        .at("/line_spacing"));
    }
    let over = ctx
        .quantity("overshoot")?
        .map_or(0.0, |q| q.base())
        .max(0.0);
    if over > 10_000.0 {
        return Err(
            ToolError::new(ErrorCode::OutOfDomain, "Overshoot must be at most 10 km.")
                .at("/overshoot"),
        );
    }
    let buf = if ctx.declares("hole_buffer") {
        ctx.quantity("hole_buffer")?
            .map_or(10.0, |q| q.base())
            .max(0.0)
    } else {
        10.0
    };
    let (angle, az) = read_angle(ctx, &outer)?;
    let (xmin, xmax, ymin, ymax) = outer
        .iter()
        .fold((f64::MAX, f64::MIN, f64::MAX, f64::MIN), |a, q| {
            (a.0.min(q.0), a.1.max(q.0), a.2.min(q.1), a.3.max(q.1))
        });
    if (xmax - xmin).max(ymax - ymin) / s > 5_000.0 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            "That spacing would need more than 5,000 lines. Use a wider spacing or a smaller area.",
        )
        .at("/line_spacing"));
    }
    let mut paths = vec![serpentine(&sweep(&outer, &holes, angle, s, buf), over, p)];
    if ctx.declares("crosshatch") && ctx.choice("crosshatch")? == Some("yes") {
        paths.push(serpentine(
            &sweep(&outer, &holes, angle + core::f64::consts::FRAC_PI_2, s, buf),
            over,
            p,
        ));
    }
    let twice: f64 = (0..outer.len())
        .map(|i| {
            let (a, b) = (outer[i], outer[(i + 1) % outer.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    Ok((plane, paths, az, twice.abs() / 2.0))
}

fn run_grid(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (plane, paths, az, _) = grid_paths(ctx)?;
    let mut pts: Vec<(P, &'static str)> = Vec::new();
    for p in &paths {
        pts.extend(p.points.iter().copied());
    }
    let (lines, photos, survey): (usize, usize, f64) = paths.iter().fold((0, 0, 0.0), |a, p| {
        (a.0 + p.lines, a.1 + p.photos, a.2 + p.survey_length)
    });
    let length = path_length(&pts);
    let turns = pts.len().saturating_sub(2);
    let mut o = vec![
        ("lines", Json::Num(lines as f64)),
        ("photos", Json::Num(photos as f64)),
        ("path_length", ctx.out("path_length", m(length))),
        ("survey_length", ctx.out("survey_length", m(survey))),
        ("turns", Json::Num(turns as f64)),
    ];
    if let Some(gs) = ctx.quantity("groundspeed")? {
        let gs = gs.base();
        if gs <= 0.0 {
            return Err(ToolError::invalid(
                "/groundspeed",
                "Groundspeed must be positive.",
            ));
        }
        o.push((
            "flight_time",
            ctx.out(
                "flight_time",
                Q {
                    value: length / gs,
                    unit: units::by_symbol(QT::Time, "s").expect("s"),
                },
            ),
        ));
    }
    o.push((
        "direction",
        ctx.out(
            "direction",
            Q {
                value: az,
                unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
            },
        ),
    ));
    if let Some(h) = ctx.quantity("height")? {
        o.push((
            "height_reference",
            Json::str(format!(
                "{} above ground level (constant) at every waypoint",
                display::quantity(h.base(), "m", Precision::Decimals(1), ctx.options.format)
            )),
        ));
    }
    o.push(("waypoints", waypoints(&plane, &pts)));
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- image count

pub static IMAGE_COUNT: ToolDef = ToolDef {
    id: "drone.photogrammetry.image-count",
    title: "Image count and survey size",
    summary: "How many photos, flight lines, and kilometers a mapping survey of an area takes at your line and photo spacing, from the same sweep the survey-grid tool flies.",
    aliases: &[
        "how many photos drone survey",
        "image count calculator",
        "drone mapping photo count",
    ],
    keywords: &[
        "image count",
        "photos",
        "flight lines",
        "survey size",
        "mapping",
        "overlap",
        "coverage",
    ],
    inputs: &[
        AREA.required().core(),
        SPACING.required().core(),
        PHOTO.required().core(),
        DIRECTION.core(),
        OVERSHOOT,
        qty(
            "groundspeed",
            "Groundspeed",
            "Like 10 m/s",
            QT::Speed,
            "m/s",
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "photos",
            "Photos",
            "At the photo spacing",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "lines",
            "Flight lines",
            "Number of lines",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        qty(
            "survey_length",
            "Line length over the area",
            "Inside the area",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "path_length",
            "Path length",
            "With overshoot and turns between lines",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        qty("area_size", "Area", "Of the area polygon", QT::Area, "ha")
            .precision(Precision::Decimals(2)),
        qty(
            "flight_time",
            "Flight time",
            "Path length / groundspeed",
            QT::Time,
            "min",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "The survey-grid sweep without waypoint output: photos per line = ⌊length / photo spacing⌋ + 1",
    accuracy: "Equal to the survey-grid pattern for the same settings (no holes here). Real missions add photos on turns if the camera keeps shooting.",
    references: &[PIX4D],
    examples: &[Example {
        id: "primary",
        title: "The 600 m by 150 m field",
        input: r#"{"area":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99295},{"lat":40.00135,"lon":-104.99295},{"lat":40.00135,"lon":-105.0}],"line_spacing":"52.5 m","photo_spacing":"30 m"}"#,
        source: "Wolf, Dewitt, and Wilkinson, ch. 18 flight-planning method: 3 lines of about 600 m at 30 m spacing",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("area", "area_size")],
    }],
    related: &[Related {
        id: "drone.mission.survey-grid",
        reason: "alternative",
    }],
    sentence: "Plan on about {photos} photos over {lines} {plural lines \"line\" \"lines\"} and {path_length} of flying.",
    limits: &[("batchRows", 1_000)],
    run: run_count,
    ..ToolDef::BLANK
};

fn run_count(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (_, paths, _, area_m2) = grid_paths(ctx)?;
    let p = &paths[0];
    let len = path_length(&p.points);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Area to cover",
            "the polygon you gave, in the plane the grid is flown on",
            format!("{} corners", p.points.len()),
            format!("{} ha", n(area_m2 / 10_000.0, 3)),
        );
        ctx.step(
            "Flight lines",
            "the area's width divided by the spacing between lines",
            format!("{} m of survey line", n(p.survey_length, 0)),
            format!("{} lines", n(p.lines as f64, 0)),
        );
        ctx.step(
            "Photos",
            "the line length divided by the distance between photos, over every line",
            format!(
                "{} m along {} lines",
                n(p.survey_length, 0),
                n(p.lines as f64, 0)
            ),
            // The card prints the count bare; the last step reads the same.
            n(p.photos as f64, 0),
        );
    }
    let mut o = vec![
        ("photos", Json::Num(p.photos as f64)),
        ("lines", Json::Num(p.lines as f64)),
        (
            "survey_length",
            ctx.out("survey_length", m(p.survey_length)),
        ),
        ("path_length", ctx.out("path_length", m(len))),
        (
            "area_size",
            ctx.out(
                "area_size",
                Q {
                    value: area_m2,
                    unit: units::by_symbol(QT::Area, "m2").expect("m2"),
                },
            ),
        ),
    ];
    if let Some(gs) = ctx.quantity("groundspeed")? {
        let gs = gs.base();
        if gs <= 0.0 {
            return Err(ToolError::invalid(
                "/groundspeed",
                "Groundspeed must be positive.",
            ));
        }
        o.push((
            "flight_time",
            ctx.out(
                "flight_time",
                Q {
                    value: len / gs,
                    unit: units::by_symbol(QT::Time, "s").expect("s"),
                },
            ),
        ));
    }
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- corridor

const CENTER_ROW: &[Field] = &[
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

const LINE_ROW: &[Field] = &[
    qty(
        "offset",
        "Offset",
        "From the centerline, right positive",
        QT::Length,
        "m",
    )
    .precision(Precision::Decimals(1)),
    Field::new(
        "waypoints",
        "Waypoints",
        "Along this line",
        Kind::List {
            items: WAYPOINT,
            min: 0,
            max: 10_000,
        },
    ),
];

pub static CORRIDOR: ToolDef = ToolDef {
    id: "drone.mission.corridor",
    title: "Corridor mapping lines",
    summary: "Parallel flight lines that follow a centerline (a pipeline, road, or river) across a corridor width, with the line count from your spacing, flown back and forth.",
    aliases: &[
        "corridor mapping",
        "pipeline survey",
        "linear corridor flight plan",
    ],
    keywords: &[
        "corridor",
        "pipeline",
        "road",
        "powerline",
        "offset lines",
        "linear mapping",
    ],
    inputs: &[
        Field::new(
            "centerline",
            "Centerline",
            "Points along the corridor, in order, like 40, -105",
            Kind::List {
                items: CENTER_ROW,
                min: 2,
                max: 5000,
            },
        )
        .required()
        .core(),
        qty("width", "Corridor width", "Like 120 m", QT::Length, "m")
            .required()
            .core(),
        SPACING.required().core(),
    ],
    outputs: &[
        Field::new(
            "line_count",
            "Lines",
            "⌈width / spacing⌉",
            Kind::Number { min: 1.0, max: 1e4 },
        )
        .precision(Precision::Decimals(0)),
        qty(
            "centerline_length",
            "Centerline length",
            "Geodesic",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(3)),
        qty(
            "path_length",
            "Path length",
            "All lines and end turns",
            QT::Distance,
            "km",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "lines",
            "Lines",
            "Offsets and waypoints, in flight order",
            Kind::List {
                items: LINE_ROW,
                min: 0,
                max: 10_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::DegenerateGeometry],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Offset polylines on a local transverse Mercator plane, mitered at bends (miter length limited to 4× the offset), offsets (k − (n − 1)/2)·spacing",
    accuracy: "Offsets are exact on the plane, true to within 1e-7 across a few kilometers.",
    references: &[PIX4D],
    examples: &[Example {
        id: "primary",
        title: "A 120 m corridor at 52.5 m spacing",
        input: r#"{"centerline":[{"lat":40.0,"lon":-105.0},{"lat":40.02,"lon":-104.98},{"lat":40.03,"lon":-104.95}],"width":"120 m","line_spacing":"52.5 m"}"#,
        source: "add-drone-suite corridor scenario: three lines at -52.5, 0, and +52.5 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("path", "lines")],
    }],
    related: &[Related {
        id: "drone.mission.survey-grid",
        reason: "alternative",
    }],
    sentence: "Fly {line_count} {plural line_count \"line\" \"lines\"} along {centerline_length} of centerline, {path_length} in all.",
    limits: &[("batchRows", 100)],
    run: run_corridor,
    ..ToolDef::BLANK
};

/// Offsets a polyline by `d` (right positive) with mitered joins.
pub fn offset_line(pts: &[P], d: f64) -> Vec<P> {
    let n = pts.len();
    let normal = |a: P, b: P| {
        let l = hypot(b.0 - a.0, b.1 - a.1);
        ((b.1 - a.1) / l, -(b.0 - a.0) / l) // right-hand normal
    };
    (0..n)
        .map(|i| {
            if i == 0 {
                let nm = normal(pts[0], pts[1]);
                (pts[0].0 + d * nm.0, pts[0].1 + d * nm.1)
            } else if i == n - 1 {
                let nm = normal(pts[n - 2], pts[n - 1]);
                (pts[i].0 + d * nm.0, pts[i].1 + d * nm.1)
            } else {
                let (n1, n2) = (normal(pts[i - 1], pts[i]), normal(pts[i], pts[i + 1]));
                let b = (n1.0 + n2.0, n1.1 + n2.1);
                let bl = hypot(b.0, b.1);
                if bl < 1e-12 {
                    return (pts[i].0 + d * n1.0, pts[i].1 + d * n1.1);
                }
                let cos_half = (b.0 * n1.0 + b.1 * n1.1) / bl;
                let k = (d / cos_half).clamp(-4.0 * d.abs(), 4.0 * d.abs());
                (pts[i].0 + k * b.0 / bl, pts[i].1 + k * b.1 / bl)
            }
        })
        .collect()
}

fn run_corridor(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("centerline")?;
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let mut ll = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let la = ctx
            .row_quantity("centerline", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lo = ctx
            .row_quantity("centerline", i, r, "lon")?
            .expect("required")
            .to(deg);
        ll.push((la, lo));
    }
    let w = positive(ctx, "width", "Corridor width")?;
    let s = positive(ctx, "line_spacing", "Line spacing")?;
    let mid = ll[ll.len() / 2];
    let plane = Plane::new(mid.0, mid.1);
    let pts: Vec<P> = ll.iter().map(|&(a, b)| plane.fwd(a, b)).collect();
    if pts
        .windows(2)
        .any(|w| hypot(w[1].0 - w[0].0, w[1].1 - w[0].1) < 1e-6)
    {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "Two consecutive centerline points are the same.",
        )
        .at("/centerline"));
    }
    if pts.iter().any(|p| hypot(p.0, p.1) > 50_000.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Corridors must stay within 50 km of their midpoint; split longer ones.",
        )
        .at("/centerline"));
    }
    let geod = Geodesic::wgs84();
    let center_len: f64 = ll
        .windows(2)
        .map(|w| {
            let d: f64 = geod.inverse(w[0].0, w[0].1, w[1].0, w[1].1);
            d
        })
        .sum();
    let lines_needed = (w / s - 1e-9).ceil().max(1.0);
    if lines_needed > 1_000.0 || lines_needed * pts.len() as f64 > 100_000.0 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            "That corridor would need more than 1,000 lines or 100,000 waypoints. Use a wider spacing, a narrower corridor, or fewer centerline points.",
        )
        .at("/width"));
    }
    let n = lines_needed as usize;
    let mut lines = Vec::new();
    let mut flown: Vec<(P, &'static str)> = Vec::new();
    for k in 0..n {
        let off = (k as f64 - (n as f64 - 1.0) / 2.0) * s;
        let mut line = offset_line(&pts, off);
        if k % 2 == 1 {
            line.reverse();
        }
        let wp: Vec<(P, &'static str)> = line
            .iter()
            .enumerate()
            .map(|(j, p)| {
                (
                    *p,
                    if j == 0 {
                        "line_start"
                    } else if j + 1 == line.len() {
                        "line_end"
                    } else {
                        "transit"
                    },
                )
            })
            .collect();
        flown.extend(wp.iter().copied());
        lines.push(Json::obj([
            ("offset", m(off).to_json()),
            ("waypoints", waypoints(&plane, &wp)),
        ]));
    }
    Ok(Json::obj(vec![
        ("line_count", Json::Num(n as f64)),
        (
            "centerline_length",
            ctx.out("centerline_length", m(center_len)),
        ),
        (
            "path_length",
            ctx.out("path_length", m(path_length(&flown))),
        ),
        ("lines", Json::Arr(lines)),
    ]))
}

// ---------------------------------------------------------------- orbit

const ORBIT_ROW: &[Field] = &[
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
        "heading",
        "Heading",
        "Toward the center",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(1)),
    Field::new(
        "gimbal_pitch",
        "Gimbal pitch",
        "Negative is down",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(1)),
];

pub static ORBIT: ToolDef = ToolDef {
    id: "drone.mission.orbit",
    title: "Orbit around a point of interest",
    summary: "Waypoints on a geodesic circle around a point, each with the heading toward the center and the gimbal pitch that keeps a target height centered, like the top of a tower.",
    aliases: &[
        "drone orbit",
        "point of interest orbit",
        "circle mission",
        "tower inspection orbit",
    ],
    keywords: &[
        "orbit",
        "POI",
        "point of interest",
        "circle",
        "gimbal pitch",
        "tower",
        "inspection",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Center latitude",
            "Decimal degrees, like 40.4406",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Center longitude",
            "Decimal degrees, like -80.002",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-180,180)"),
        qty("radius", "Radius", "Like 50 m", QT::Length, "m")
            .required()
            .core(),
        qty(
            "height",
            "Flight height",
            "Above the center's ground, like 80 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qty(
            "target_height",
            "Target height",
            "What to keep centered, like 60 m (the tower top); default 0 (the base)",
            QT::Length,
            "m",
        )
        .core(),
        Field::new(
            "photos",
            "Photos",
            "Waypoints around the circle, default 36",
            Kind::Number {
                min: 3.0,
                max: 3600.0,
            },
        ),
        Field::new(
            "rotation",
            "Direction",
            "clockwise (default) or counterclockwise",
            Kind::Choice(&["clockwise", "counterclockwise"]),
        ),
    ],
    outputs: &[
        qty(
            "gimbal_pitch",
            "Gimbal pitch",
            "−atan((height − target) / radius)",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "circumference",
            "Circumference",
            "Geodesic",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "photo_spacing",
            "Photo spacing",
            "Along the circle",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "waypoints",
            "Waypoints",
            "In flight order",
            Kind::List {
                items: ORBIT_ROW,
                min: 0,
                max: 3600,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Geodesic direct from the center at equal azimuth steps (Karney 2013); heading = reverse azimuth; pitch = −atan(Δh / r)",
    accuracy: "Exact geodesic positions; heights are relative to the center's ground level.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 60 m tower, 50 m radius, flying at 80 m",
        input: r#"{"lat":40.0,"lon":-105.0,"radius":"50 m","height":"80 m","target_height":"60 m","photos":12}"#,
        source: "add-drone-suite orbit scenario: gimbal pitch -21.8° (atan(20/50)), heading toward the center",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("path", "waypoints")],
    }],
    related: &[Related {
        id: "drone.mission.survey-grid",
        reason: "alternative",
    }],
    sentence: "Point the camera {gimbal_pitch} and fly the {circumference} circle.",
    limits: &[("batchRows", 100)],
    run: run_orbit,
    ..ToolDef::BLANK
};

fn run_orbit(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let lat = ctx.req_quantity("lat")?.to(deg);
    let lon = ctx.req_quantity("lon")?.to(deg);
    if !(-89.0..=89.0).contains(&lat) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The center must be between 89° S and 89° N.",
        )
        .at("/lat"));
    }
    let r = positive(ctx, "radius", "Radius")?;
    let h = ctx.req_quantity("height")?.base();
    let t = ctx.quantity("target_height")?.map_or(0.0, |q| q.base());
    let n = ctx.number("photos")?.unwrap_or(36.0).round() as usize;
    let cw = ctx.choice("rotation")? != Some("counterclockwise");
    let pitch = -atan((h - t) / r).to_degrees();
    let geod = Geodesic::wgs84();
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    let mut wps = Vec::new();
    let mut circ = 0.0;
    let mut prev: Option<(f64, f64)> = None;
    for k in 0..=n {
        let az = if cw {
            360.0 * k as f64 / n as f64
        } else {
            -360.0 * k as f64 / n as f64
        };
        let (la, lo, az2): (f64, f64, f64) = geod.direct(lat, lon, az, r);
        if let Some((pla, plo)) = prev {
            let d: f64 = geod.inverse(pla, plo, la, lo);
            circ += d;
        }
        prev = Some((la, lo));
        if k < n {
            let heading = (az2 + 180.0).rem_euclid(360.0);
            wps.push(Json::obj([
                ("lat", dq(la)),
                ("lon", dq((lo + 540.0).rem_euclid(360.0) - 180.0)),
                ("heading", dq(heading)),
                ("gimbal_pitch", dq(pitch)),
            ]));
        }
    }
    // Chords undercount the arc slightly; use the geodesic circle's arc length.
    let arc = circ * (core::f64::consts::PI / n as f64) / sin(core::f64::consts::PI / n as f64);
    Ok(Json::obj(vec![
        (
            "gimbal_pitch",
            ctx.out(
                "gimbal_pitch",
                Q {
                    value: pitch,
                    unit: deg,
                },
            ),
        ),
        ("circumference", ctx.out("circumference", m(arc))),
        ("photo_spacing", ctx.out("photo_spacing", m(arc / n as f64))),
        ("waypoints", Json::Arr(wps)),
    ]))
}
