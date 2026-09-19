//! Transverse Mercator by Krüger's series to 6th order in n (Karney 2011,
//! "Transverse Mercator with an accuracy of a few nanometers", J. Geodesy 85),
//! as in GeographicLib's TransverseMercator: better than 5 nm within 3,900 km of
//! the central meridian. Also the conformal-latitude helpers (τ ↔ τ′) that
//! polar stereographic reuses.

use libm::{asinh, atan, atan2, atanh, cos, cosh, hypot, sin, sinh, sqrt, tan};

/// Ellipsoid-derived constants for the series.
#[derive(Clone, Copy, Debug)]
pub struct Tm {
    pub a: f64,
    pub f: f64,
    pub k0: f64,
    e2: f64,
    es: f64,
    /// Rectifying radius A divided by a.
    b1: f64,
    alp: [f64; 7],
    bet: [f64; 7],
}

/// τ′ (tangent of conformal latitude) from τ (tangent of latitude).
pub fn taupf(tau: f64, es: f64) -> f64 {
    let tau1 = hypot(1.0, tau);
    let sig = sinh(es * atanh(es * tau / tau1));
    hypot(1.0, sig) * tau - sig * tau1
}

/// τ from τ′ by Newton's method (Karney 2011, eq. 19–21): converges to
/// machine precision in at most 5 iterations for |f| < 0.02.
pub fn tauf(taup: f64, es: f64) -> f64 {
    let e2m = 1.0 - es * es;
    let tol = f64::EPSILON.sqrt() / 10.0;
    let mut tau = taup / e2m;
    for _ in 0..5 {
        let taupa = taupf(tau, es);
        let dtau =
            (taup - taupa) * (1.0 + e2m * tau * tau) / (e2m * hypot(1.0, tau) * hypot(1.0, taupa));
        tau += dtau;
        if dtau.abs() < tol * tau.abs().max(1.0) {
            break;
        }
    }
    tau
}

impl Tm {
    pub fn new(a: f64, f: f64, k0: f64) -> Tm {
        let n = f / (2.0 - f);
        let (n2, n3) = (n * n, n * n * n);
        let (n4, n5, n6) = (n2 * n2, n2 * n3, n3 * n3);
        let alp = [
            0.0,
            n / 2.0 - 2.0 / 3.0 * n2 + 5.0 / 16.0 * n3 + 41.0 / 180.0 * n4 - 127.0 / 288.0 * n5
                + 7891.0 / 37800.0 * n6,
            13.0 / 48.0 * n2 - 3.0 / 5.0 * n3 + 557.0 / 1440.0 * n4 + 281.0 / 630.0 * n5
                - 1_983_433.0 / 1_935_360.0 * n6,
            61.0 / 240.0 * n3 - 103.0 / 140.0 * n4
                + 15061.0 / 26880.0 * n5
                + 167_603.0 / 181_440.0 * n6,
            49561.0 / 161_280.0 * n4 - 179.0 / 168.0 * n5 + 6_601_661.0 / 7_257_600.0 * n6,
            34729.0 / 80640.0 * n5 - 3_418_889.0 / 1_995_840.0 * n6,
            212_378_941.0 / 319_334_400.0 * n6,
        ];
        let bet = [
            0.0,
            n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - 1.0 / 360.0 * n4 - 81.0 / 512.0 * n5
                + 96199.0 / 604_800.0 * n6,
            1.0 / 48.0 * n2 + 1.0 / 15.0 * n3 - 437.0 / 1440.0 * n4 + 46.0 / 105.0 * n5
                - 1_118_711.0 / 3_870_720.0 * n6,
            17.0 / 480.0 * n3 - 37.0 / 840.0 * n4 - 209.0 / 4480.0 * n5 + 5569.0 / 90720.0 * n6,
            4397.0 / 161_280.0 * n4 - 11.0 / 504.0 * n5 - 830_251.0 / 7_257_600.0 * n6,
            4583.0 / 161_280.0 * n5 - 108_847.0 / 3_991_680.0 * n6,
            20_648_693.0 / 638_668_800.0 * n6,
        ];
        let e2 = f * (2.0 - f);
        Tm {
            a,
            f,
            k0,
            e2,
            es: sqrt(e2),
            b1: (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0) / (1.0 + n),
            alp,
            bet,
        }
    }

    /// Forward: (lat, lon − lon0) in degrees → (x, y, convergence γ in degrees, point scale k).
    /// The convergence is the bearing of grid north clockwise from true north.
    pub fn forward(&self, lat: f64, dlon: f64) -> (f64, f64, f64, f64) {
        let (phi, lam) = (lat.to_radians(), dlon.to_radians());
        let (sl, cl) = (sin(lam), cos(lam));
        let (xip, etap, gam0, k0p) = if lat.abs() == 90.0 {
            (core::f64::consts::FRAC_PI_2.copysign(phi), 0.0, lam, 1.0)
        } else {
            let tau = tan(phi);
            let taup = taupf(tau, self.es);
            let xip = atan2(taup, cl);
            let etap = asinh(sl / hypot(taup, cl));
            let gam = atan2(sl * taup, cl * hypot(1.0, taup));
            let k = sqrt(1.0 - self.e2 + self.e2 * cos(phi) * cos(phi)) * hypot(1.0, tau)
                / hypot(taup, cl);
            (xip, etap, gam, k)
        };
        let (mut xi, mut eta, mut p, mut q) = (xip, etap, 1.0, 0.0);
        for j in 1..=6 {
            let jj = 2.0 * j as f64;
            let (s, c) = (sin(jj * xip), cos(jj * xip));
            let (sh, ch) = (sinh(jj * etap), cosh(jj * etap));
            xi += self.alp[j] * s * ch;
            eta += self.alp[j] * c * sh;
            p += jj * self.alp[j] * c * ch;
            q += jj * self.alp[j] * s * sh;
        }
        let gamma = gam0 + atan2(q, p);
        let k = self.k0 * self.b1 * k0p * hypot(p, q);
        let ka = self.k0 * self.b1 * self.a;
        (ka * eta, ka * xi, gamma.to_degrees(), k)
    }

    /// Inverse: (x, y) in meters → (lat, lon − lon0) in degrees.
    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let ka = self.k0 * self.b1 * self.a;
        let (xi, eta) = (y / ka, x / ka);
        let (mut xip, mut etap) = (xi, eta);
        for j in 1..=6 {
            let jj = 2.0 * j as f64;
            xip -= self.bet[j] * sin(jj * xi) * cosh(jj * eta);
            etap -= self.bet[j] * cos(jj * xi) * sinh(jj * eta);
        }
        let (s, c) = (sin(xip), cos(xip));
        let r = hypot(sinh(etap), c);
        if r == 0.0 {
            return (90f64.copysign(xip), 0.0);
        }
        let taup = s / r;
        let lam = atan2(sinh(etap), c);
        let tau = tauf(taup, self.es);
        (atan(tau).to_degrees(), lam.to_degrees())
    }

    /// Rectifying radius A (the meridian quadrant is A π/2).
    pub fn rectifying_radius(&self) -> f64 {
        self.a * self.b1
    }

    /// Series coefficients: conformal to rectifying latitude (α) and back (β), 1-based.
    pub fn series(&self) -> (&[f64; 7], &[f64; 7]) {
        (&self.alp, &self.bet)
    }

    pub fn es(&self) -> f64 {
        self.es
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WGS84_A: f64 = 6_378_137.0;
    const WGS84_F: f64 = 1.0 / 298.257_223_563;

    #[test]
    fn pittsburgh_zone_17() {
        let tm = Tm::new(WGS84_A, WGS84_F, 0.9996);
        let (x, y, _, _) = tm.forward(40.446_111, -79.982_222 + 81.0);
        assert!(
            (x + 500_000.0 - 586_309.953).abs() < 1e-3,
            "{}",
            x + 500_000.0
        );
        assert!((y - 4_477_770.428).abs() < 1e-3, "{y}");
    }

    #[test]
    fn round_trip_and_central_meridian_scale() {
        let tm = Tm::new(WGS84_A, WGS84_F, 0.9996);
        for lat in [-80.0, -45.0, 0.0, 12.3, 60.0, 84.0] {
            for dlon in [-3.0, -0.5, 0.0, 1.7, 3.0, 20.0] {
                let (x, y, _, k) = tm.forward(lat, dlon);
                let (la, lo) = tm.inverse(x, y);
                assert!(
                    (la - lat).abs() < 1e-11 && (lo - dlon).abs() < 1e-11,
                    "{lat} {dlon} -> {la} {lo}"
                );
                if dlon == 0.0 {
                    assert!((k - 0.9996).abs() < 1e-12, "{k}");
                }
            }
        }
    }

    /// Convergence and scale agree with finite differences of the forward map.
    #[test]
    fn convergence_and_scale_match_finite_differences() {
        let tm = Tm::new(WGS84_A, WGS84_F, 0.9996);
        let (e2, h) = (WGS84_F * (2.0 - WGS84_F), 1e-6);
        for (lat, dlon) in [(40.0, 2.0), (-30.0, -2.5), (70.0, 3.0), (10.0, -1.0)] {
            let (x0, y0, gamma, k) = tm.forward(lat, dlon);
            let (x1, y1, _, _) = tm.forward(lat + h, dlon);
            let gam_fd = -atan2(x1 - x0, y1 - y0).to_degrees();
            assert!((gam_fd - gamma).abs() < 1e-5, "γ {gamma} vs {gam_fd}");
            let (x2, y2, _, _) = tm.forward(lat, dlon + h);
            let phi = lat.to_radians();
            let nu = WGS84_A / sqrt(1.0 - e2 * sin(phi) * sin(phi));
            let k_fd = hypot(x2 - x0, y2 - y0) / (nu * cos(phi) * h.to_radians());
            assert!((k_fd - k).abs() < 1e-7, "k {k} vs {k_fd}");
        }
    }
}
