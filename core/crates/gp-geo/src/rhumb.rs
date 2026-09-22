//! Rhumb lines (loxodromes) on the ellipsoid, after C. F. F. Karney's Rhumb
//! class in GeographicLib: in conformal coordinates a rhumb is straight, so the course is
//! atan2(Δλ, Δψ) and the distance follows from the rectifying latitude μ. The
//! distance per unit of isometric latitude, D = ΔM/Δψ, is evaluated with
//! divided differences, so nearly east-west rhumbs keep full accuracy.

use core::f64::consts::FRAC_PI_2;

use libm::{asinh, atan, atan2, cos, cosh, hypot, sin, sinh, tan};

use crate::tm::{Tm, tauf, taupf};

pub struct Rhumb {
    tm: Tm,
    es: f64,
    a: f64,
}

/// A rhumb's end: latitude and longitude in degrees, and the distance past a
/// pole it could not travel (0 unless the rhumb reached a pole).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct End {
    pub lat: f64,
    pub lon: f64,
    pub beyond_pole: f64,
}

fn norm_lon(x: f64) -> f64 {
    let r = (x + 180.0).rem_euclid(360.0) - 180.0;
    if r == -180.0 && x > 0.0 { 180.0 } else { r }
}

impl Rhumb {
    pub fn new(a: f64, f: f64) -> Rhumb {
        let tm = Tm::new(a, f, 1.0);
        Rhumb {
            es: libm::sqrt(f * (2.0 - f)),
            tm,
            a,
        }
    }

    /// Conformal latitude χ (radians) of latitude `lat` (degrees).
    fn chi(&self, lat: f64) -> f64 {
        if lat.abs() >= 90.0 {
            return FRAC_PI_2.copysign(lat);
        }
        atan(taupf(tan(lat.to_radians()), self.es))
    }

    /// Rectifying latitude μ from χ.
    fn mu(&self, chi: f64) -> f64 {
        let (alp, _) = self.tm.series();
        chi + (1..=6)
            .map(|j| alp[j] * sin(2.0 * j as f64 * chi))
            .sum::<f64>()
    }

    /// χ from μ.
    fn chi_of_mu(&self, mu: f64) -> f64 {
        let (_, bet) = self.tm.series();
        mu - (1..=6)
            .map(|j| bet[j] * sin(2.0 * j as f64 * mu))
            .sum::<f64>()
    }

    /// D = ΔM/Δψ between two latitudes off the poles (meters per unit of ψ),
    /// by divided differences: ΔM/Δψ = A (Δμ/Δχ)(Δχ/Δψ).
    fn d(&self, chi1: f64, chi2: f64) -> f64 {
        let (psi1, psi2) = (asinh(tan(chi1)), asinh(tan(chi2)));
        let dpsi = psi2 - psi1;
        // Δχ/Δψ: gd(ψ2) − gd(ψ1) = atan2(2 cosh m sinh(Δψ/2), 1 + sinh ψ1 sinh ψ2).
        let (dchi, dchi_dpsi) = if dpsi == 0.0 {
            (0.0, cos(chi1))
        } else {
            let dchi = atan2(
                2.0 * cosh((psi1 + psi2) / 2.0) * sinh(dpsi / 2.0),
                1.0 + sinh(psi1) * sinh(psi2),
            );
            (dchi, dchi / dpsi)
        };
        // Δμ/Δχ = 1 + Σ αj 2 cos(j(χ1 + χ2)) sin(jΔχ)/Δχ.
        let (alp, _) = self.tm.series();
        let dmu_dchi = 1.0
            + (1..=6)
                .map(|j| {
                    let jj = j as f64;
                    let sinc = if dchi == 0.0 {
                        jj
                    } else {
                        sin(jj * dchi) / dchi
                    };
                    alp[j] * 2.0 * cos(jj * (chi1 + chi2)) * sinc
                })
                .sum::<f64>();
        self.tm.rectifying_radius() * dmu_dchi * dchi_dpsi
    }

    /// q(φ) of the authalic latitude, from sin φ: sin ξ = q(φ) / q(90°).
    fn q(&self, s: f64) -> f64 {
        let e = self.es;
        let e2 = e * e;
        let t = if e < 1e-8 { s } else { libm::atanh(e * s) / e };
        (1.0 - e2) * (s / (1.0 - e2 * s * s) + t)
    }

    /// The authalic radius squared, c² = a² q(90°) / 2: the ellipsoid's area is 4πc².
    pub fn c2(&self) -> f64 {
        self.a * self.a * self.q(1.0) / 2.0
    }

    /// The mean of sin ξ (authalic latitude) along the rhumb between two
    /// latitudes off the poles. Longitude is linear in the isometric latitude
    /// ψ along a rhumb, so this is the mean over ψ, by 8-point Gauss-Legendre
    /// on pieces at most 0.05 wide.
    fn mean_sin_xi(&self, lat1: f64, lat2: f64) -> f64 {
        const X: [f64; 4] = [
            0.183_434_642_495_649_8,
            0.525_532_409_916_329,
            0.796_666_477_413_626_7,
            0.960_289_856_497_536_3,
        ];
        const W: [f64; 4] = [
            0.362_683_783_378_362,
            0.313_706_645_877_887_3,
            0.222_381_034_453_374_5,
            0.101_228_536_290_376_3,
        ];
        let qp = self.q(1.0);
        let psi = |lat: f64| asinh(taupf(tan(lat.to_radians()), self.es));
        let sin_xi = |p: f64| {
            let t = tauf(sinh(p), self.es);
            self.q(t / hypot(1.0, t)) / qp
        };
        let (p1, p2) = (psi(lat1), psi(lat2));
        let span = p2 - p1;
        if span.abs() < 1e-12 {
            return sin_xi((p1 + p2) / 2.0);
        }
        let k = libm::ceil(span.abs() / 0.05).max(1.0) as usize;
        let h = span / k as f64;
        let mut sum = 0.0;
        for i in 0..k {
            let mid = p1 + (i as f64 + 0.5) * h;
            for j in 0..4 {
                sum += W[j] * (sin_xi(mid - X[j] * h / 2.0) + sin_xi(mid + X[j] * h / 2.0));
            }
        }
        // Each piece's weights sum to 2 over its half-width units.
        sum / (2.0 * k as f64)
    }

    /// Signed area (m², counterclockwise positive) of a ring of rhumb lines
    /// through `ring` (degrees, off the poles), after Karney (2024): the sum
    /// over edges of c² Δλ (1 − mean sin ξ), which also covers a ring that
    /// circles a pole. Of the two regions the ring bounds, the smaller one.
    pub fn ring_area(&self, ring: &[(f64, f64)]) -> f64 {
        let c2 = self.c2();
        let n = ring.len();
        let mut area = 0.0;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let dlon = norm_lon(b.1 - a.1).to_radians();
            area += c2 * dlon * (1.0 - self.mean_sin_xi(a.0, b.0));
        }
        let total = 4.0 * core::f64::consts::PI * c2;
        if area > total / 2.0 {
            area -= total;
        } else if area < -total / 2.0 {
            area += total;
        }
        area
    }

    /// Inverse: distance (m) and constant course (degrees, 0-360) from 1 to 2.
    pub fn inverse(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
        let dlon = norm_lon(lon2 - lon1).to_radians();
        let (chi1, chi2) = (self.chi(lat1), self.chi(lat2));
        let dm = self.tm.rectifying_radius() * (self.mu(chi2) - self.mu(chi1));
        if lat1.abs() >= 90.0 || lat2.abs() >= 90.0 {
            // To or from a pole the rhumb is a meridian (winding infinitely at the pole).
            return (dm.abs(), if dm >= 0.0 { 0.0 } else { 180.0 });
        }
        let east = dlon * self.d(chi1, chi2);
        let azi = atan2(east, dm).to_degrees();
        (hypot(dm, east), if azi < 0.0 { azi + 360.0 } else { azi })
    }

    /// Direct: the end of a rhumb from (lat1, lon1) on course `azi` for `s`
    /// meters. A rhumb that would pass a pole stops there, with the distance
    /// left over in `beyond_pole`.
    pub fn direct(&self, lat1: f64, lon1: f64, azi: f64, s: f64) -> End {
        let a = azi.to_radians();
        let chi1 = self.chi(lat1);
        let mu1 = self.mu(chi1);
        let big_a = self.tm.rectifying_radius();
        let mut mu2 = mu1 + s * cos(a) / big_a;
        let mut travelled = s;
        let mut beyond_pole = 0.0;
        if mu2.abs() > FRAC_PI_2 {
            mu2 = FRAC_PI_2.copysign(mu2);
            travelled = (mu2 - mu1) * big_a / cos(a);
            beyond_pole = s - travelled;
        }
        if mu2.abs() == FRAC_PI_2 {
            // At the pole longitude is undefined; keep the start's.
            return End {
                lat: 90.0f64.copysign(mu2),
                lon: norm_lon(lon1),
                beyond_pole,
            };
        }
        let chi2 = self.chi_of_mu(mu2);
        let lat2 = atan(tauf(tan(chi2), self.es)).to_degrees();
        let dlon = if lat1.abs() >= 90.0 {
            0.0
        } else {
            travelled * sin(a) / self.d(chi1, self.chi(lat2))
        };
        End {
            lat: lat2,
            lon: norm_lon(lon1 + dlon.to_degrees()),
            beyond_pole,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jfk_to_lhr_matches_the_spec() {
        let r = Rhumb::new(6_378_137.0, 1.0 / 298.257_223_563);
        let (s, azi) = r.inverse(40.6413, -73.7781, 51.47, -0.4543);
        assert!((azi - 77.968).abs() < 0.001, "{azi}");
        assert!((s - 5_774_190.0).abs() < 1.0, "{s}");
        let e = r.direct(40.6413, -73.7781, azi, s);
        assert!(
            (e.lat - 51.47).abs() < 1e-11 && (e.lon + 0.4543).abs() < 1e-11,
            "{e:?}"
        );
    }
}
