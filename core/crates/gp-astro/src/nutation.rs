//! Nutation and the mean obliquity by IAU 2006/2000A, from the IERS
//! Conventions (2010): the fundamental arguments (eq. 5.43 and 5.44), the
//! nutation in longitude and obliquity (Tables 5.3a and 5.3b), the mean
//! obliquity (eq. 5.40), and the complementary terms of the equation of the
//! equinoxes (Table 5.2e).

use crate::iers_tables::{DEPS_0, DEPS_1, DPSI_0, DPSI_1, EECT_0, EECT_1};

/// Radians per arcsecond.
pub const AS2R: f64 = std::f64::consts::PI / (180.0 * 3600.0);

/// The 14 fundamental arguments (radians) at TT Julian centuries `t`: the five
/// Delaunay arguments (eq. 5.43) and the planetary longitudes and general
/// precession (eq. 5.44).
pub fn arguments(t: f64) -> [f64; 14] {
    let poly = |c: [f64; 5]| ((((c[4] * t + c[3]) * t + c[2]) * t + c[1]) * t + c[0]) * AS2R;
    let deg = |d: f64| d * 3600.0;
    [
        poly([
            deg(134.963_402_51),
            1_717_915_923.217_8,
            31.879_2,
            0.051_635,
            -0.000_244_70,
        ]),
        poly([
            deg(357.529_109_18),
            129_596_581.048_1,
            -0.553_2,
            0.000_136,
            -0.000_011_49,
        ]),
        poly([
            deg(93.272_090_62),
            1_739_527_262.847_8,
            -12.751_2,
            -0.001_037,
            0.000_004_17,
        ]),
        poly([
            deg(297.850_195_47),
            1_602_961_601.209,
            -6.370_6,
            0.006_593,
            -0.000_031_69,
        ]),
        poly([
            deg(125.044_555_01),
            -6_962_890.543_1,
            7.472_2,
            0.007_702,
            -0.000_059_39,
        ]),
        4.402_608_842 + 2_608.790_314_157_4 * t,
        3.176_146_697 + 1_021.328_554_621_1 * t,
        1.753_470_314 + 628.307_584_999_1 * t,
        6.203_480_913 + 334.061_242_670_0 * t,
        0.599_546_497 + 52.969_096_264_1 * t,
        0.874_016_757 + 21.329_910_496_0 * t,
        5.481_293_872 + 7.478_159_856_7 * t,
        5.311_886_287 + 3.813_303_563_8 * t,
        (0.024_381_75 + 0.000_005_386_91 * t) * t,
    ]
}

/// Σ (s sin a + c cos a) over a table's rows, in microarcseconds.
fn sum(rows: &[([i8; 14], f64, f64)], f: &[f64; 14]) -> f64 {
    rows.iter()
        .map(|(m, s, c)| {
            let a: f64 = m.iter().zip(f).map(|(k, x)| f64::from(*k) * x).sum();
            s * libm::sin(a) + c * libm::cos(a)
        })
        .sum()
}

/// Nutation in longitude and in obliquity (radians) at TT Julian centuries.
pub fn nutation(t: f64) -> (f64, f64) {
    let f = arguments(t);
    let dpsi = (sum(DPSI_0, &f) + t * sum(DPSI_1, &f)) * 1e-6 * AS2R;
    let deps = (sum(DEPS_0, &f) + t * sum(DEPS_1, &f)) * 1e-6 * AS2R;
    (dpsi, deps)
}

/// Nutation in longitude only (radians), for the equation of the equinoxes.
pub fn nutation_longitude(t: f64) -> f64 {
    let f = arguments(t);
    (sum(DPSI_0, &f) + t * sum(DPSI_1, &f)) * 1e-6 * AS2R
}

/// Mean obliquity of the ecliptic (radians), IAU 2006 (eq. 5.40).
pub fn mean_obliquity(t: f64) -> f64 {
    (((((-0.000_000_043_4 * t - 0.000_000_576) * t + 0.002_003_4) * t - 0.000_183_1) * t
        - 46.836_769)
        * t
        + 84_381.406)
        * AS2R
}

/// The complementary terms of the equation of the equinoxes (radians).
pub fn equinox_complement(t: f64) -> f64 {
    let f = arguments(t);
    (sum(EECT_0, &f) + t * sum(EECT_1, &f)) * 1e-6 * AS2R
}

/// The Fukushima-Williams precession angles γ̄, φ̄, ψ̄ (IERS Conventions 2010
/// eq. 5.40, which carry the frame bias), radians, at TT Julian centuries t.
pub fn precession_angles(t: f64) -> (f64, f64, f64) {
    let gamb = (-0.052_928
        + (10.556_378
            + (0.493_204_4 + (-0.000_312_38 + (-0.000_002_788 + 0.000_000_026_0 * t) * t) * t)
                * t)
            * t)
        * AS2R;
    let phib = (84_381.412_819
        + (-46.811_016
            + (0.051_126_8 + (0.000_532_89 + (-0.000_000_440 - 0.000_000_017_6 * t) * t) * t) * t)
            * t)
        * AS2R;
    let psib = (-0.041_775
        + (5_038.481_484
            + (1.558_417_5 + (-0.000_185_22 + (-0.000_026_452 - 0.000_000_014_8 * t) * t) * t)
                * t)
            * t)
        * AS2R;
    (gamb, phib, psib)
}

fn rot1(a: f64, v: [f64; 3]) -> [f64; 3] {
    let (s, c) = (libm::sin(a), libm::cos(a));
    [v[0], c * v[1] + s * v[2], -s * v[1] + c * v[2]]
}

fn rot3(a: f64, v: [f64; 3]) -> [f64; 3] {
    let (s, c) = (libm::sin(a), libm::cos(a));
    [c * v[0] + s * v[1], -s * v[0] + c * v[1], v[2]]
}

/// A vector on the mean ecliptic and equinox of J2000 carried to the true
/// equator and equinox of date: to the J2000 equator by the obliquity at
/// J2000.0, then by `icrs_to_true`.
pub fn ecliptic_j2000_to_true(v: [f64; 3], t: f64) -> [f64; 3] {
    icrs_to_true(rot1(-84_381.406 * AS2R, v), t)
}

/// The bias-precession-nutation rotation for one instant, from the ICRS to
/// the true equator and equinox of date: R1(−(εA + Δε)) R3(−(ψ̄ + Δψ))
/// R1(φ̄) R3(γ̄) by the Fukushima-Williams angles (IERS eq. 5.39), with the
/// nutation and true obliquity it was built from.
pub struct Frame {
    pub dpsi: f64,
    pub eps_true: f64,
    m: [[f64; 3]; 3],
}

impl Frame {
    pub fn at(t: f64) -> Frame {
        let (gamb, phib, psib) = precession_angles(t);
        let (dpsi, deps) = nutation(t);
        let eps_true = mean_obliquity(t) + deps;
        let rot = |v| rot1(-eps_true, rot3(-(psib + dpsi), rot1(phib, rot3(gamb, v))));
        let c = [
            rot([1.0, 0.0, 0.0]),
            rot([0.0, 1.0, 0.0]),
            rot([0.0, 0.0, 1.0]),
        ];
        Frame {
            dpsi,
            eps_true,
            m: [
                [c[0][0], c[1][0], c[2][0]],
                [c[0][1], c[1][1], c[2][1]],
                [c[0][2], c[1][2], c[2][2]],
            ],
        }
    }

    pub fn apply(&self, v: [f64; 3]) -> [f64; 3] {
        let m = &self.m;
        [
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ]
    }
}

/// A vector on the ICRS carried to the true equator and equinox of date
/// (`Frame`).
pub fn icrs_to_true(v: [f64; 3], t: f64) -> [f64; 3] {
    Frame::at(t).apply(v)
}
