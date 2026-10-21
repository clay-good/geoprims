//! The Moon from ELP 2000-82B (Chapront-Touze and Chapront), as the authors'
//! subroutine ELP82B computes it: the series give longitude, latitude, and
//! distance on the inertial mean ecliptic of date, then a rotation by the
//! ecliptic's precession (the subroutine's P and Q) gives rectangular
//! coordinates on the mean ecliptic and equinox of J2000.

use crate::elp_tables::MOON;
use crate::nutation::AS2R;

const J2000: f64 = 2_451_545.0;
/// The theory's mean distance and the fitted one (km).
const ATH: f64 = 384_747.980_674_316_5;
const A0: f64 = 384_747.980_644_895_4;
/// The Moon's mean longitude W1: degrees, then arcseconds per century^k.
const W1: [f64; 5] = [
    218.0 + 18.0 / 60.0 + 59.955_71 / 3600.0,
    1_732_559_343.736_04,
    -5.888_3,
    0.006_604,
    -0.000_031_69,
];
const P: [f64; 5] = [
    0.101_803_91e-4,
    0.470_204_39e-6,
    -0.541_736_7e-9,
    -0.250_794_8e-11,
    0.463_486e-14,
];
const Q: [f64; 5] = [
    -0.113_469_002e-3,
    0.123_726_74e-6,
    0.126_541_7e-8,
    -0.137_180_8e-11,
    -0.320_334e-14,
];

fn poly(c: &[f64; 5], t: f64) -> f64 {
    c.iter().rev().fold(0.0, |acc, x| acc * t + x)
}

/// Geocentric rectangular coordinates (km) of the Moon on the mean ecliptic
/// and equinox of J2000, at a TT (TDB) Julian date.
pub fn moon_j2000(jd_tt: f64) -> [f64; 3] {
    let t = (jd_tt - J2000) / 36_525.0;
    let tp = [1.0, t, t * t, t * t * t, t * t * t * t];
    let mut r = [0.0; 3];
    for (coord, by_power) in MOON.iter().enumerate() {
        for (power, terms) in by_power.iter().enumerate() {
            let sum: f64 = terms
                .iter()
                .map(|c| {
                    let arg = c[1] + t * (c[2] + t * (c[3] + t * (c[4] + t * c[5])));
                    c[0] * libm::sin(arg)
                })
                .sum();
            r[coord] += sum * tp[power];
        }
    }
    let lon = r[0] * AS2R
        + W1[0].to_radians()
        + (W1[1] * t + W1[2] * tp[2] + W1[3] * tp[3] + W1[4] * tp[4]) * AS2R;
    let lat = r[1] * AS2R;
    let dist = r[2] * A0 / ATH;
    let x1 = dist * libm::cos(lat) * libm::cos(lon);
    let x2 = dist * libm::cos(lat) * libm::sin(lon);
    let x3 = dist * libm::sin(lat);
    let pw = poly(&P, t) * t;
    let qw = poly(&Q, t) * t;
    let ra = 2.0 * (1.0 - pw * pw - qw * qw).sqrt();
    let pwqw = 2.0 * pw * qw;
    let pw2 = 1.0 - 2.0 * pw * pw;
    let qw2 = 1.0 - 2.0 * qw * qw;
    let (pw, qw) = (pw * ra, qw * ra);
    [
        pw2 * x1 + pwqw * x2 + pw * x3,
        pwqw * x1 + qw2 * x2 - qw * x3,
        -pw * x1 + qw * x2 + (pw2 + qw2 - 1.0) * x3,
    ]
}
