//! Ellipsoid geometry and frame conversions (geodesy/reference-frames):
//! derived parameters, radii of curvature, meridian arcs, auxiliary
//! latitudes, geodetic ↔ ECEF, and local tangent planes (ENU, NED, AER).
//!
//! Meridian arcs use the exact elliptic-integral form with Carlson's
//! symmetric integrals, so they hold for any flattening. The ECEF inverse is
//! Vermeille's (2011) closed form as implemented in GeographicLib's
//! Geocentric class: non-iterative and accurate at every height.

use libm::{atan, atan2, atanh, cbrt, cos, hypot, sin, sqrt, tan};

use crate::ellipsoid::Ellipsoid;
use crate::tm::{tauf, taupf};

const DEG: f64 = core::f64::consts::PI / 180.0;

/// Derived ellipsoid parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Derived {
    pub b: f64,
    pub e2: f64,
    pub ep2: f64,
    /// Third flattening n = (a − b)/(a + b).
    pub n: f64,
    /// Mean radius R1 = (2a + b)/3.
    pub r1: f64,
    /// Authalic radius: the sphere of equal area.
    pub r2: f64,
    /// Volumetric radius: the sphere of equal volume.
    pub r3: f64,
}

/// atanh(e·x)/e, with its limit x at e = 0.
fn atanhee(x: f64, e: f64) -> f64 {
    if e == 0.0 { x } else { atanh(e * x) / e }
}

impl Ellipsoid {
    pub fn b(&self) -> f64 {
        self.a * (1.0 - self.f)
    }
    pub fn e2(&self) -> f64 {
        self.f * (2.0 - self.f)
    }
    pub fn derived(&self) -> Derived {
        let (a, b, e2) = (self.a, self.b(), self.e2());
        let e = sqrt(e2);
        Derived {
            b,
            e2,
            ep2: e2 / (1.0 - e2),
            n: self.f / (2.0 - self.f),
            r1: (2.0 * a + b) / 3.0,
            // c² = a²/2 + b²/2 · atanh(e)/e.
            r2: sqrt(a * a / 2.0 + b * b / 2.0 * atanhee(1.0, e)),
            r3: cbrt(a * a * b),
        }
    }

    /// W = √(1 − e² sin²φ).
    fn w(&self, phi: f64) -> f64 {
        let s = sin(phi);
        sqrt(1.0 - self.e2() * s * s)
    }

    /// Meridional radius of curvature M at geodetic latitude φ (radians).
    pub fn meridional_radius(&self, phi: f64) -> f64 {
        let w = self.w(phi);
        self.a * (1.0 - self.e2()) / (w * w * w)
    }

    /// Prime-vertical radius of curvature N.
    pub fn prime_vertical_radius(&self, phi: f64) -> f64 {
        self.a / self.w(phi)
    }

    /// Radius of curvature of the normal section in azimuth α (Euler).
    pub fn radius_in_azimuth(&self, phi: f64, alpha: f64) -> f64 {
        let (m, n) = (self.meridional_radius(phi), self.prime_vertical_radius(phi));
        let (c, s) = (cos(alpha), sin(alpha));
        1.0 / (c * c / m + s * s / n)
    }

    /// Meridian arc from the equator to φ: a[E(φ, e) − e² sinφ cosφ / W].
    pub fn meridian_arc(&self, phi: f64) -> f64 {
        let e2 = self.e2();
        let (s, c) = (sin(phi), cos(phi));
        self.a * (ellip_e(s, c, e2) - e2 * s * c / self.w(phi))
    }

    /// The quarter meridian, equator to pole.
    pub fn quarter_meridian(&self) -> f64 {
        self.meridian_arc(90.0 * DEG)
    }
}

/// Carlson's RF by the duplication method, with the 7th-order series and
/// tolerance of GeographicLib's EllipticFunction::RF.
fn rf(x: f64, y: f64, z: f64) -> f64 {
    let tol = (3.0 * f64::EPSILON * 0.01).powf(1.0 / 8.0);
    let a0 = (x + y + z) / 3.0;
    let q = (a0 - x).abs().max((a0 - y).abs()).max((a0 - z).abs()) / tol;
    let (mut an, mut x0, mut y0, mut z0, mut mul) = (a0, x, y, z, 1.0);
    while q >= mul * an.abs() {
        let lam = sqrt(x0) * sqrt(y0) + sqrt(y0) * sqrt(z0) + sqrt(z0) * sqrt(x0);
        an = (an + lam) / 4.0;
        x0 = (x0 + lam) / 4.0;
        y0 = (y0 + lam) / 4.0;
        z0 = (z0 + lam) / 4.0;
        mul *= 4.0;
    }
    let xx = (a0 - x) / (mul * an);
    let yy = (a0 - y) / (mul * an);
    let zz = -(xx + yy);
    let e2 = xx * yy - zz * zz;
    let e3 = xx * yy * zz;
    (e3 * (6930.0 * e3 + e2 * (15015.0 * e2 - 16380.0) + 17160.0)
        + e2 * ((10010.0 - 5775.0 * e2) * e2 - 24024.0)
        + 240240.0)
        / (240240.0 * sqrt(an))
}

/// Carlson's RD by the duplication method, as GeographicLib's EllipticFunction::RD.
fn rd(x: f64, y: f64, z: f64) -> f64 {
    let tol = (0.2 * (f64::EPSILON * 0.01)).powf(1.0 / 8.0);
    let a0 = (x + y + 3.0 * z) / 5.0;
    let q = (a0 - x).abs().max((a0 - y).abs()).max((a0 - z).abs()) / tol;
    let (mut an, mut x0, mut y0, mut z0, mut mul, mut s) = (a0, x, y, z, 1.0, 0.0);
    while q >= mul * an.abs() {
        let lam = sqrt(x0) * sqrt(y0) + sqrt(y0) * sqrt(z0) + sqrt(z0) * sqrt(x0);
        s += 1.0 / (mul * sqrt(z0) * (z0 + lam));
        an = (an + lam) / 4.0;
        x0 = (x0 + lam) / 4.0;
        y0 = (y0 + lam) / 4.0;
        z0 = (z0 + lam) / 4.0;
        mul *= 4.0;
    }
    let xx = (a0 - x) / (mul * an);
    let yy = (a0 - y) / (mul * an);
    let zz = -(xx + yy) / 3.0;
    let e2 = xx * yy - 6.0 * zz * zz;
    let e3 = (3.0 * xx * yy - 8.0 * zz * zz) * zz;
    let e4 = 3.0 * (xx * yy - zz * zz) * zz * zz;
    let e5 = xx * yy * zz * zz * zz;
    ((471240.0 - 540540.0 * e2) * e5
        + (612612.0 * e2 - 540540.0 * e3 - 556920.0) * e4
        + e3 * (306306.0 * e3 + e2 * (675675.0 * e2 - 706860.0) + 680680.0)
        + e2 * ((417690.0 - 255255.0 * e2) * e2 - 875160.0)
        + 4084080.0)
        / (4084080.0 * mul * an * sqrt(an))
        + 3.0 * s
}

/// Incomplete elliptic integral of the second kind E(φ, k) with k² = `k2`,
/// from sin φ and cos φ: s·RF(c², 1 − k²s², 1) − k²s³/3 · RD(c², 1 − k²s², 1).
fn ellip_e(s: f64, c: f64, k2: f64) -> f64 {
    if s == 0.0 {
        return 0.0;
    }
    let (c2, d2) = (c * c, 1.0 - k2 * s * s);
    let f = s * rf(c2, d2, 1.0);
    if k2 == 0.0 {
        return f;
    }
    f - k2 * s * s * s / 3.0 * rd(c2, d2, 1.0)
}

/// The auxiliary latitudes of geodetic latitude φ (radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Auxiliary {
    pub geocentric: f64,
    pub parametric: f64,
    pub rectifying: f64,
    pub conformal: f64,
    pub authalic: f64,
    /// Isometric latitude ψ, in radians (unbounded).
    pub isometric: f64,
}

/// Which auxiliary latitude a value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aux {
    Geodetic,
    Geocentric,
    Parametric,
    Rectifying,
    Conformal,
    Authalic,
    Isometric,
}

impl Ellipsoid {
    /// qp − q(φ) for φ ≥ 0, without cancellation near the pole:
    /// (1 − s)(1 + e²s)/(1 − e²s²) + (1 − e²)·atanh(e(1 − s)/(1 − e²s))/e.
    fn q_gap(&self, s: f64, c: f64) -> f64 {
        let e2 = self.e2();
        let e = sqrt(e2);
        let one_minus_s = c * c / (1.0 + s);
        one_minus_s * (1.0 + e2 * s) / (1.0 - e2 * s * s)
            + (1.0 - e2) * atanhee(one_minus_s / (1.0 - e2 * s), e)
    }

    /// q(φ) = (1 − e²)[s/(1 − e²s²) + atanh(e s)/e].
    fn q(&self, s: f64) -> f64 {
        let e2 = self.e2();
        (1.0 - e2) * (s / (1.0 - e2 * s * s) + atanhee(s, sqrt(e2)))
    }

    /// Authalic latitude from geodetic, accurate to the pole.
    fn authalic(&self, phi: f64) -> f64 {
        let (s, c) = (sin(phi.abs()), cos(phi));
        let qp = self.q(1.0);
        let q = self.q(s);
        let gap = self.q_gap(s, c);
        atan2(q, sqrt(gap * (qp + q))).copysign(phi)
    }

    /// Geodetic latitude from authalic ξ by Newton's method on qp − q(φ).
    fn geodetic_from_authalic(&self, xi: f64) -> f64 {
        let e2 = self.e2();
        if e2 == 0.0 {
            return xi;
        }
        let (sx, cx) = (sin(xi.abs()), cos(xi));
        let qp = self.q(1.0);
        // Target gap: qp(1 − sin ξ).
        let want = qp * cx * cx / (1.0 + sx);
        let mut phi = xi.abs();
        for _ in 0..8 {
            let (s, c) = (sin(phi), cos(phi));
            let gap = self.q_gap(s, c);
            let w2 = 1.0 - e2 * s * s;
            let dq = 2.0 * (1.0 - e2) * c / (w2 * w2);
            if dq == 0.0 {
                break;
            }
            let step = (gap - want) / dq;
            phi = (phi + step).min(90.0 * DEG);
            if step.abs() < 1e-17 {
                break;
            }
        }
        phi.copysign(xi)
    }

    /// Geodetic latitude from rectifying μ by Newton's method on the meridian arc.
    fn geodetic_from_rectifying(&self, mu: f64) -> f64 {
        let target = mu / (90.0 * DEG) * self.quarter_meridian();
        let mut phi = mu;
        for _ in 0..8 {
            let step = (target - self.meridian_arc(phi)) / self.meridional_radius(phi);
            phi += step;
            if step.abs() < 1e-17 {
                break;
            }
        }
        phi.clamp(-90.0 * DEG, 90.0 * DEG)
    }

    /// Every auxiliary latitude of geodetic latitude φ (radians).
    pub fn auxiliary(&self, phi: f64) -> Auxiliary {
        let (s, c) = (sin(phi), cos(phi));
        let e2 = self.e2();
        let es = sqrt(e2);
        let taup = if c == 0.0 {
            f64::INFINITY.copysign(s)
        } else {
            taupf(s / c, es)
        };
        Auxiliary {
            geocentric: atan2((1.0 - e2) * s, c),
            parametric: atan2((1.0 - self.f) * s, c),
            rectifying: 90.0 * DEG * self.meridian_arc(phi) / self.quarter_meridian(),
            conformal: atan(taup),
            authalic: self.authalic(phi),
            isometric: libm::asinh(taup),
        }
    }

    /// Geodetic latitude (radians) from an auxiliary latitude of kind `from`.
    pub fn geodetic_from(&self, from: Aux, x: f64) -> f64 {
        let e2 = self.e2();
        let es = sqrt(e2);
        match from {
            Aux::Geodetic => x,
            Aux::Geocentric => atan2(sin(x), (1.0 - e2) * cos(x)),
            Aux::Parametric => atan2(sin(x), (1.0 - self.f) * cos(x)),
            Aux::Rectifying => self.geodetic_from_rectifying(x),
            Aux::Conformal => {
                let c = cos(x);
                if c <= 0.0 { x } else { atan(tauf(tan(x), es)) }
            }
            Aux::Authalic => self.geodetic_from_authalic(x),
            Aux::Isometric => atan(tauf(libm::sinh(x), es)),
        }
    }
}

/// Geodetic (φ, λ in radians, h in meters) to ECEF (X, Y, Z) meters.
pub fn to_ecef(ell: &Ellipsoid, phi: f64, lam: f64, h: f64) -> (f64, f64, f64) {
    let e2 = ell.e2();
    let n = ell.prime_vertical_radius(phi);
    let (sp, cp) = (sin(phi), cos(phi));
    let (sl, cl) = (sin(lam), cos(lam));
    (
        (n + h) * cp * cl,
        (n + h) * cp * sl,
        (n * (1.0 - e2) + h) * sp,
    )
}

/// ECEF to geodetic (φ, λ radians, h meters) by Vermeille's closed form, as
/// in GeographicLib's Geocentric::IntReverse. `None` when (X, Y) and Z are
/// all zero (the Earth's center), where latitude is undefined.
pub fn from_ecef(ell: &Ellipsoid, x: f64, y: f64, z: f64) -> Option<(f64, f64, f64)> {
    let a = ell.a;
    let f = ell.f;
    let e2 = ell.e2();
    let e2m = (1.0 - f) * (1.0 - f);
    let e2a = e2.abs();
    let e4a = e2 * e2;
    let r = hypot(x, y);
    if r == 0.0 && z == 0.0 {
        return None;
    }
    let (slam, clam) = if r != 0.0 { (y / r, x / r) } else { (0.0, 1.0) };
    let (sphi, cphi, h);
    if e4a == 0.0 {
        // A sphere.
        let hh = hypot(r, z);
        sphi = z / hh;
        cphi = r / hh;
        h = hh - a;
    } else {
        let mut p = (r / a) * (r / a);
        let mut q = e2m * (z / a) * (z / a);
        let rr = (p + q - e4a) / 6.0;
        if f < 0.0 {
            core::mem::swap(&mut p, &mut q);
        }
        if !(e4a * q == 0.0 && rr <= 0.0) {
            let s = e4a * p * q / 4.0;
            let r2 = rr * rr;
            let r3 = rr * r2;
            let disc = s * (2.0 * r3 + s);
            let mut u = rr;
            if disc >= 0.0 {
                let mut t3 = s + r3;
                t3 += if t3 < 0.0 { -sqrt(disc) } else { sqrt(disc) };
                let t = cbrt(t3);
                u += t + if t != 0.0 { r2 / t } else { 0.0 };
            } else {
                let ang = atan2(sqrt(-disc), -(s + r3));
                u += 2.0 * rr * cos(ang / 3.0);
            }
            let v = sqrt(u * u + e4a * q);
            let uv = if u < 0.0 { e4a * q / (v - u) } else { u + v };
            let w = (e2a * (uv - q) / (2.0 * v)).max(0.0);
            let k = uv / (sqrt(uv + w * w) + w);
            let k1 = if f >= 0.0 { k } else { k - e2 };
            let k2 = if f >= 0.0 { k + e2 } else { k };
            let d = k1 * r / k2;
            let hh = hypot(z / k1, r / k2);
            sphi = (z / k1) / hh;
            cphi = (r / k2) / hh;
            h = (1.0 - e2m / k1) * hypot(d, z);
        } else {
            // On the equatorial plane inside the evolute.
            let zz = sqrt((if f >= 0.0 { e4a - p } else { p }) / e2m);
            let xx = sqrt(if f < 0.0 { e4a - p } else { p });
            let hh = hypot(zz, xx);
            sphi = if z < 0.0 { -zz / hh } else { zz / hh };
            cphi = xx / hh;
            h = -a * (if f >= 0.0 { e2m } else { 1.0 }) * hh / e2a;
        }
    }
    Some((atan2(sphi, cphi), atan2(slam, clam), h))
}

/// The ECEF → ENU rotation at (φ, λ): rows are the east, north, and up unit vectors.
pub fn enu_rotation(phi: f64, lam: f64) -> [[f64; 3]; 3] {
    let (sp, cp) = (sin(phi), cos(phi));
    let (sl, cl) = (sin(lam), cos(lam));
    [
        [-sl, cl, 0.0],
        [-sp * cl, -sp * sl, cp],
        [cp * cl, cp * sl, sp],
    ]
}

/// ENU of an ECEF point relative to an ECEF origin whose geodetic position is (φ0, λ0).
pub fn ecef_to_enu(origin: (f64, f64, f64), phi0: f64, lam0: f64, p: (f64, f64, f64)) -> [f64; 3] {
    let r = enu_rotation(phi0, lam0);
    let d = [p.0 - origin.0, p.1 - origin.1, p.2 - origin.2];
    [0, 1, 2].map(|i| r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2])
}

/// The ECEF point at ENU offset `enu` from the origin.
pub fn enu_to_ecef(
    origin: (f64, f64, f64),
    phi0: f64,
    lam0: f64,
    enu: [f64; 3],
) -> (f64, f64, f64) {
    let r = enu_rotation(phi0, lam0);
    let c = [0, 1, 2].map(|j| r[0][j] * enu[0] + r[1][j] * enu[1] + r[2][j] * enu[2]);
    (origin.0 + c[0], origin.1 + c[1], origin.2 + c[2])
}

/// Azimuth (radians, clockwise from north), elevation (radians), and range from ENU.
pub fn enu_to_aer(enu: [f64; 3]) -> (f64, f64, f64) {
    let horiz = hypot(enu[0], enu[1]);
    (
        atan2(enu[0], enu[1]),
        atan2(enu[2], horiz),
        hypot(horiz, enu[2]),
    )
}

/// ENU from azimuth, elevation (radians), and range.
pub fn aer_to_enu(az: f64, el: f64, range: f64) -> [f64; 3] {
    let horiz = range * cos(el);
    [horiz * sin(az), horiz * cos(az), range * sin(el)]
}
