//! The exact transverse Mercator (Lee 1976), ported from GeographicLib's
//! TransverseMercatorExact and the parts of its EllipticFunction it needs
//! (Karney 2011, "Transverse Mercator with an accuracy of a few nanometers",
//! section 4). It holds to double precision over the whole ellipsoid, where
//! the Krüger series in `tm` loses accuracy past 3,900 km from the central
//! meridian.

use crate::tm::{tauf, taupf};
use core::f64::consts::{FRAC_PI_2, PI};
use libm::{asinh, atan, atan2, cbrt, cos, cosh, hypot, sin, sinh, sqrt, tan, tanh};

/// Carlson's symmetric integrals and Jacobi's elliptic functions for one
/// parameter k² (GeographicLib's EllipticFunction with α² = 0).
struct Elliptic {
    k2: f64,
    kp2: f64,
    /// Complete integrals K and E.
    kc: f64,
    ec: f64,
}

fn rf(x: f64, y: f64, z: f64) -> f64 {
    // Carlson, eqs 2.2 to 2.7.
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

fn rf2(x: f64, y: f64) -> f64 {
    // Carlson, eqs 2.36 to 2.38.
    let tol = 2.7 * sqrt(f64::EPSILON * 0.01);
    let (mut xn, mut yn) = (sqrt(x), sqrt(y));
    if xn < yn {
        core::mem::swap(&mut xn, &mut yn);
    }
    while (xn - yn).abs() > tol * xn {
        let t = (xn + yn) / 2.0;
        yn = sqrt(xn * yn);
        xn = t;
    }
    PI / (xn + yn)
}

fn rd(x: f64, y: f64, z: f64) -> f64 {
    // Carlson, eqs 2.28 to 2.34.
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

fn rg2(x: f64, y: f64) -> f64 {
    // Carlson, eqs 2.36 to 2.39.
    let tol = 2.7 * sqrt(f64::EPSILON * 0.01);
    let (x0, y0) = (sqrt(x.max(y)), sqrt(x.min(y)));
    let (mut xn, mut yn, mut s, mut mul) = (x0, y0, 0.0, 0.25);
    while (xn - yn).abs() > tol * xn {
        let t = (xn + yn) / 2.0;
        yn = sqrt(xn * yn);
        xn = t;
        mul *= 2.0;
        let d = xn - yn;
        s += mul * d * d;
    }
    (((x0 + y0) / 2.0).powi(2) - s) * PI / (2.0 * (xn + yn))
}

impl Elliptic {
    fn new(k2: f64) -> Elliptic {
        let kp2 = 1.0 - k2;
        Elliptic {
            k2,
            kp2,
            kc: if kp2 != 0.0 {
                rf2(kp2, 1.0)
            } else {
                f64::INFINITY
            },
            ec: if kp2 != 0.0 { 2.0 * rg2(kp2, 1.0) } else { 1.0 },
        }
    }

    /// K − E.
    fn ke(&self) -> f64 {
        self.kc - self.ec
    }

    /// Jacobi's sn, cn, dn by Bulirsch's method (1965, p. 89).
    fn sncndn(&self, x: f64) -> (f64, f64, f64) {
        if self.kp2 == 0.0 {
            let c = 1.0 / cosh(x);
            return (tanh(x), c, c);
        }
        let tol = sqrt(f64::EPSILON * 0.01);
        let mut mc = self.kp2;
        let (mut m, mut n) = ([0.0f64; 13], [0.0f64; 13]);
        let mut l = 0usize;
        let mut c = 0.0;
        let mut a = 1.0;
        while l < 13 {
            m[l] = a;
            mc = sqrt(mc);
            n[l] = mc;
            c = (a + mc) / 2.0;
            if (a - mc).abs() <= tol * a {
                l += 1;
                break;
            }
            mc *= a;
            a = c;
            l += 1;
        }
        let x = x * c;
        let (mut sn, mut cn, mut dn) = (sin(x), cos(x), 1.0);
        if sn != 0.0 {
            let mut a = cn / sn;
            c *= a;
            while l > 0 {
                l -= 1;
                let b = m[l];
                a *= c;
                c *= dn;
                dn = (n[l] + a) / (b + a);
                a = c / b;
            }
            let a = 1.0 / sqrt(c * c + 1.0);
            sn = if sn.is_sign_negative() { -a } else { a };
            cn = c * sn;
        }
        (sn, cn, dn)
    }

    /// The incomplete integral of the second kind in terms of sn, cn, dn.
    fn e_of(&self, sn: f64, cn: f64, dn: f64) -> f64 {
        let (cn2, dn2, sn2) = (cn * cn, dn * dn, sn * sn);
        let mut ei = if cn2 != 0.0 {
            sn.abs()
                * if self.k2 <= 0.0 {
                    rf(cn2, dn2, 1.0) - self.k2 * sn2 * rd(cn2, dn2, 1.0) / 3.0
                } else if self.kp2 >= 0.0 {
                    self.kp2 * rf(cn2, dn2, 1.0)
                        + self.k2 * self.kp2 * sn2 * rd(cn2, 1.0, dn2) / 3.0
                        + self.k2 * cn.abs() / dn
                } else {
                    -self.kp2 * sn2 * rd(dn2, 1.0, cn2) / 3.0 + dn / cn.abs()
                }
        } else {
            self.ec
        };
        if cn.is_sign_negative() {
            ei = 2.0 * self.ec - ei;
        }
        ei.copysign(sn)
    }
}

/// Wraps a longitude difference into [-180, 180].
fn ang_diff(lon0: f64, lon: f64) -> f64 {
    let d = (lon - lon0) % 360.0;
    if d > 180.0 {
        d - 360.0
    } else if d <= -180.0 {
        d + 360.0
    } else {
        d
    }
}

pub struct TmExact {
    a: f64,
    k0: f64,
    mu: f64,
    mv: f64,
    e: f64,
    eu: Elliptic,
    ev: Elliptic,
}

const NUMIT: usize = 10;

impl TmExact {
    pub fn new(a: f64, f: f64, k0: f64) -> TmExact {
        let mu = f * (2.0 - f);
        TmExact {
            a,
            k0,
            mu,
            mv: 1.0 - mu,
            e: sqrt(mu),
            eu: Elliptic::new(mu),
            ev: Elliptic::new(1.0 - mu),
        }
    }

    fn tol2() -> f64 {
        0.1 * f64::EPSILON
    }

    fn taytol() -> f64 {
        f64::EPSILON.powf(0.6)
    }

    /// Lee 54.17: (u, v) to (τ′, λ).
    #[allow(clippy::too_many_arguments)]
    fn zeta(&self, snu: f64, cnu: f64, dnu: f64, snv: f64, cnv: f64, dnv: f64) -> (f64, f64) {
        let overflow = 1.0 / (f64::EPSILON * f64::EPSILON);
        let d1 = sqrt(cnu * cnu + self.mv * (snu * snv).powi(2));
        let d2 = sqrt(self.mu * cnu * cnu + self.mv * cnv * cnv);
        let big = |s: f64| {
            if s.is_sign_negative() {
                -overflow
            } else {
                overflow
            }
        };
        let t1 = if d1 != 0.0 { snu * dnv / d1 } else { big(snu) };
        let t2 = if d2 != 0.0 {
            sinh(self.e * asinh(self.e * snu / d2))
        } else {
            big(snu)
        };
        let taup = t1 * hypot(1.0, t2) - t2 * hypot(1.0, t1);
        let lam = if d1 != 0.0 && d2 != 0.0 {
            atan2(dnu * snv, cnu * cnv) - self.e * atan2(self.e * cnu * snv, dnu * cnv)
        } else {
            0.0
        };
        (taup, lam)
    }

    /// Lee 54.21: the reciprocal derivative dw/dζ.
    fn dwdzeta(&self, snu: f64, cnu: f64, dnu: f64, snv: f64, cnv: f64, dnv: f64) -> (f64, f64) {
        let d = self.mv * (cnv * cnv + self.mu * (snu * snv).powi(2)).powi(2);
        (
            cnu * dnu * dnv * (cnv * cnv - self.mu * (snu * snv).powi(2)) / d,
            -snu * snv * cnv * ((dnu * dnv).powi(2) + self.mu * cnu * cnu) / d,
        )
    }

    /// A starting point for inverting ζ; true when it is already exact.
    fn zetainv0(&self, psi: f64, lam: f64) -> (f64, f64, bool) {
        let e = self.e;
        if psi < -e * PI / 4.0
            && lam > (1.0 - 2.0 * e) * FRAC_PI_2
            && psi < lam - (1.0 - e) * FRAC_PI_2
        {
            let psix = 1.0 - psi / e;
            let lamx = (FRAC_PI_2 - lam) / e;
            let u = asinh(sin(lamx) / hypot(cos(lamx), sinh(psix))) * (1.0 + self.mu / 2.0);
            let v = atan2(cos(lamx), sinh(psix)) * (1.0 + self.mu / 2.0);
            (self.eu.kc - u, self.ev.kc - v, false)
        } else if psi < e * FRAC_PI_2 && lam > (1.0 - 2.0 * e) * FRAC_PI_2 {
            let dlam = lam - (1.0 - e) * FRAC_PI_2;
            let rad = hypot(psi, dlam);
            let ang = atan2(dlam - psi, psi + dlam) - 0.75 * PI;
            let done = rad < e * Self::taytol();
            let rad = cbrt(3.0 / (self.mv * e) * rad);
            let ang = ang / 3.0;
            (rad * cos(ang), rad * sin(ang) + self.ev.kc, done)
        } else {
            let v = asinh(sin(lam) / hypot(cos(lam), sinh(psi)));
            let u = atan2(sinh(psi), cos(lam));
            let s = self.eu.kc / FRAC_PI_2;
            (u * s, v * s, false)
        }
    }

    /// (τ′, λ) to (u, v) by Newton's method.
    fn zetainv(&self, taup: f64, lam: f64) -> (f64, f64) {
        let psi = asinh(taup);
        let scal = 1.0 / hypot(1.0, taup);
        let (mut u, mut v, done) = self.zetainv0(psi, lam);
        if done {
            return (u, v);
        }
        let stol2 = Self::tol2() / psi.max(1.0).powi(2);
        let mut trip = false;
        for _ in 0..NUMIT {
            let (snu, cnu, dnu) = self.eu.sncndn(u);
            let (snv, cnv, dnv) = self.ev.sncndn(v);
            let (tau1, lam1) = self.zeta(snu, cnu, dnu, snv, cnv, dnv);
            let (du1, dv1) = self.dwdzeta(snu, cnu, dnu, snv, cnv, dnv);
            let (t1, l1) = ((tau1 - taup) * scal, lam1 - lam);
            let delu = t1 * du1 - l1 * dv1;
            let delv = t1 * dv1 + l1 * du1;
            u -= delu;
            v -= delv;
            if trip {
                break;
            }
            let delw2 = delu * delu + delv * delv;
            // A NaN stops the loop too, as GeographicLib's reversed test does.
            if delw2 < stol2 || delw2.is_nan() {
                trip = true;
            }
        }
        (u, v)
    }

    /// Lee 55.4: (u, v) to (ξ, η).
    #[allow(clippy::too_many_arguments)]
    fn sigma(
        &self,
        snu: f64,
        cnu: f64,
        dnu: f64,
        v: f64,
        snv: f64,
        cnv: f64,
        dnv: f64,
    ) -> (f64, f64) {
        let d = self.mu * cnu * cnu + self.mv * cnv * cnv;
        (
            self.eu.e_of(snu, cnu, dnu) - self.mu * snu * cnu * dnu / d,
            v - self.ev.e_of(snv, cnv, dnv) + self.mv * snv * cnv * dnv / d,
        )
    }

    /// The reciprocal of Lee 55.9.
    fn dwdsigma(&self, snu: f64, cnu: f64, dnu: f64, snv: f64, cnv: f64, dnv: f64) -> (f64, f64) {
        let d = self.mv * (cnv * cnv + self.mu * (snu * snv).powi(2)).powi(2);
        let dnr = dnu * cnv * dnv;
        let dni = -self.mu * snu * cnu * snv;
        ((dnr * dnr - dni * dni) / d, 2.0 * dnr * dni / d)
    }

    fn sigmainv0(&self, xi: f64, eta: f64) -> (f64, f64, bool) {
        let (eue, evke) = (self.eu.ec, self.ev.ke());
        if eta > 1.25 * evke || (xi < -0.25 * eue && xi < eta - evke) {
            let x = xi - eue;
            let y = eta - evke;
            let r2 = x * x + y * y;
            (self.eu.kc + x / r2, self.ev.kc - y / r2, false)
        } else if (eta > 0.75 * evke && xi < 0.25 * eue) || eta > evke {
            let deta = eta - evke;
            let rad = hypot(xi, deta);
            let ang = atan2(deta - xi, xi + deta) - 0.75 * PI;
            let done = rad < 2.0 * Self::taytol();
            let rad = cbrt(3.0 / self.mv * rad);
            let ang = ang / 3.0;
            (rad * cos(ang), rad * sin(ang) + self.ev.kc, done)
        } else {
            let s = self.eu.kc / self.eu.ec;
            (xi * s, eta * s, false)
        }
    }

    fn sigmainv(&self, xi: f64, eta: f64) -> (f64, f64) {
        let (mut u, mut v, done) = self.sigmainv0(xi, eta);
        if done {
            return (u, v);
        }
        let mut trip = false;
        for _ in 0..NUMIT {
            let (snu, cnu, dnu) = self.eu.sncndn(u);
            let (snv, cnv, dnv) = self.ev.sncndn(v);
            let (xi1, eta1) = self.sigma(snu, cnu, dnu, v, snv, cnv, dnv);
            let (du1, dv1) = self.dwdsigma(snu, cnu, dnu, snv, cnv, dnv);
            let (x1, e1) = (xi1 - xi, eta1 - eta);
            let delu = x1 * du1 - e1 * dv1;
            let delv = x1 * dv1 + e1 * du1;
            u -= delu;
            v -= delv;
            if trip {
                break;
            }
            let delw2 = delu * delu + delv * delv;
            if delw2 < Self::tol2() || delw2.is_nan() {
                trip = true;
            }
        }
        (u, v)
    }

    /// Lee 55.12 and 55.13: convergence (radians) and scale.
    #[allow(clippy::too_many_arguments)]
    fn scale(
        &self,
        tau: f64,
        snu: f64,
        cnu: f64,
        dnu: f64,
        snv: f64,
        cnv: f64,
        dnv: f64,
    ) -> (f64, f64) {
        let sec2 = 1.0 + tau * tau;
        let gamma = atan2(self.mv * snu * snv * cnv, cnu * dnu * dnv);
        let k = sqrt(self.mv + self.mu / sec2)
            * sqrt(sec2)
            * sqrt(
                (self.mv * snv * snv + (cnu * dnv).powi(2))
                    / (self.mu * cnu * cnu + self.mv * cnv * cnv),
            );
        (gamma, k)
    }

    /// (x, y, convergence in degrees, scale) for a point, relative to the
    /// central meridian `lon0` and the equator.
    pub fn forward(&self, lon0: f64, lat: f64, lon: f64) -> (f64, f64, f64, f64) {
        let mut lon = ang_diff(lon0, lon);
        let mut latsign = if lat.is_sign_negative() { -1.0 } else { 1.0 };
        let lonsign = if lon.is_sign_negative() { -1.0 } else { 1.0 };
        lon *= lonsign;
        let lat = lat * latsign;
        let backside = lon > 90.0;
        if backside {
            if lat == 0.0 {
                latsign = -1.0;
            }
            lon = 180.0 - lon;
        }
        let lam = lon.to_radians();
        let tau = tan(lat.to_radians());
        let (u, v) = if lat == 90.0 {
            (self.eu.kc, 0.0)
        } else if lat == 0.0 && lon == 90.0 * (1.0 - self.e) {
            (0.0, self.ev.kc)
        } else {
            self.zetainv(taupf(tau, self.e), lam)
        };
        let (snu, cnu, dnu) = self.eu.sncndn(u);
        let (snv, cnv, dnv) = self.ev.sncndn(v);
        let (mut xi, eta) = self.sigma(snu, cnu, dnu, v, snv, cnv, dnv);
        if backside {
            xi = 2.0 * self.eu.ec - xi;
        }
        let y = xi * self.a * self.k0 * latsign;
        let x = eta * self.a * self.k0 * lonsign;
        let (mut gamma, k) = if lat == 90.0 {
            (lon, 1.0)
        } else {
            // Recompute (τ, λ) from (u, v) for an accurate scale.
            let (taup, _) = self.zeta(snu, cnu, dnu, snv, cnv, dnv);
            let tau = tauf(taup, self.e);
            let (g, k) = self.scale(tau, snu, cnu, dnu, snv, cnv, dnv);
            (g.to_degrees(), k)
        };
        if backside {
            gamma = 180.0 - gamma;
        }
        (x, y, gamma * latsign * lonsign, k * self.k0)
    }

    /// (lat, lon, convergence in degrees, scale) from x and y.
    pub fn inverse(&self, lon0: f64, x: f64, y: f64) -> (f64, f64, f64, f64) {
        let mut xi = y / (self.a * self.k0);
        let mut eta = x / (self.a * self.k0);
        let xisign = if xi.is_sign_negative() { -1.0 } else { 1.0 };
        let etasign = if eta.is_sign_negative() { -1.0 } else { 1.0 };
        xi *= xisign;
        eta *= etasign;
        let backside = xi > self.eu.ec;
        if backside {
            xi = 2.0 * self.eu.ec - xi;
        }
        let (u, v) = if xi == 0.0 && eta == self.ev.ke() {
            (0.0, self.ev.kc)
        } else {
            self.sigmainv(xi, eta)
        };
        let (snu, cnu, dnu) = self.eu.sncndn(u);
        let (snv, cnv, dnv) = self.ev.sncndn(v);
        let (mut lat, mut lon, mut gamma, k) = if v != 0.0 || u != self.eu.kc {
            let (taup, lam) = self.zeta(snu, cnu, dnu, snv, cnv, dnv);
            let tau = tauf(taup, self.e);
            let (g, k) = self.scale(tau, snu, cnu, dnu, snv, cnv, dnv);
            (atan(tau).to_degrees(), lam.to_degrees(), g.to_degrees(), k)
        } else {
            (90.0, 0.0, 0.0, 1.0)
        };
        if backside {
            lon = 180.0 - lon;
        }
        lon *= etasign;
        lon = ang_diff(0.0, lon + lon0);
        lat *= xisign;
        if backside {
            gamma = 180.0 - gamma;
        }
        gamma *= xisign * etasign;
        (lat, lon, gamma, k * self.k0)
    }

    /// The northing of the equator-to-latitude distance along the central
    /// meridian, for a latitude of origin.
    pub fn y_of(&self, lat: f64) -> f64 {
        self.forward(0.0, lat, 0.0).1
    }
}

/// The exact transverse Mercator with an origin, scale factor, and falsings.
pub struct TmExactGrid {
    tm: TmExact,
    lon0: f64,
    fe: f64,
    fn_: f64,
    y0: f64,
}

impl TmExactGrid {
    #[allow(clippy::too_many_arguments)]
    pub fn new(a: f64, f: f64, lat0: f64, lon0: f64, k0: f64, fe: f64, fn_: f64) -> TmExactGrid {
        let tm = TmExact::new(a, f, k0);
        let y0 = tm.y_of(lat0);
        TmExactGrid {
            tm,
            lon0,
            fe,
            fn_,
            y0,
        }
    }

    /// Easting, northing, convergence (degrees), scale.
    pub fn forward(&self, lat: f64, lon: f64) -> (f64, f64, f64, f64) {
        let (x, y, g, k) = self.tm.forward(self.lon0, lat, lon);
        (self.fe + x, self.fn_ + y - self.y0, g, k)
    }

    /// Latitude and longitude.
    pub fn inverse(&self, e: f64, n: f64) -> (f64, f64) {
        let (lat, lon, _, _) = self
            .tm
            .inverse(self.lon0, e - self.fe, n - self.fn_ + self.y0);
        (lat, lon)
    }
}
