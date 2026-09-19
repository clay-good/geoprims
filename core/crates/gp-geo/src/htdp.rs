//! NGS HTDP frame transformations (v3.6.0): 14-parameter sets from ITRF94 to
//! NAD 83 and the ITRF and WGS 84 realizations, applied exactly as HTDP's
//! subroutines frit94 and toit94 do (coordinate-frame rotations, additive
//! scale, and a reverse that negates the parameters). Parameters are the
//! `tranpa` block of SETTP in htdp.f, used whenever NAD 83 is involved.

/// Arc-seconds per radian, as HTDP's RHOSEC.
const RHOSEC: f64 = 180.0 * 3600.0 / core::f64::consts::PI;

/// tx, ty, tz, dtx, dty, dtz (m, m/yr); rx, ry, rz, drx, dry, drz (arcsec, arcsec/yr);
/// scale, dscale (unitless, per yr); reference epoch.
type Set = [f64; 15];

/// (frame, HTDP set) — from ITRF94 to the frame.
pub const SETS: &[(&str, Set)] = &[
    (
        "NAD83(2011)",
        [
            0.9910, -1.9072, -0.5129, 0.0, 0.0, 0.0, 0.02579, 0.00965, 0.01166, 0.0000532,
            -0.0007423, -0.0000316, 0.0, 0.0, 1997.0,
        ],
    ),
    (
        "NAD83(PA11)",
        [
            0.90557,
            -2.01999,
            -0.55165,
            -0.00069,
            0.00070,
            -0.00046,
            0.02761633,
            0.01369255,
            0.00277265,
            -0.00039747,
            0.00102214,
            -0.00216627,
            -0.61504e-9,
            0.18201e-9,
            1997.0,
        ],
    ),
    (
        "NAD83(MA11)",
        [
            0.90557,
            -2.01999,
            -0.55165,
            -0.00069,
            0.00070,
            -0.00046,
            0.02884633,
            0.01064355,
            0.00898865,
            -0.00003347,
            0.00012014,
            -0.00032727,
            -0.61504e-9,
            0.18201e-9,
            1997.0,
        ],
    ),
    (
        "ITRF2020",
        [
            -0.01290,
            0.00241,
            0.02827,
            -0.00079,
            0.00070,
            0.00124,
            -0.00029978,
            0.00042037,
            0.00031714,
            -0.00001347,
            0.00001514,
            0.00001973,
            0.05109e-9,
            0.07201e-9,
            2010.0,
        ],
    ),
    (
        "ITRF2014",
        [
            -0.01430,
            0.00201,
            0.02867,
            -0.00079,
            0.00060,
            0.00144,
            -0.00029978,
            0.00042037,
            0.00031714,
            -0.00001347,
            0.00001514,
            0.00001973,
            -0.36891e-9,
            0.07201e-9,
            2010.0,
        ],
    ),
    (
        "ITRF2008",
        [
            -0.00243,
            -0.00389,
            0.01365,
            -0.00079,
            0.00060,
            0.00134,
            -0.00012467,
            0.00022355,
            0.00006065,
            -0.00001347,
            0.00001514,
            0.00001973,
            -1.71504e-9,
            0.10201e-9,
            1997.0,
        ],
    ),
    (
        "ITRF2005",
        [
            -0.00533,
            -0.00479,
            0.00895,
            -0.00049,
            0.00060,
            0.00134,
            -0.00012467,
            0.00022355,
            0.00006065,
            -0.00001347,
            0.00001514,
            0.00001973,
            -0.77504e-9,
            0.10201e-9,
            1997.0,
        ],
    ),
    (
        "ITRF2000",
        [
            -0.00463,
            -0.00589,
            0.00855,
            -0.00069,
            0.00070,
            -0.00046,
            -0.00012467,
            0.00022355,
            0.00006065,
            -0.00001347,
            0.00001514,
            0.00001973,
            -0.61504e-9,
            0.18201e-9,
            1997.0,
        ],
    ),
    (
        "WGS84(G1674)",
        [
            -0.0087,
            0.00091,
            0.02707,
            -0.00079,
            0.00060,
            0.00134,
            -0.00056978,
            0.00069037,
            -0.00006286,
            -0.00001347,
            0.00001514,
            0.00001973,
            6.51109e-9,
            0.10201e-9,
            2010.0,
        ],
    ),
    (
        "WGS84(G1150)",
        [
            -0.0058,
            -0.00019,
            -0.00513,
            -0.00069,
            0.00070,
            -0.00046,
            -0.00029978,
            0.00042037,
            0.00031714,
            -0.00001347,
            0.00001514,
            0.00001973,
            4.83109e-9,
            0.18201e-9,
            2010.0,
        ],
    ),
];

/// The set's parameters at `date`, with rotations in radians and the scale as 1 + s.
fn at(s: &Set, date: f64, sign: f64) -> ([f64; 3], [f64; 3], f64) {
    let dt = date - s[14];
    let t = [0, 1, 2].map(|i| sign * (s[i] + s[3 + i] * dt));
    let r = [0, 1, 2].map(|i| sign * (s[6 + i] + s[9 + i] * dt) / RHOSEC);
    (t, r, 1.0 + sign * (s[12] + s[13] * dt))
}

fn apply((t, r, ds): ([f64; 3], [f64; 3], f64), x: [f64; 3]) -> [f64; 3] {
    [
        t[0] + ds * x[0] + r[2] * x[1] - r[1] * x[2],
        t[1] - r[2] * x[0] + ds * x[1] + r[0] * x[2],
        t[2] + r[1] * x[0] - r[0] * x[1] + ds * x[2],
    ]
}

/// Transforms ECEF `x` at `date` from one frame to another through ITRF94, as HTDP does.
pub fn transform(from: &str, to: &str, x: [f64; 3], date: f64) -> Option<[f64; 3]> {
    let set = |f: &str| SETS.iter().find(|(n, _)| *n == f).map(|(_, s)| s);
    let (a, b) = (set(from)?, set(to)?);
    if from == to {
        return Some(x);
    }
    // toit94 (negated parameters), then frit94.
    let x94 = apply(at(a, date, -1.0), x);
    Some(apply(at(b, date, 1.0), x94))
}
