//! Map projections with user-set parameters (geodesy/projections spec,
//! "General projection methods with custom parameters"), on any ellipsoid,
//! following IOGP Guidance Note 7-2: Lambert Conic Conformal 1SP and 2SP
//! (EPSG 9801, 9802), Albers Equal Area (9822), Polar Stereographic variants
//! A and B (9810, 9829), Popular Visualisation Pseudo Mercator, the Web
//! Mercator of EPSG:3857 (1024), and Equidistant Cylindrical (1028).
//!
//! Latitudes that Guidance Note 7-2 recovers with truncated series are solved
//! here to convergence instead: φ from the isometric t by fixed-point
//! iteration, φ from Albers' q by Newton's method, and φ from a meridian
//! distance by the Krüger series of `tm`, which is good to nanometers.

use crate::tm::Tm;
use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use libm::{asin, asinh, atan, atan2, cos, exp, log, pow, sin, sinh, sqrt, tan};

/// A projected point: easting and northing (m), convergence (degrees, the
/// bearing of grid north clockwise from true north), and the point scale
/// along the meridian (h) and along the parallel (k), equal in a conformal
/// projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub e: f64,
    pub n: f64,
    pub convergence: f64,
    pub h: f64,
    pub k: f64,
}

/// The limit of Web Mercator, atan(sinh π): the latitude that makes the map square.
pub fn web_mercator_limit() -> f64 {
    atan(sinh(PI)).to_degrees()
}

/// Isometric-latitude helper t(φ) of Guidance Note 7-2.
pub fn t_of(phi: f64, e: f64) -> f64 {
    let s = sin(phi);
    tan(FRAC_PI_4 - phi / 2.0) / pow((1.0 - e * s) / (1.0 + e * s), e / 2.0)
}

/// m(φ) = cos φ / √(1 − e² sin² φ), the parallel's radius over a.
pub fn m_of(phi: f64, e2: f64) -> f64 {
    cos(phi) / sqrt(1.0 - e2 * sin(phi) * sin(phi))
}

/// φ from t by the fixed-point iteration of Guidance Note 7-2 (to 1e-14 rad).
pub fn phi_of_t(t: f64, e: f64) -> f64 {
    let mut phi = FRAC_PI_2 - 2.0 * atan(t);
    for _ in 0..30 {
        let s = sin(phi);
        let next = FRAC_PI_2 - 2.0 * atan(t * pow((1.0 - e * s) / (1.0 + e * s), e / 2.0));
        let done = (next - phi).abs() < 1e-14;
        phi = next;
        if done {
            break;
        }
    }
    phi
}

/// Wraps a longitude difference into [-180, 180). Within ±540° it adds or
/// takes 360 once, which is exact (Sterbenz); shifting by 180 first, as
/// `(d + 180) mod 360 − 180` does, would round away several nanometers.
pub fn dlon(lon: f64, lon0: f64) -> f64 {
    let mut d = lon - lon0;
    if !(-540.0..=540.0).contains(&d) {
        d = d.rem_euclid(360.0);
    }
    if d >= 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    if d >= 180.0 { d - 360.0 } else { d }
}

fn wrap(lon: f64) -> f64 {
    dlon(lon, 0.0)
}

/// Transverse Mercator (EPSG method 9807) by Krüger's series to sixth order
/// (Karney 2011, `tm`), with an origin, scale factor, and falsings.
pub struct TmGrid {
    tm: Tm,
    lon0: f64,
    fe: f64,
    fn_: f64,
    /// The northing of the latitude of origin, which the grid counts from.
    y0: f64,
    k0: f64,
}

/// Within 3,900 km of the central meridian on an Earth-sized ellipsoid the
/// series is good to 5 nm (Karney 2011); the reach scales with the
/// ellipsoid's size.
pub const TM_SERIES_REACH: f64 = 3_900_000.0;

impl TmGrid {
    #[allow(clippy::too_many_arguments)]
    pub fn new(a: f64, f: f64, lat0: f64, lon0: f64, k0: f64, fe: f64, fn_: f64) -> TmGrid {
        let tm = Tm::new(a, f, k0);
        let (_, y0, _, _) = tm.forward(lat0, 0.0);
        TmGrid {
            tm,
            lon0,
            fe,
            fn_,
            y0,
            k0,
        }
    }

    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        let (x, y, gam, k) = self.tm.forward(lat, dlon(lon, self.lon0));
        Grid {
            e: self.fe + x,
            n: self.fn_ + y - self.y0,
            convergence: gam,
            h: k,
            k,
        }
    }

    pub fn inverse(&self, e: f64, n: f64) -> (f64, f64) {
        let (lat, dl) = self.tm.inverse(e - self.fe, n - self.fn_ + self.y0);
        (lat, self.lon0 + dl)
    }

    /// How far the easting is from the central meridian on the ground, for
    /// judging the series' reach.
    pub fn offset(&self, e: f64) -> f64 {
        ((e - self.fe) / self.k0).abs()
    }
}

/// Lambert Conic Conformal, 1SP (EPSG 9801) and 2SP (EPSG 9802).
pub struct Lcc {
    a: f64,
    e2: f64,
    n: f64,
    /// a·F·k0
    af: f64,
    /// The radius of the origin's parallel.
    rf: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

impl Lcc {
    /// Two standard parallels and a false origin (EPSG 9802).
    #[allow(clippy::too_many_arguments)]
    pub fn two_sp(
        a: f64,
        f: f64,
        lat0: f64,
        lon0: f64,
        lat1: f64,
        lat2: f64,
        fe: f64,
        fn_: f64,
    ) -> Lcc {
        let (e2, e) = (f * (2.0 - f), sqrt(f * (2.0 - f)));
        let (p1, p2, p0) = (lat1.to_radians(), lat2.to_radians(), lat0.to_radians());
        let (m1, m2) = (m_of(p1, e2), m_of(p2, e2));
        let (t1, t2, t0) = (t_of(p1, e), t_of(p2, e), t_of(p0, e));
        let n = if (lat1 - lat2).abs() < 1e-12 {
            sin(p1)
        } else {
            (log(m1) - log(m2)) / (log(t1) - log(t2))
        };
        let af = a * m1 / (n * pow(t1, n));
        Lcc {
            a,
            e2,
            n,
            af,
            rf: af * pow(t0, n),
            lon0,
            fe,
            fn_,
        }
    }

    /// One standard parallel, the natural origin's, with a scale factor there (EPSG 9801).
    pub fn one_sp(a: f64, f: f64, lat0: f64, lon0: f64, k0: f64, fe: f64, fn_: f64) -> Lcc {
        let (e2, e) = (f * (2.0 - f), sqrt(f * (2.0 - f)));
        let p0 = lat0.to_radians();
        let (m0, t0, n) = (m_of(p0, e2), t_of(p0, e), sin(p0));
        let af = a * k0 * m0 / (n * pow(t0, n));
        Lcc {
            a,
            e2,
            n,
            af,
            rf: af * pow(t0, n),
            lon0,
            fe,
            fn_,
        }
    }

    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        let phi = lat.to_radians();
        // The pole away from the cone's apex is at infinity; floating point
        // would make it merely huge, since tan(π/2) is finite in doubles.
        if lat * self.n.signum() <= -90.0 {
            return Grid {
                e: f64::INFINITY,
                n: f64::INFINITY,
                convergence: 0.0,
                h: f64::INFINITY,
                k: f64::INFINITY,
            };
        }
        let r = self.af * pow(t_of(phi, sqrt(self.e2)), self.n);
        let theta = self.n * dlon(lon, self.lon0).to_radians();
        let k = r * self.n / (self.a * m_of(phi, self.e2));
        Grid {
            e: self.fe + r * sin(theta),
            n: self.fn_ + self.rf - r * cos(theta),
            convergence: theta.to_degrees(),
            h: k,
            k,
        }
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.fe, self.rf - (y - self.fn_));
        let s = self.n.signum();
        let r = s * (dx * dx + dy * dy).sqrt();
        let t = pow(r / self.af, 1.0 / self.n);
        let theta = atan2(s * dx, s * dy);
        (
            phi_of_t(t, sqrt(self.e2)).to_degrees(),
            wrap(theta.to_degrees() / self.n + self.lon0),
        )
    }
}

/// Albers Equal Area (EPSG 9822).
pub struct Albers {
    a: f64,
    e2: f64,
    e: f64,
    n: f64,
    c: f64,
    rho0: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

impl Albers {
    fn q(&self, phi: f64) -> f64 {
        albers_q(phi, self.e2, self.e)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a: f64,
        f: f64,
        lat0: f64,
        lon0: f64,
        lat1: f64,
        lat2: f64,
        fe: f64,
        fn_: f64,
    ) -> Albers {
        let (e2, e) = (f * (2.0 - f), sqrt(f * (2.0 - f)));
        let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
        let (m1, m2) = (m_of(p1, e2), m_of(p2, e2));
        let (q1, q2) = (albers_q(p1, e2, e), albers_q(p2, e2, e));
        let n = if (lat1 - lat2).abs() < 1e-12 {
            sin(p1)
        } else {
            (m1 * m1 - m2 * m2) / (q2 - q1)
        };
        let c = m1 * m1 + n * q1;
        let rho0 = a * sqrt(c - n * albers_q(lat0.to_radians(), e2, e)) / n;
        Albers {
            a,
            e2,
            e,
            n,
            c,
            rho0,
            lon0,
            fe,
            fn_,
        }
    }

    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        let phi = lat.to_radians();
        let rho = self.a * sqrt(self.c - self.n * self.q(phi)) / self.n;
        let theta = self.n * dlon(lon, self.lon0).to_radians();
        // Equal area: the scales along the parallel and the meridian multiply to 1.
        let k = rho * self.n / (self.a * m_of(phi, self.e2));
        Grid {
            e: self.fe + rho * sin(theta),
            n: self.fn_ + self.rho0 - rho * cos(theta),
            convergence: theta.to_degrees(),
            h: 1.0 / k,
            k,
        }
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.fe, self.rho0 - (y - self.fn_));
        let s = self.n.signum();
        let rho = s * (dx * dx + dy * dy).sqrt();
        let q = (self.c - rho * rho * self.n * self.n / (self.a * self.a)) / self.n;
        let theta = atan2(s * dx, s * dy);
        (
            self.phi_of_q(q).to_degrees(),
            wrap(theta.to_degrees() / self.n + self.lon0),
        )
    }

    /// Like `inverse`, or None for a point outside the ring the poles map
    /// to, where q passes its polar value (a nanometer of slack).
    pub fn inverse_checked(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (dx, dy) = (x - self.fe, self.rho0 - (y - self.fn_));
        let rho = (dx * dx + dy * dy).sqrt();
        let q = (self.c - rho * rho * self.n * self.n / (self.a * self.a)) / self.n;
        let qp = self.q(FRAC_PI_2);
        // dq/dρ = 2ρn/a², so a nanometer of ρ moves q by about 2ρ|n|/a² · 1e-9.
        let slack = 2.0 * rho * self.n.abs() / (self.a * self.a) * 1e-9 + 1e-15;
        (q.abs() <= qp + slack).then(|| self.inverse(x, y))
    }

    /// φ from q by Newton's method on q(φ), started from the authalic latitude.
    fn phi_of_q(&self, q: f64) -> f64 {
        let qp = self.q(FRAC_PI_2);
        if q.abs() >= qp {
            return FRAC_PI_2.copysign(q);
        }
        let mut phi = asin(q / qp);
        for _ in 0..40 {
            let (s, c) = (sin(phi), cos(phi));
            let w = 1.0 - self.e2 * s * s;
            let dq = 2.0 * (1.0 - self.e2) * c / (w * w);
            if dq <= 0.0 {
                break;
            }
            let step = (q - self.q(phi)) / dq;
            phi = (phi + step).clamp(-FRAC_PI_2, FRAC_PI_2);
            if step.abs() < 1e-15 {
                break;
            }
        }
        phi
    }
}

/// q(φ) of Guidance Note 7-2 (twice the authalic sine, scaled).
fn albers_q(phi: f64, e2: f64, e: f64) -> f64 {
    let s = sin(phi);
    if e == 0.0 {
        return 2.0 * s;
    }
    (1.0 - e2) * (s / (1.0 - e2 * s * s) - log((1.0 - e * s) / (1.0 + e * s)) / (2.0 * e))
}

/// Polar Stereographic, variant A (scale at the pole, EPSG 9810) or B
/// (a standard parallel, EPSG 9829), about the North or South Pole.
pub struct PolarStereo {
    a: f64,
    e2: f64,
    e: f64,
    north: bool,
    /// 2·a·k0 / √((1+e)^(1+e)·(1−e)^(1−e))
    scale: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

fn polar_c(e: f64) -> f64 {
    sqrt(pow(1.0 + e, 1.0 + e) * pow(1.0 - e, 1.0 - e))
}

impl PolarStereo {
    /// Variant A: the scale factor at the pole.
    pub fn variant_a(
        a: f64,
        f: f64,
        north: bool,
        lon0: f64,
        k0: f64,
        fe: f64,
        fn_: f64,
    ) -> PolarStereo {
        let (e2, e) = (f * (2.0 - f), sqrt(f * (2.0 - f)));
        PolarStereo {
            a,
            e2,
            e,
            north,
            scale: 2.0 * a * k0 / polar_c(e),
            lon0,
            fe,
            fn_,
        }
    }

    /// Variant B: the latitude of true scale; its sign picks the pole.
    pub fn variant_b(a: f64, f: f64, lat_c: f64, lon0: f64, fe: f64, fn_: f64) -> PolarStereo {
        let (e2, e) = (f * (2.0 - f), sqrt(f * (2.0 - f)));
        let north = lat_c > 0.0;
        let pc = lat_c.abs().to_radians();
        let k0 = if (lat_c.abs() - 90.0).abs() < 1e-12 {
            1.0
        } else {
            m_of(pc, e2) * polar_c(e) / (2.0 * t_of(pc, e))
        };
        PolarStereo::variant_a(a, f, north, lon0, k0, fe, fn_)
    }

    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        // The south case is the north case mirrored: φ → −φ, northing flipped.
        let (phi, sign) = if self.north {
            (lat.to_radians(), 1.0)
        } else {
            (-lat.to_radians(), -1.0)
        };
        // The pole opposite the center is at infinity (see `Lcc::forward`).
        let rho = if phi <= -FRAC_PI_2 {
            f64::INFINITY
        } else {
            self.scale * t_of(phi, self.e)
        };
        let dl = dlon(lon, self.lon0).to_radians();
        let k = if (phi - FRAC_PI_2).abs() < 1e-15 {
            self.scale * polar_c(self.e) / (2.0 * self.a)
        } else {
            rho / (self.a * m_of(phi, self.e2))
        };
        Grid {
            e: self.fe + rho * sin(dl),
            n: self.fn_ - sign * rho * cos(dl),
            convergence: sign * dl.to_degrees(),
            h: k,
            k,
        }
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let sign = if self.north { 1.0 } else { -1.0 };
        let (dx, dy) = (x - self.fe, y - self.fn_);
        let rho = dx.hypot(dy);
        let phi = phi_of_t(rho / self.scale, self.e);
        let lon = if rho == 0.0 {
            self.lon0
        } else {
            self.lon0 + atan2(dx, -sign * dy).to_degrees()
        };
        (sign * phi.to_degrees(), wrap(lon))
    }
}

/// Popular Visualisation Pseudo Mercator (EPSG 1024), the Web Mercator of
/// EPSG:3857: spherical Mercator formulas with the WGS 84 semi-major axis,
/// applied to WGS 84 latitudes. It is not conformal on the ellipsoid, so the
/// two scales differ slightly.
pub struct WebMercator;

pub const WGS84_A: f64 = 6_378_137.0;
const WGS84_E2: f64 = (1.0 / 298.257_223_563) * (2.0 - 1.0 / 298.257_223_563);

impl WebMercator {
    pub fn forward(lat: f64, lon: f64) -> Grid {
        let phi = lat.to_radians();
        let (s, c) = (sin(phi), cos(phi));
        let w = 1.0 - WGS84_E2 * s * s;
        // Meridian radius ρ = a(1−e²)/w^1.5 and prime-vertical radius ν = a/√w.
        let rho = WGS84_A * (1.0 - WGS84_E2) / (w * sqrt(w));
        let nu = WGS84_A / sqrt(w);
        Grid {
            e: WGS84_A * wrap(lon).to_radians(),
            n: WGS84_A * asinh(tan(phi)),
            convergence: 0.0,
            h: WGS84_A / (rho * c),
            k: WGS84_A / (nu * c),
        }
    }

    pub fn inverse(x: f64, y: f64) -> (f64, f64) {
        (
            atan(sinh(y / WGS84_A)).to_degrees(),
            wrap((x / WGS84_A).to_degrees()),
        )
    }
}

/// Equidistant Cylindrical (EPSG 1028): northing is the meridian distance
/// from the equator, easting is the distance along the standard parallel.
pub struct EquidistantCylindrical {
    tm: Tm,
    a: f64,
    e2: f64,
    /// ν1·cos φ1, the standard parallel's radius.
    r1: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

impl EquidistantCylindrical {
    pub fn new(a: f64, f: f64, lat1: f64, lon0: f64, fe: f64, fn_: f64) -> EquidistantCylindrical {
        let e2 = f * (2.0 - f);
        EquidistantCylindrical {
            tm: Tm::new(a, f, 1.0),
            a,
            e2,
            r1: a * m_of(lat1.to_radians(), e2),
            lon0,
            fe,
            fn_,
        }
    }

    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        // The central meridian of a transverse Mercator with k0 = 1 is true
        // to scale, so its northing there is the meridian distance.
        let (_, m, _, _) = self.tm.forward(lat, 0.0);
        Grid {
            e: self.fe + self.r1 * dlon(lon, self.lon0).to_radians(),
            n: self.fn_ + m,
            convergence: 0.0,
            h: 1.0,
            k: self.r1 / (self.a * m_of(lat.to_radians(), self.e2)),
        }
    }

    /// Like `inverse`, or None for a northing beyond a pole.
    pub fn inverse_checked(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let quadrant = self.tm.rectifying_radius() * FRAC_PI_2;
        ((y - self.fn_).abs() <= quadrant + 1e-9).then(|| self.inverse(x, y))
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (lat, _) = self.tm.inverse(0.0, y - self.fn_);
        (
            lat,
            wrap(self.lon0 + ((x - self.fe) / self.r1).to_degrees()),
        )
    }
}

/// Convergence and the scales along the meridian and the parallel of a map
/// `f`, from chords of about 100 m of ground each way, Richardson-extrapolated
/// so the curvature of the images cancels: the chord's direction and length
/// are then good to about 1e-10. Near a pole the meridian step shrinks so it
/// never crosses it, and within a millionth of a degree of the pole, where
/// the parallel has no length, the factors are taken that far from it.
fn factors(
    f: impl Fn(f64, f64) -> (f64, f64),
    g: &Geodesic,
    lat: f64,
    lon: f64,
) -> (f64, f64, f64) {
    let lat = lat.clamp(-90.0 + 1e-6, 90.0 - 1e-6);
    let phi = lat.to_radians();
    let e2 = g.f * (2.0 - g.f);
    // The parallel's radius, and a step of 100 m along each line.
    let r = g.a * m_of(phi, e2);
    let dd = (100.0_f64 / 111_000.0).min((90.0 - lat.abs()) * 0.9);
    let dl = (100.0 / r).min(0.1);
    let chord = |(e1, n1): (f64, f64), (e2, n2): (f64, f64)| (e2 - e1, n2 - n1);
    let meridian = |d: f64| {
        let (de, dn) = chord(f(lat - d, lon), f(lat + d, lon));
        let s: f64 = g.inverse(lat - d, lon, lat + d, lon);
        (-atan2(de, dn), de.hypot(dn) / s)
    };
    let parallel = |d: f64| {
        let (de, dn) = chord(
            f(lat, lon - (d / 2.0).to_degrees()),
            f(lat, lon + (d / 2.0).to_degrees()),
        );
        de.hypot(dn) / (2.0 * r * sin(d / 2.0))
    };
    let rich = |a: f64, b: f64| (4.0 * b - a) / 3.0;
    let ((c1, h1), (c2, h2)) = (meridian(dd), meridian(dd / 2.0));
    (
        dlon(rich(c1, c2).to_degrees(), 0.0),
        rich(h1, h2),
        rich(parallel(dl), parallel(dl / 2.0)),
    )
}

/// Hotine Oblique Mercator, variant A (EPSG 9812, falsings at the natural
/// origin) and variant B (EPSG 9815, coordinates given at the projection
/// center), following Guidance Note 7-2. The inverse solves φ exactly rather
/// than with the note's truncated series.
pub struct Hotine {
    /// Guidance Note 7-2's A, B, H, and γ0.
    a: f64,
    b: f64,
    h: f64,
    g0: f64,
    lon0: f64,
    /// The rectified grid angle γc (radians).
    gc: f64,
    fe: f64,
    fn_: f64,
    e2: f64,
    /// Variant B's offset of u to the projection center, |uc|·sign(φc); 0 for A.
    uoff: f64,
    geod: Geodesic,
}

impl Hotine {
    /// `fe` and `fn_` are the false easting and northing (variant A) or the
    /// easting and northing at the projection center (variant B).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a_ell: f64,
        f: f64,
        latc: f64,
        lonc: f64,
        alpha: f64,
        gamma: f64,
        k0: f64,
        fe: f64,
        fn_: f64,
        variant_b: bool,
    ) -> Hotine {
        let (e2, e) = (f * (2.0 - f), sqrt(f * (2.0 - f)));
        let pc = latc.to_radians();
        let b = sqrt(1.0 + e2 * pow(cos(pc), 4.0) / (1.0 - e2));
        let a = a_ell * b * k0 * sqrt(1.0 - e2) / (1.0 - e2 * sin(pc) * sin(pc));
        let t0 = t_of(pc, e);
        let d = (b * sqrt(1.0 - e2) / (cos(pc) * sqrt(1.0 - e2 * sin(pc) * sin(pc)))).max(1.0);
        let f_ = d + sqrt(d * d - 1.0) * pc.signum();
        let h = f_ * pow(t0, b);
        let g = (f_ - 1.0 / f_) / 2.0;
        let g0 = asin(sin(alpha.to_radians()) / d);
        let lon0 = lonc - (asin(g * tan(g0)) / b).to_degrees();
        let uoff = if !variant_b {
            0.0
        } else if (alpha.abs() - 90.0).abs() < 1e-12 {
            (a * (lonc - lon0).to_radians()).abs() * pc.signum()
        } else {
            (a / b * atan(sqrt(d * d - 1.0) / cos(alpha.to_radians()))).abs() * pc.signum()
        };
        Hotine {
            a,
            b,
            h,
            g0,
            lon0,
            gc: gamma.to_radians(),
            fe,
            fn_,
            e2,
            uoff,
            geod: Geodesic::new(a_ell, f),
        }
    }

    fn uv(&self, lat: f64, lon: f64) -> (f64, f64) {
        let e = sqrt(self.e2);
        let q = self.h / pow(t_of(lat.to_radians(), e), self.b);
        let s = (q - 1.0 / q) / 2.0;
        let t = (q + 1.0 / q) / 2.0;
        // Wrapped, so a grid near the antimeridian takes points across it.
        let dl = self.b * dlon(lon, self.lon0).to_radians();
        let v = sin(dl);
        let u_ = (-v * cos(self.g0) + s * sin(self.g0)) / t;
        let vv = self.a * log((1.0 - u_) / (1.0 + u_)) / (2.0 * self.b);
        let uu = self.a * atan2(s * cos(self.g0) + v * sin(self.g0), cos(dl)) / self.b;
        (uu - self.uoff, vv)
    }

    /// Easting and northing.
    pub fn en(&self, lat: f64, lon: f64) -> (f64, f64) {
        let (u, v) = self.uv(lat, lon);
        (
            v * cos(self.gc) + u * sin(self.gc) + self.fe,
            u * cos(self.gc) - v * sin(self.gc) + self.fn_,
        )
    }

    /// The grid point, with convergence and scales from differences of the
    /// map (it is conformal, so the two scales agree to their precision).
    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        let (e, n) = self.en(lat, lon);
        let (convergence, h, k) = factors(|la, lo| self.en(la, lo), &self.geod, lat, lon);
        Grid {
            e,
            n,
            convergence,
            h,
            k,
        }
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.fe, y - self.fn_);
        let v = dx * cos(self.gc) - dy * sin(self.gc);
        let u = dy * cos(self.gc) + dx * sin(self.gc) + self.uoff;
        let q = exp(-(self.b * v / self.a));
        let s = (q - 1.0 / q) / 2.0;
        let t = (q + 1.0 / q) / 2.0;
        let vv = sin(self.b * u / self.a);
        let uu = (vv * cos(self.g0) + s * sin(self.g0)) / t;
        let tt = pow(self.h / sqrt((1.0 + uu) / (1.0 - uu)), 1.0 / self.b);
        let phi = phi_of_t(tt, sqrt(self.e2));
        let lon = self.lon0
            - (atan2(
                s * cos(self.g0) - vv * sin(self.g0),
                cos(self.b * u / self.a),
            ) / self.b)
                .to_degrees();
        (phi.to_degrees(), lon)
    }
}

/// Orthographic (EPSG method 9840): the ellipsoid seen from infinitely far
/// above the origin, each point dropped straight onto the plane tangent there.
pub struct Orthographic {
    a: f64,
    e2: f64,
    lat0: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

impl Orthographic {
    pub fn new(a: f64, f: f64, lat0: f64, lon0: f64, fe: f64, fn_: f64) -> Orthographic {
        Orthographic {
            a,
            e2: f * (2.0 - f),
            lat0,
            lon0,
            fe,
            fn_,
        }
    }

    fn nu(&self, phi: f64) -> f64 {
        self.a / sqrt(1.0 - self.e2 * sin(phi) * sin(phi))
    }

    /// Easting and northing without the falsings, and the Jacobian's columns
    /// ∂(E, N)/∂φ and ∂(E, N)/∂λ (per radian).
    fn map(&self, phi: f64, dl: f64) -> ((f64, f64), (f64, f64), (f64, f64)) {
        let p0 = self.lat0.to_radians();
        let (s0, c0) = (sin(p0), cos(p0));
        let (s, c) = (sin(phi), cos(phi));
        let (sd, cd) = (sin(dl), cos(dl));
        let (nu, nu0) = (self.nu(phi), self.nu(p0));
        let rho = nu * (1.0 - self.e2) / (1.0 - self.e2 * s * s);
        let x = nu * c * sd;
        let y = nu * (s * c0 - c * s0 * cd) + self.e2 * (nu0 * s0 - nu * s) * c0;
        // d(ν cos φ)/dφ = −ρ sin φ and d(ν sin φ)/dφ = ρ cos φ / (1 − e²).
        let dphi = (-rho * s * sd, rho * (c0 * c + s0 * s * cd));
        let dlam = (nu * c * cd, nu * c * s0 * sd);
        ((x, y), dphi, dlam)
    }

    /// None for a point on the far side, whose vertical is 90° or more from
    /// the origin's: it would land on top of a point on the near side.
    pub fn forward(&self, lat: f64, lon: f64) -> Option<Grid> {
        if normal_dot(self.lat0, self.lon0, lat, lon) < -1e-12 {
            return None;
        }
        let phi = lat.to_radians();
        let ((x, y), dphi, dlam) = self.map(phi, dlon(lon, self.lon0).to_radians());
        let s = sin(phi);
        let w = 1.0 - self.e2 * s * s;
        let rho = self.a * (1.0 - self.e2) / (w * sqrt(w));
        let par = self.nu(phi) * cos(phi);
        Some(Grid {
            e: self.fe + x,
            n: self.fn_ + y,
            convergence: -atan2(dphi.0, dphi.1).to_degrees(),
            h: dphi.0.hypot(dphi.1) / rho,
            k: if par > 0.0 {
                dlam.0.hypot(dlam.1) / par
            } else {
                dphi.0.hypot(dphi.1) / rho
            },
        })
    }

    /// Newton's method in φ and λ from the sphere's answer; None off the
    /// disk the near side covers, or if it does not settle (on the rim, where
    /// the map folds). Convergence is quadratic, so one step after the update
    /// falls under 1e-12 radians the error is at the rounding of a double; a
    /// fixed 1e-15 test could stall there, a couple of ulps from π.
    pub fn inverse(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (x, y) = (x - self.fe, y - self.fn_);
        let p0 = self.lat0.to_radians();
        // The sphere of radius ν0 as the first guess.
        let r = self.nu(p0);
        let rr = x.hypot(y) / r;
        if rr > 1.0 + 1e-9 {
            return None;
        }
        let c = asin(rr.min(1.0));
        let (mut phi, mut dl) = if rr == 0.0 {
            (p0, 0.0)
        } else {
            (
                asin(cos(c) * sin(p0) + y * sin(c) * cos(p0) / (rr * r)),
                atan2(x * sin(c), rr * r * cos(p0) * cos(c) - y * sin(p0) * sin(c)),
            )
        };
        let mut last = false;
        for _ in 0..50 {
            let ((fx, fy), (ex, ey), (lx, ly)) = self.map(phi, dl);
            let (rx, ry) = (x - fx, y - fy);
            let det = ex * ly - lx * ey;
            if det == 0.0 {
                return None;
            }
            let dphi = (rx * ly - lx * ry) / det;
            let dlam = (ex * ry - rx * ey) / det;
            phi = (phi + dphi).clamp(-FRAC_PI_2, FRAC_PI_2);
            dl += dlam;
            if last {
                let lat = phi.to_degrees();
                let lon = wrap(self.lon0 + dl.to_degrees());
                return (normal_dot(self.lat0, self.lon0, lat, lon) >= -1e-12)
                    .then_some((lat, lon));
            }
            last = dphi.abs() < 1e-12 && dlam.abs() < 1e-12;
        }
        None
    }
}

/// The cosine of the angle between the verticals (ellipsoid normals) at two points.
fn normal_dot(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    sin(p1) * sin(p2) + cos(p1) * cos(p2) * cos((lon2 - lon1).to_radians())
}

/// The azimuthal projections built on the geodesic from their center
/// (Karney 2013, "Algorithms for geodesics", sections 8 and 9, as in
/// GeographicLib's AzimuthalEquidistant and Gnomonic classes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Azimuthal {
    /// Distances and directions from the center are true.
    Equidistant,
    /// Every geodesic through the center is a straight line (and, near the
    /// center, every geodesic is nearly one).
    Gnomonic,
}

pub struct AzimuthalProj {
    kind: Azimuthal,
    g: Geodesic,
    lat0: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
}

impl AzimuthalProj {
    pub fn new(
        kind: Azimuthal,
        a: f64,
        f: f64,
        lat0: f64,
        lon0: f64,
        fe: f64,
        fn_: f64,
    ) -> AzimuthalProj {
        AzimuthalProj {
            kind,
            g: Geodesic::new(a, f),
            lat0,
            lon0,
            fe,
            fn_,
        }
    }

    /// None for a gnomonic point a quarter of the way round the Earth or
    /// more from the center, where the geodesic scale M12 is not positive.
    pub fn forward(&self, lat: f64, lon: f64) -> Option<Grid> {
        let (s, azi1, azi2, m12, big_m12, _, _): (f64, f64, f64, f64, f64, f64, f64) =
            self.g.inverse(self.lat0, self.lon0, lat, lon);
        let (sa, ca) = (sin(azi1.to_radians()), cos(azi1.to_radians()));
        match self.kind {
            Azimuthal::Equidistant => {
                // Distance from the center is true (R = 1) and the scale
                // across the radius is s/m12. The radius reaches the point
                // heading azi2, so the meridian is azi2 off it and the
                // parallel 90 − azi2, each stretched by R along and T across.
                let t = if m12 > 0.0 && s > 0.0 { s / m12 } else { 1.0 };
                let (s2, c2) = (sin(azi2.to_radians()), cos(azi2.to_radians()));
                let north = atan2(-t * s2, c2).to_degrees();
                Some(Grid {
                    e: self.fe + s * sa,
                    n: self.fn_ + s * ca,
                    convergence: dlon(-(azi1 + north), 0.0),
                    h: c2.hypot(t * s2),
                    k: s2.hypot(t * c2),
                })
            }
            Azimuthal::Gnomonic => {
                // Less than a hemisphere: nothing whose vertical is 90° or
                // more from the center's (and, on the ellipsoid, nothing
                // where the geodesic scale is not positive).
                // A point at 90° to within rounding counts as on the horizon.
                if big_m12 <= 0.0 || normal_dot(self.lat0, self.lon0, lat, lon) < 1e-12 {
                    return None;
                }
                let rho = m12 / big_m12;
                let (e, n) = (self.fe + rho * sa, self.fn_ + rho * ca);
                // On the ellipsoid m12 and M12 depend on the geodesic's
                // azimuth as well as its length, so the images of the radius
                // and of the direction across it are not at right angles:
                // the factors come from the map itself.
                let (convergence, h, k) = factors(
                    |la, lo| {
                        let (_, az, _, m, big_m, _, _): (f64, f64, f64, f64, f64, f64, f64) =
                            self.g.inverse(self.lat0, self.lon0, la, lo);
                        let r = m / big_m;
                        (r * sin(az.to_radians()), r * cos(az.to_radians()))
                    },
                    &self.g,
                    lat,
                    lon,
                );
                Some(Grid {
                    e,
                    n,
                    convergence,
                    h,
                    k,
                })
            }
        }
    }

    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.fe, y - self.fn_);
        let azi0 = atan2(dx, dy).to_degrees();
        let rho = dx.hypot(dy);
        let (lat, lon) = match self.kind {
            Azimuthal::Equidistant => self.g.direct(self.lat0, self.lon0, azi0, rho),
            Azimuthal::Gnomonic => self.gnomonic_inverse(azi0, rho),
        };
        (lat, wrap(lon))
    }

    /// Newton's method on the distance s along the geodesic leaving the
    /// center at azi0, solving m12/M12 = ρ (or M12/m12 = 1/ρ far out), as
    /// GeographicLib's Gnomonic::Reverse does.
    fn gnomonic_inverse(&self, azi0: f64, rho: f64) -> (f64, f64) {
        let a = self.g.a;
        let little = rho <= a;
        let target = if little { rho } else { 1.0 / rho };
        let mut s = a * atan(rho / a);
        let mut trip = false;
        for _ in 0..10 {
            let (lat, lon, _, m, big_m, _): (f64, f64, f64, f64, f64, f64) =
                self.g.direct(self.lat0, self.lon0, azi0, s);
            if trip {
                return (lat, lon);
            }
            let ds = if little {
                (m - target * big_m) * big_m
            } else {
                (target * m - big_m) * m
            };
            s -= ds;
            // A NaN stops the loop too, as GeographicLib's reversed test does.
            if ds.abs() < 0.01 * f64::EPSILON.sqrt() * a || ds.is_nan() {
                trip = true;
            }
        }
        (f64::NAN, f64::NAN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;

    /// Guidance Note 7-2's worked examples for the methods it works.
    #[test]
    fn guidance_note_examples() {
        // 3.1.1.1 Lambert Conic Conformal (2SP), NAD27 / Texas South Central (Clarke 1866, US survey feet).
        let ftus = 1200.0 / 3937.0;
        let (a, f) = (6_378_206.400, 1.0 / 294.978_698_2);
        let l = Lcc::two_sp(
            a,
            f,
            27.0 + 50.0 / 60.0,
            -99.0,
            28.0 + 23.0 / 60.0,
            30.0 + 17.0 / 60.0,
            2_000_000.0 * ftus,
            0.0,
        );
        let g = l.forward(28.5, -96.0);
        assert!((g.e / ftus - 2_963_503.91).abs() < 0.01, "{}", g.e / ftus);
        assert!((g.n / ftus - 254_759.80).abs() < 0.01, "{}", g.n / ftus);
        // 3.1.1.2 Lambert Conic Conformal (1SP), JAD69 / Jamaica National Grid (Clarke 1866).
        let l = Lcc::one_sp(a, f, 18.0, -77.0, 1.0, 250_000.0, 150_000.0);
        let g = l.forward(
            17.0 + 55.0 / 60.0 + 55.80 / 3600.0,
            -(76.0 + 56.0 / 60.0 + 37.26 / 3600.0),
        );
        assert!(
            (g.e - 255_966.58).abs() < 0.01 && (g.n - 142_493.51).abs() < 0.01,
            "{g:?}"
        );
        // 3.2.2 Polar Stereographic variant A, WGS 84 / UPS North.
        let p = PolarStereo::variant_a(A, F, true, 0.0, 0.994, 2_000_000.0, 2_000_000.0);
        let g = p.forward(73.0, 44.0);
        assert!(
            (g.e - 3_320_416.75).abs() < 0.01 && (g.n - 632_668.43).abs() < 0.01,
            "{g:?}"
        );
        // Variant B, WGS 84 / Australian Antarctic Polar Stereographic.
        let p = PolarStereo::variant_b(A, F, -71.0, 70.0, 6_000_000.0, 6_000_000.0);
        let g = p.forward(-75.0, 120.0);
        assert!(
            (g.e - 7_255_380.79).abs() < 0.01 && (g.n - 7_053_389.56).abs() < 0.01,
            "{g:?}"
        );
        // Pseudo Mercator, WGS 84 / Pseudo-Mercator.
        let g = WebMercator::forward(24.0 + 22.0 / 60.0 + 54.433 / 3600.0, -(100.0 + 20.0 / 60.0));
        assert!(
            (g.e + 11_169_055.58).abs() < 0.01 && (g.n - 2_800_000.0).abs() < 0.01,
            "{g:?}"
        );
    }

    #[test]
    fn round_trips() {
        let lcc = Lcc::two_sp(A, F, 23.0, -96.0, 29.5, 45.5, 0.0, 0.0);
        let alb = Albers::new(A, F, 23.0, -96.0, 29.5, 45.5, 0.0, 0.0);
        let ps = PolarStereo::variant_b(A, F, -71.0, 0.0, 0.0, 0.0);
        let eqc = EquidistantCylindrical::new(A, F, 30.0, 10.0, 0.0, 0.0);
        for lat in [-60.0, -10.0, 0.0, 15.5, 40.0, 70.0] {
            for lon in [-170.0, -96.0, -20.0, 0.0, 45.0, 179.0] {
                let back = |g: Grid, inv: &dyn Fn(f64, f64) -> (f64, f64)| {
                    let (la, lo) = inv(g.e, g.n);
                    assert!(
                        (la - lat).abs() < 1e-11 && dlon(lo, lon).abs() < 1e-11,
                        "{lat} {lon} -> {la} {lo}"
                    );
                };
                if lat > 0.0 {
                    back(lcc.forward(lat, lon), &|x, y| lcc.inverse(x, y));
                }
                back(alb.forward(lat, lon), &|x, y| alb.inverse(x, y));
                if lat < 0.0 {
                    back(ps.forward(lat, lon), &|x, y| ps.inverse(x, y));
                }
                back(eqc.forward(lat, lon), &|x, y| eqc.inverse(x, y));
                back(WebMercator::forward(lat, lon), &|x, y| {
                    WebMercator::inverse(x, y)
                });
            }
        }
    }
}
