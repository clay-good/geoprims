//! Helmert 7- and 14-parameter (time-dependent) transformations in the
//! position-vector and coordinate-frame conventions (IOGP Guidance Note 7-2,
//! §4.2.3 and §4.2.5: EPSG methods 1032, 1033, 1053, and 1056), and the IERS
//! ITRF2020 parameter sets.

use core::f64::consts::PI;

/// One arc-second in radians.
pub const ARCSEC: f64 = PI / 180.0 / 3600.0;

/// The rotation sign convention. They differ only in the sign of the rotations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    /// EPSG 1033/1053: rotations turn the position vector (IERS convention).
    PositionVector,
    /// EPSG 1032/1056: rotations turn the coordinate frame.
    CoordinateFrame,
}

/// Seven parameters: translations (m), rotations (arc-seconds), scale (ppm).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Params {
    pub t: [f64; 3],
    pub r: [f64; 3],
    pub ds: f64,
}

impl Params {
    fn add_scaled(self, o: Params, k: f64) -> Params {
        Params {
            t: [0, 1, 2].map(|i| self.t[i] + o.t[i] * k),
            r: [0, 1, 2].map(|i| self.r[i] + o.r[i] * k),
            ds: self.ds + o.ds * k,
        }
    }
}

/// A time-dependent Helmert transformation; a 7-parameter one has zero rates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Helmert {
    pub p: Params,
    /// Rates per year, in the same units.
    pub rate: Params,
    /// Parameter reference epoch (decimal year).
    pub t0: f64,
    pub convention: Convention,
}

impl Helmert {
    /// The parameters at coordinate epoch `t`: p + rate · (t − t0).
    pub fn at(&self, t: f64) -> Params {
        self.p.add_scaled(self.rate, t - self.t0)
    }

    /// The linearized rotation matrix R of GN 7-2 (without the scale).
    fn rotation(&self, p: &Params) -> [[f64; 3]; 3] {
        let s = match self.convention {
            Convention::PositionVector => 1.0,
            Convention::CoordinateFrame => -1.0,
        };
        let [rx, ry, rz] = p.r.map(|r| s * r * ARCSEC);
        [[1.0, -rz, ry], [rz, 1.0, -rx], [-ry, rx, 1.0]]
    }

    /// V_T = M · R · V_S + T at coordinate epoch `t`, with M = 1 + dS.
    pub fn forward(&self, v: [f64; 3], t: f64) -> [f64; 3] {
        let p = self.at(t);
        let r = self.rotation(&p);
        let m = 1.0 + p.ds * 1e-6;
        [0, 1, 2].map(|i| m * (r[i][0] * v[0] + r[i][1] * v[1] + r[i][2] * v[2]) + p.t[i])
    }

    /// The exact reverse, V_S = R⁻¹ · (V_T − T) / M (GN 7-2 "Reversibility"),
    /// rather than the approximation that negates the parameters.
    pub fn reverse(&self, v: [f64; 3], t: f64) -> [f64; 3] {
        let p = self.at(t);
        let r = self.rotation(&p);
        let m = 1.0 + p.ds * 1e-6;
        let b = [0, 1, 2].map(|i| (v[i] - p.t[i]) / m);
        solve3(r, b)
    }
}

/// Solves A x = b for a well-conditioned 3 × 3 matrix by Cramer's rule.
fn solve3(a: [[f64; 3]; 3], b: [f64; 3]) -> [f64; 3] {
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(a);
    [0, 1, 2].map(|j| {
        let mut m = a;
        for i in 0..3 {
            m[i][j] = b[i];
        }
        det(m) / d
    })
}

/// A published ITRF2020 → past-ITRF parameter set (IERS, position-vector
/// convention), in the table's units: mm, ppb, and milliarcseconds.
struct IersRow {
    frame: &'static str,
    p: [f64; 7],
    rate: [f64; 7],
}

/// "Transformation parameters from ITRF2020 to past ITRFs" (IERS/IGN, epoch 2015.0).
const ITRF2020_TO: &[IersRow] = &[
    IersRow {
        frame: "ITRF2014",
        p: [-1.4, -0.9, 1.4, -0.42, 0.00, 0.00, 0.00],
        rate: [0.0, -0.1, 0.2, 0.00, 0.00, 0.00, 0.00],
    },
    IersRow {
        frame: "ITRF2008",
        p: [0.2, 1.0, 3.3, -0.29, 0.00, 0.00, 0.00],
        rate: [0.0, -0.1, 0.1, 0.03, 0.00, 0.00, 0.00],
    },
    IersRow {
        frame: "ITRF2005",
        p: [2.7, 0.1, -1.4, 0.65, 0.00, 0.00, 0.00],
        rate: [0.3, -0.1, 0.1, 0.03, 0.00, 0.00, 0.00],
    },
    IersRow {
        frame: "ITRF2000",
        p: [-0.2, 0.8, -34.2, 2.25, 0.00, 0.00, 0.00],
        rate: [0.1, 0.0, -1.7, 0.11, 0.00, 0.00, 0.00],
    },
    IersRow {
        frame: "ITRF97",
        p: [6.5, -3.9, -77.9, 3.98, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF96",
        p: [6.5, -3.9, -77.9, 3.98, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF94",
        p: [6.5, -3.9, -77.9, 3.98, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF93",
        p: [-65.8, 1.9, -71.3, 4.47, -3.36, -4.33, 0.75],
        rate: [-2.8, -0.2, -2.3, 0.12, -0.11, -0.19, 0.07],
    },
    IersRow {
        frame: "ITRF92",
        p: [14.5, -1.9, -85.9, 3.27, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF91",
        p: [26.5, 12.1, -91.9, 4.67, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF90",
        p: [24.5, 8.1, -107.9, 4.97, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF89",
        p: [29.5, 32.1, -145.9, 8.37, 0.00, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
    IersRow {
        frame: "ITRF88",
        p: [24.5, -3.9, -169.9, 11.47, 0.10, 0.00, 0.36],
        rate: [0.1, -0.6, -3.1, 0.12, 0.00, 0.00, 0.02],
    },
];

/// Every ITRF realization this module transforms between.
pub const ITRF_FRAMES: &[&str] = &[
    "ITRF2020", "ITRF2014", "ITRF2008", "ITRF2005", "ITRF2000", "ITRF97", "ITRF96", "ITRF94",
    "ITRF93", "ITRF92", "ITRF91", "ITRF90", "ITRF89", "ITRF88",
];

/// The IERS transformation from ITRF2020 to `frame`, converted to meters,
/// arc-seconds, and ppm. `None` for ITRF2020 itself or an unknown frame.
pub fn itrf2020_to(frame: &str) -> Option<Helmert> {
    let row = ITRF2020_TO.iter().find(|r| r.frame == frame)?;
    let conv = |v: &[f64; 7]| Params {
        t: [v[0] / 1000.0, v[1] / 1000.0, v[2] / 1000.0],
        ds: v[3] / 1000.0,
        r: [v[4] / 1000.0, v[5] / 1000.0, v[6] / 1000.0],
    };
    Some(Helmert {
        p: conv(&row.p),
        rate: conv(&row.rate),
        t0: 2015.0,
        convention: Convention::PositionVector,
    })
}

/// Transforms ECEF coordinates at epoch `t` from one ITRF realization to
/// another through ITRF2020 (reverse of ITRF2020 → from, then ITRF2020 → to).
pub fn itrf_transform(from: &str, to: &str, v: [f64; 3], t: f64) -> Option<[f64; 3]> {
    let known = |f: &str| ITRF_FRAMES.contains(&f);
    if !known(from) || !known(to) {
        return None;
    }
    let mid = match itrf2020_to(from) {
        Some(h) => h.reverse(v, t),
        None => v,
    };
    Some(match itrf2020_to(to) {
        Some(h) => h.forward(mid, t),
        None => mid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() <= tol)
    }

    #[test]
    fn iogp_guidance_note_examples() {
        // GN 7-2 §4.2.3: WGS 72 → WGS 84 (EPSG 1238), both conventions.
        let pv = Helmert {
            p: Params {
                t: [0.0, 0.0, 4.5],
                r: [0.0, 0.0, 0.554],
                ds: 0.219,
            },
            rate: Params::default(),
            t0: 0.0,
            convention: Convention::PositionVector,
        };
        let src = [3_657_660.66, 255_768.55, 5_201_382.11];
        let want = [3_657_660.78, 255_778.43, 5_201_387.75];
        // The note prints results to the centimeter; its X (…660.78) is 0.774 by
        // hand, M·(X − rz·Y) = 1.000000219 × 3,657,659.973, so compare within 1 cm.
        assert!(
            close(pv.forward(src, 0.0), want, 0.01),
            "{:?}",
            pv.forward(src, 0.0)
        );
        let cf = Helmert {
            p: Params {
                r: [0.0, 0.0, -0.554],
                ..pv.p
            },
            convention: Convention::CoordinateFrame,
            ..pv
        };
        assert_eq!(pv.forward(src, 0.0), cf.forward(src, 0.0));
        // §4.2.5: ITRF2008 → GDA94 (EPSG 6276) at epoch 2013.90, coordinate frame.
        let td = Helmert {
            p: Params {
                t: [-0.08468, -0.01942, 0.03201],
                r: [-0.0004254, 0.0022578, 0.0024015],
                ds: 0.00971,
            },
            rate: Params {
                t: [0.00142, 0.00134, 0.00090],
                r: [0.0015461, 0.0011820, 0.0011551],
                ds: 0.000109,
            },
            t0: 1994.0,
            convention: Convention::CoordinateFrame,
        };
        let s = [-3_789_470.710, 4_841_770.404, -1_690_893.952];
        let got = td.forward(s, 2013.9);
        assert!(
            close(got, [-3_789_470.004, 4_841_770.686, -1_690_895.108], 0.0005),
            "{got:?}"
        );
        // The exact reverse returns the source.
        assert!(close(td.reverse(got, 2013.9), s, 1e-9));
    }
}
