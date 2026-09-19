//! State Plane Coordinate System of 1983 (geodesy/projections spec): the 124
//! NGS zones on GRS 80, from the EPSG dataset (generated table in
//! `spcs83_zones.rs`). Transverse Mercator uses Krüger's series (`tm`);
//! Lambert Conic Conformal (2SP) and Hotine Oblique Mercator (variant A) follow
//! IOGP Guidance Note 7-2 (EPSG methods 9802 and 9812).

use crate::tm::Tm;
use libm::{asin, atan, atan2, cos, exp, log, pow, sin, sqrt, tan};

pub use crate::spcs83_zones::ZONES;

/// GRS 80, the NAD83 ellipsoid.
pub const GRS80_A: f64 = 6_378_137.0;
pub const GRS80_F: f64 = 1.0 / 298.257_222_101;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Proj {
    Tm {
        lat0: f64,
        lon0: f64,
        k0: f64,
        fe: f64,
        fn_: f64,
    },
    Lcc {
        lat0: f64,
        lon0: f64,
        lat1: f64,
        lat2: f64,
        fe: f64,
        fn_: f64,
    },
    OmercA {
        latc: f64,
        lonc: f64,
        alpha: f64,
        gamma: f64,
        k0: f64,
        fe: f64,
        fn_: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// NGS zone code, like "3702".
    pub fips: &'static str,
    pub epsg: u32,
    pub name: &'static str,
    pub proj: Proj,
    /// The feet unit EPSG defines for the zone ("ftUS" or "ft"), if any.
    pub feet: Option<&'static str>,
    /// Area of use: west, south, east, north (degrees).
    pub bbox: [f64; 4],
}

/// A projected point: easting, northing (m), convergence (degrees, the bearing
/// of grid north clockwise from true north), and point scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub e: f64,
    pub n: f64,
    pub convergence: f64,
    pub k: f64,
}

fn e2() -> f64 {
    GRS80_F * (2.0 - GRS80_F)
}

/// Isometric-latitude helper t(φ) of Guidance Note 7-2.
fn t_of(phi: f64, e: f64) -> f64 {
    let s = sin(phi);
    tan(core::f64::consts::FRAC_PI_4 - phi / 2.0) / pow((1.0 - e * s) / (1.0 + e * s), e / 2.0)
}

fn m_of(phi: f64, e2: f64) -> f64 {
    cos(phi) / sqrt(1.0 - e2 * sin(phi) * sin(phi))
}

/// φ from t by the fixed-point iteration of Guidance Note 7-2 (to 1e-14 rad).
fn phi_of_t(t: f64, e: f64) -> f64 {
    let mut phi = core::f64::consts::FRAC_PI_2 - 2.0 * atan(t);
    for _ in 0..30 {
        let s = sin(phi);
        let next = core::f64::consts::FRAC_PI_2
            - 2.0 * atan(t * pow((1.0 - e * s) / (1.0 + e * s), e / 2.0));
        let done = (next - phi).abs() < 1e-14;
        phi = next;
        if done {
            break;
        }
    }
    phi
}

struct Lcc {
    n: f64,
    af: f64,
    rf: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

fn lcc(lat0: f64, lon0: f64, lat1: f64, lat2: f64, fe: f64, fn_: f64) -> Lcc {
    let (e2, e) = (e2(), sqrt(e2()));
    let (p1, p2, p0) = (lat1.to_radians(), lat2.to_radians(), lat0.to_radians());
    let (m1, m2) = (m_of(p1, e2), m_of(p2, e2));
    let (t1, t2, t0) = (t_of(p1, e), t_of(p2, e), t_of(p0, e));
    let n = if (lat1 - lat2).abs() < 1e-12 {
        sin(p1)
    } else {
        (log(m1) - log(m2)) / (log(t1) - log(t2))
    };
    let f = m1 / (n * pow(t1, n));
    let af = GRS80_A * f;
    Lcc {
        n,
        af,
        rf: af * pow(t0, n),
        lon0,
        fe,
        fn_,
    }
}

impl Lcc {
    fn forward(&self, lat: f64, lon: f64) -> Grid {
        let (e2, e) = (e2(), sqrt(e2()));
        let phi = lat.to_radians();
        let r = self.af * pow(t_of(phi, e), self.n);
        let theta = self.n * (lon - self.lon0).to_radians();
        Grid {
            e: self.fe + r * sin(theta),
            n: self.fn_ + self.rf - r * cos(theta),
            convergence: theta.to_degrees(),
            k: r * self.n / (GRS80_A * m_of(phi, e2)),
        }
    }

    fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let e = sqrt(e2());
        let (dx, dy) = (x - self.fe, self.rf - (y - self.fn_));
        let s = self.n.signum();
        let r = s * (dx * dx + dy * dy).sqrt();
        let t = pow(r / self.af, 1.0 / self.n);
        let theta = atan2(s * dx, s * dy);
        (
            phi_of_t(t, e).to_degrees(),
            theta.to_degrees() / self.n + self.lon0,
        )
    }
}

struct OmercA {
    a: f64,
    b: f64,
    h: f64,
    g0: f64,
    lon0: f64,
    gc: f64,
    fe: f64,
    fn_: f64,
}

fn omerc_a(latc: f64, lonc: f64, alpha: f64, gamma: f64, k0: f64, fe: f64, fn_: f64) -> OmercA {
    let (e2, e) = (e2(), sqrt(e2()));
    let pc = latc.to_radians();
    let b = sqrt(1.0 + e2 * pow(cos(pc), 4.0) / (1.0 - e2));
    let a = GRS80_A * b * k0 * sqrt(1.0 - e2) / (1.0 - e2 * sin(pc) * sin(pc));
    let t0 = t_of(pc, e);
    let d = (b * sqrt(1.0 - e2) / (cos(pc) * sqrt(1.0 - e2 * sin(pc) * sin(pc)))).max(1.0);
    let f = d + sqrt(d * d - 1.0) * pc.signum();
    let h = f * pow(t0, b);
    let g = (f - 1.0 / f) / 2.0;
    let g0 = asin(sin(alpha.to_radians()) / d);
    let lon0 = lonc - (asin(g * tan(g0)) / b).to_degrees();
    OmercA {
        a,
        b,
        h,
        g0,
        lon0,
        gc: gamma.to_radians(),
        fe,
        fn_,
    }
}

impl OmercA {
    fn uv(&self, lat: f64, lon: f64) -> (f64, f64) {
        let e = sqrt(e2());
        let q = self.h / pow(t_of(lat.to_radians(), e), self.b);
        let s = (q - 1.0 / q) / 2.0;
        let t = (q + 1.0 / q) / 2.0;
        let dl = self.b * (lon - self.lon0).to_radians();
        let v = sin(dl);
        let u_ = (-v * cos(self.g0) + s * sin(self.g0)) / t;
        let vv = self.a * log((1.0 - u_) / (1.0 + u_)) / (2.0 * self.b);
        let uu = self.a * atan2(s * cos(self.g0) + v * sin(self.g0), cos(dl)) / self.b;
        (uu, vv)
    }

    fn en(&self, lat: f64, lon: f64) -> (f64, f64) {
        let (u, v) = self.uv(lat, lon);
        (
            v * cos(self.gc) + u * sin(self.gc) + self.fe,
            u * cos(self.gc) - v * sin(self.gc) + self.fn_,
        )
    }

    fn forward(&self, lat: f64, lon: f64) -> Grid {
        let (x, y) = self.en(lat, lon);
        // Convergence and scale from derivatives by Richardson-extrapolated
        // central differences: steps of 1e-3° keep cancellation and the O(h⁴)
        // truncation near 1e-9° (1e-6° steps lost 1e-7° to cancellation).
        let h = 1e-3;
        let diff = |dlat: f64, dlon: f64| {
            let at = |s: f64| {
                let (xp, yp) = self.en(lat + s * dlat, lon + s * dlon);
                let (xm, ym) = self.en(lat - s * dlat, lon - s * dlon);
                ((xp - xm) / (2.0 * s), (yp - ym) / (2.0 * s))
            };
            let (x1, y1) = at(h);
            let (x2, y2) = at(h / 2.0);
            ((4.0 * x2 - x1) / 3.0, (4.0 * y2 - y1) / 3.0)
        };
        let (xn, yn) = diff(1.0, 0.0);
        let (xe, ye) = diff(0.0, 1.0);
        let e2 = e2();
        let phi = lat.to_radians();
        let w = sqrt(1.0 - e2 * sin(phi) * sin(phi));
        // Length of one degree along the parallel on the ellipsoid.
        let par = 1f64.to_radians() * GRS80_A * cos(phi) / w;
        Grid {
            e: x,
            n: y,
            convergence: -atan2(xn, yn).to_degrees(),
            k: xe.hypot(ye) / par,
        }
    }

    fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let e2 = e2();
        let (dx, dy) = (x - self.fe, y - self.fn_);
        let v = dx * cos(self.gc) - dy * sin(self.gc);
        let u = dy * cos(self.gc) + dx * sin(self.gc);
        let q = exp(-(self.b * v / self.a));
        let s = (q - 1.0 / q) / 2.0;
        let t = (q + 1.0 / q) / 2.0;
        let vv = sin(self.b * u / self.a);
        let uu = (vv * cos(self.g0) + s * sin(self.g0)) / t;
        let tt = pow(self.h / sqrt((1.0 + uu) / (1.0 - uu)), 1.0 / self.b);
        // Solve for φ exactly rather than with the truncated series in G7-2.
        let phi = phi_of_t(tt, sqrt(e2));
        let lon = self.lon0
            - (atan2(
                s * cos(self.g0) - vv * sin(self.g0),
                cos(self.b * u / self.a),
            ) / self.b)
                .to_degrees();
        (phi.to_degrees(), lon)
    }
}

/// Wraps a longitude difference into [-180, 180).
fn dlon(lon: f64, lon0: f64) -> f64 {
    let d = (lon - lon0 + 180.0).rem_euclid(360.0) - 180.0;
    if d == 180.0 { -180.0 } else { d }
}

impl Zone {
    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        match self.proj {
            Proj::Tm {
                lat0,
                lon0,
                k0,
                fe,
                fn_,
            } => {
                let tm = Tm::new(GRS80_A, GRS80_F, k0);
                let (x, y, gam, k) = tm.forward(lat, dlon(lon, lon0));
                let (_, y0, _, _) = tm.forward(lat0, 0.0);
                Grid {
                    e: fe + x,
                    n: fn_ + y - y0,
                    convergence: gam,
                    k,
                }
            }
            Proj::Lcc {
                lat0,
                lon0,
                lat1,
                lat2,
                fe,
                fn_,
            } => lcc(lat0, lon0, lat1, lat2, fe, fn_).forward(lat, lon0 + dlon(lon, lon0)),
            Proj::OmercA {
                latc,
                lonc,
                alpha,
                gamma,
                k0,
                fe,
                fn_,
            } => omerc_a(latc, lonc, alpha, gamma, k0, fe, fn_).forward(lat, lon),
        }
    }

    /// (lat, lon) in degrees from easting and northing in meters.
    pub fn inverse(&self, e: f64, n: f64) -> (f64, f64) {
        match self.proj {
            Proj::Tm {
                lat0,
                lon0,
                k0,
                fe,
                fn_,
            } => {
                let tm = Tm::new(GRS80_A, GRS80_F, k0);
                let (_, y0, _, _) = tm.forward(lat0, 0.0);
                let (lat, dl) = tm.inverse(e - fe, n - fn_ + y0);
                (lat, lon0 + dl)
            }
            Proj::Lcc {
                lat0,
                lon0,
                lat1,
                lat2,
                fe,
                fn_,
            } => lcc(lat0, lon0, lat1, lat2, fe, fn_).inverse(e, n),
            Proj::OmercA {
                latc,
                lonc,
                alpha,
                gamma,
                k0,
                fe,
                fn_,
            } => omerc_a(latc, lonc, alpha, gamma, k0, fe, fn_).inverse(e, n),
        }
    }

    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        let [w, s, e, n] = self.bbox;
        lat >= s
            && lat <= n
            && if w <= e {
                lon >= w && lon <= e
            } else {
                lon >= w || lon <= e
            }
    }

    /// "Pennsylvania South" (without the trailing "zone").
    pub fn short_name(&self) -> &'static str {
        self.name.strip_suffix(" zone").unwrap_or(self.name)
    }
}

/// Finds a zone by NGS code ("3702"), EPSG code, or name ("Pennsylvania South").
pub fn find(key: &str) -> Option<&'static Zone> {
    let k = key.trim();
    let norm = |s: &str| {
        s.to_ascii_lowercase()
            .replace(" zone", "")
            .replace(['&', '.'], "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    ZONES.iter().find(|z| {
        z.fips == k
            || (k.len() == 3 && k.bytes().all(|b| b.is_ascii_digit()) && z.fips == format!("0{k}"))
            || z.epsg.to_string() == k
            || norm(z.name) == norm(k)
    })
}

/// Zones whose area-of-use box contains the point.
pub fn candidates(lat: f64, lon: f64) -> Vec<&'static Zone> {
    ZONES.iter().filter(|z| z.contains(lat, lon)).collect()
}
