//! Geodesics on strongly flattened ellipsoids, |f| > 0.02, where the series
//! behind the everyday method lose accuracy (the job of GeographicLib's
//! GeodesicExact). The integrals of Karney (2013) are evaluated directly, by
//! composite 16-point Gauss-Legendre quadrature on the auxiliary sphere, where
//! they are smooth and periodic, so the quadrature converges to rounding:
//!
//! - distance, s = b ∫ w dσ with w = √(1 + k² sin²σ), eq. 7;
//! - longitude, λ = ω − f sin α₀ ∫ (2 − f) / (1 + (1 − f) w) dσ, eq. 8;
//! - reduced length and geodesic scale from J = ∫ (w − 1/w) dσ, eqs. 38-39;
//! - the area under the geodesic, eqs. 58-59.
//!
//! The inverse problem puts the points in Karney's canonical position, where
//! λ12 rises with the start azimuth on [0, π], and bisects on that azimuth to
//! rounding. Supported for |f| ≤ 0.5; nearly antipodal points, where the
//! shortest geodesic may not be unique, are refused.

use core::f64::consts::PI;

use libm::{asin, asinh, atan, atan2, atanh, cos, hypot, sin, sqrt};

/// The largest |f| the quadrature is checked for.
pub const MAX_F: f64 = 0.5;

const GX: [f64; 8] = [
    0.095_012_509_837_637_44,
    0.281_603_550_779_258_9,
    0.458_016_777_657_227_4,
    0.617_876_244_402_643_8,
    0.755_404_408_355_003,
    0.865_631_202_387_831_8,
    0.944_575_023_073_232_6,
    0.989_400_934_991_649_9,
];
const GW: [f64; 8] = [
    0.189_450_610_455_068_5,
    0.182_603_415_044_923_6,
    0.169_156_519_395_002_5,
    0.149_595_988_816_576_7,
    0.124_628_971_255_533_9,
    0.095_158_511_682_492_8,
    0.062_253_523_938_647_9,
    0.027_152_459_411_754_1,
];

/// ∫ from `x1` to `x2` of `f`, on pieces at most π/16 wide.
fn integ(f: impl Fn(f64) -> f64, x1: f64, x2: f64) -> f64 {
    let span = x2 - x1;
    if span == 0.0 {
        return 0.0;
    }
    let k = libm::ceil(span.abs() / (PI / 16.0)).max(1.0) as usize;
    let h = span / k as f64;
    let mut total = 0.0;
    for i in 0..k {
        let mid = x1 + (i as f64 + 0.5) * h;
        let mut sum = 0.0;
        for j in 0..8 {
            let d = GX[j] * h / 2.0;
            sum += GW[j] * (f(mid - d) + f(mid + d));
        }
        total += sum * h / 2.0;
    }
    total
}

/// asinh(√x)/√x, continued through x = 0 and to x < 0 as asin(√−x)/√−x.
fn ash(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        1.0 - x / 6.0 + 3.0 * x * x / 40.0 - 5.0 * x * x * x / 112.0
    } else if x > 0.0 {
        asinh(sqrt(x)) / sqrt(x)
    } else {
        asin(sqrt(-x)) / sqrt(-x)
    }
}

/// Karney's t(x) = x + √(1/x + 1) asinh √x, written to hold at x ≤ 0.
fn tfun(x: f64) -> f64 {
    x + sqrt(1.0 + x) * ash(x)
}

pub struct Exact {
    a: f64,
    f: f64,
    b: f64,
    e2: f64,
    ep2: f64,
    c2: f64,
}

/// A solved geodesic: the far end, and the auxiliary quantities GeodSolve -f prints.
#[derive(Clone, Copy, Debug)]
pub struct Solution {
    pub lat1: f64,
    pub lon1: f64,
    pub azi1: f64,
    pub lat2: f64,
    pub lon2: f64,
    pub azi2: f64,
    pub s12: f64,
    /// Arc length on the auxiliary sphere, degrees.
    pub a12: f64,
    pub m12: f64,
    pub big_m12: f64,
    pub big_m21: f64,
    /// Area between the geodesic and the equator, m².
    pub area12: f64,
}

impl Exact {
    pub fn new(a: f64, f: f64) -> Exact {
        let e2 = f * (2.0 - f);
        let b = a * (1.0 - f);
        let ash_e = if e2 == 0.0 {
            1.0
        } else if e2 > 0.0 {
            atanh(sqrt(e2)) / sqrt(e2)
        } else {
            atan(sqrt(-e2)) / sqrt(-e2)
        };
        Exact {
            a,
            f,
            b,
            e2,
            ep2: e2 / (1.0 - e2),
            c2: a * a / 2.0 + b * b / 2.0 * ash_e,
        }
    }

    /// The direct problem: from (lat1, lon1) on azimuth `azi1` for `s12` meters.
    pub fn direct(&self, lat1: f64, lon1: f64, azi1: f64, s12: f64) -> Solution {
        let f = self.f;
        let beta1 = atan((1.0 - f) * libm::tan(lat1.to_radians()));
        let (sbet1, cbet1) = if lat1.abs() >= 90.0 {
            (1.0f64.copysign(lat1), 1e-300)
        } else {
            (sin(beta1), cos(beta1))
        };
        let (salp1, calp1) = (sin(azi1.to_radians()), cos(azi1.to_radians()));
        let salp0 = salp1 * cbet1;
        let calp0 = hypot(calp1, salp1 * sbet1);
        let sig1 = atan2(
            sbet1,
            if sbet1 == 0.0 && calp1 == 0.0 {
                1.0
            } else {
                calp1 * cbet1
            },
        );
        let (ssig1, csig1) = (sin(sig1), cos(sig1));
        let k2 = self.ep2 * calp0 * calp0;
        let w = |s: f64| sqrt(1.0 + k2 * sin(s) * sin(s));
        // σ2 from the distance: b ∫ w dσ = s12, by Newton's method.
        let mut sig2 = sig1 + s12 / self.b;
        for _ in 0..30 {
            let err = self.b * integ(w, sig1, sig2) - s12;
            let step = err / (self.b * w(sig2));
            sig2 -= step;
            if step.abs() < 1e-15 * sig2.abs().max(1.0) {
                break;
            }
        }
        let (ssig2, csig2) = (sin(sig2), cos(sig2));
        let sig12 = sig2 - sig1;
        let sbet2 = calp0 * ssig2;
        let cbet2 = hypot(salp0, calp0 * csig2);
        let lat2 = atan2(sbet2, (1.0 - f) * cbet2).to_degrees();
        let (salp2, calp2) = (salp0, calp0 * csig2);
        // Longitude on the auxiliary sphere, unrolled with σ (Karney's LONG_UNROLL).
        let e = 1.0f64.copysign(salp0);
        let (somg1, somg2) = (salp0 * ssig1, salp0 * ssig2);
        let omg12 = e
            * (sig12 - (atan2(ssig2, csig2) - atan2(ssig1, csig1))
                + (atan2(e * somg2, csig2) - atan2(e * somg1, csig1)));
        let i3 = integ(|s| (2.0 - f) / (1.0 + (1.0 - f) * w(s)), sig1, sig2);
        let lam12 = omg12 - f * salp0 * i3;
        // Reduced length and geodesic scale.
        let (w1, w2) = (w(sig1), w(sig2));
        let j12 = integ(|s| w(s) - 1.0 / w(s), sig1, sig2);
        let m12 = self.b * (w2 * csig1 * ssig2 - w1 * ssig1 * csig2 - csig1 * csig2 * j12);
        let csig12 = cos(sig12);
        let t = k2 * (ssig2 - ssig1) * (ssig2 + ssig1) / (w1 + w2);
        let big_m12 = csig12 + (t * ssig2 - csig2 * j12) * ssig1 / w1;
        let big_m21 = csig12 - (t * ssig1 - csig1 * j12) * ssig2 / w2;
        // Area: c²(α2 − α1) + e² a² cos α0 sin α0 (I4(σ2) − I4(σ1)).
        let salp12 = salp2 * calp1 - calp2 * salp1;
        let calp12 = calp2 * calp1 + salp2 * salp1;
        let mut area12 = self.c2 * atan2(salp12, calp12);
        if salp0 != 0.0 && calp0 != 0.0 {
            let tep = tfun(self.ep2);
            let g = |s: f64| {
                let x = k2 * sin(s) * sin(s);
                let den = self.ep2 - x;
                if den.abs() < 1e-14 {
                    0.0
                } else {
                    (tep - tfun(x)) / den * sin(s) / 2.0
                }
            };
            area12 += self.e2 * self.a * self.a * calp0 * salp0 * -integ(g, sig1, sig2);
        }
        Solution {
            lat1,
            lon1,
            azi1,
            lat2,
            lon2: lon1 + lam12.to_degrees(),
            azi2: atan2(salp2, calp2).to_degrees(),
            s12,
            a12: sig12.to_degrees(),
            m12,
            big_m12,
            big_m21,
            area12,
        }
    }

    /// λ12 (radians) and the arcs σ1, σ2 for start azimuth `alp1` (radians)
    /// in canonical position: β1 ≤ 0 and |β2| ≤ |β1|, following the geodesic
    /// to where it first reaches β2 (Karney 2013, section 4).
    fn lambda12(
        &self,
        sbet1: f64,
        cbet1: f64,
        sbet2: f64,
        cbet2: f64,
        alp1: f64,
    ) -> (f64, f64, f64) {
        let f = self.f;
        let (salp1, calp1) = (sin(alp1), cos(alp1));
        let salp0 = salp1 * cbet1;
        let calp0 = hypot(calp1, salp1 * sbet1);
        let sig1 = atan2(sbet1, calp1 * cbet1);
        // cos α2 cos β2 on the way up to β2 (eq. 45).
        let c2 = sqrt((calp1 * cbet1).powi(2) + (cbet2 - cbet1) * (cbet2 + cbet1));
        let sig2r = atan2(sbet2, c2);
        let (ssig1, csig1, ssig2, csig2) = (sin(sig1), cos(sig1), sin(sig2r), cos(sig2r));
        let sig12 = atan2(
            (csig1 * ssig2 - ssig1 * csig2).max(0.0),
            csig1 * csig2 + ssig1 * ssig2,
        );
        let sig2 = sig1 + sig12;
        let (somg1, somg2) = (salp0 * ssig1, salp0 * ssig2);
        let omg12 = atan2(
            (csig1 * somg2 - somg1 * csig2).max(0.0),
            csig1 * csig2 + somg1 * somg2,
        );
        let k2 = self.ep2 * calp0 * calp0;
        let i3 = integ(
            |s| (2.0 - f) / (1.0 + (1.0 - f) * sqrt(1.0 + k2 * sin(s) * sin(s))),
            sig1,
            sig2,
        );
        (omg12 - f * salp0 * i3, sig1, sig2)
    }

    /// The inverse problem, or None for nearly antipodal points, where the
    /// shortest geodesic may not be unique and is not searched for here.
    pub fn inverse(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Option<Solution> {
        let f = self.f;
        let mut lon12 = (lon2 - lon1 + 180.0).rem_euclid(360.0) - 180.0;
        if lon12 == -180.0 {
            lon12 = 180.0;
        }
        // Canonical position (Karney's GenInverse): λ12 ≥ 0, |φ1| ≥ |φ2|, φ1 ≤ 0.
        let mut lonsign = if lon12 < 0.0 { -1.0 } else { 1.0 };
        lon12 *= lonsign;
        let swapp = if lat1.abs() < lat2.abs() { -1.0 } else { 1.0 };
        let (mut la1, mut la2) = (lat1, lat2);
        if swapp < 0.0 {
            lonsign = -lonsign;
            core::mem::swap(&mut la1, &mut la2);
        }
        let latsign = if la1 < 0.0 { 1.0 } else { -1.0 };
        la1 *= latsign;
        la2 *= latsign;
        let red = |lat: f64| {
            if lat.abs() >= 90.0 {
                (1.0f64.copysign(lat), 0.0)
            } else {
                let b = atan((1.0 - f) * libm::tan(lat.to_radians()));
                (sin(b), cos(b))
            }
        };
        let ((sbet1, cbet1), (sbet2, cbet2)) = (red(la1), red(la2));
        let lam12 = lon12.to_radians();
        // Nearly antipodal points are left alone.
        let sig = atan2(
            hypot(
                cbet2 * sin(lam12),
                cbet1 * sbet2 - sbet1 * cbet2 * cos(lam12),
            ),
            sbet1 * sbet2 + cbet1 * cbet2 * cos(lam12),
        );
        if sig > 170f64.to_radians() {
            return None;
        }
        let alp1 = if la1 == 0.0 && la2 == 0.0 && lam12 <= (1.0 - f) * PI {
            // Along the equator.
            core::f64::consts::FRAC_PI_2
        } else if cbet1 == 0.0 || lam12 == 0.0 || lam12 == PI {
            // Along a meridian (through the pole when λ12 = 180°).
            lam12
        } else {
            // λ12 rises with α1 on [0, π]: bisect to rounding.
            let (mut lo, mut hi) = (0.0f64, PI);
            for _ in 0..200 {
                let mid = (lo + hi) / 2.0;
                if mid <= lo || mid >= hi {
                    break;
                }
                if self.lambda12(sbet1, cbet1, sbet2, cbet2, mid).0 < lam12 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            (lo + hi) / 2.0
        };
        // The canonical azimuths at each end, then undo the symmetries.
        let (_, sig1, sig2) = self.lambda12(sbet1, cbet1, sbet2, cbet2, alp1);
        let salp0 = sin(alp1) * cbet1;
        let calp0 = hypot(cos(alp1), sin(alp1) * sbet1);
        let (mut salp1, mut calp1) = (sin(alp1), cos(alp1));
        let (mut salp2, mut calp2) = (salp0, calp0 * cos(sig2));
        if swapp < 0.0 {
            core::mem::swap(&mut salp1, &mut salp2);
            core::mem::swap(&mut calp1, &mut calp2);
        }
        salp1 *= swapp * lonsign;
        calp1 *= swapp * latsign;
        let k2 = self.ep2 * calp0 * calp0;
        let s12 = self.b * integ(|s| sqrt(1.0 + k2 * sin(s) * sin(s)), sig1, sig2);
        let d = self.direct(lat1, lon1, atan2(salp1, calp1).to_degrees(), s12);
        // A meridian through the pole is only shortest before its conjugate point.
        if d.m12 < 0.0 {
            return None;
        }
        Some(Solution { lat2, lon2, ..d })
    }
}
