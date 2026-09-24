//! State Plane Coordinate System of 1983 (geodesy/projections spec): the 124
//! NGS zones on GRS 80, from the EPSG dataset (generated table in
//! `spcs83_zones.rs`). Transverse Mercator uses Krüger's series (`tm`);
//! Lambert Conic Conformal (2SP) and Hotine Oblique Mercator (variant A) follow
//! IOGP Guidance Note 7-2 (EPSG methods 9802 and 9812).

use crate::proj::{Hotine, Lcc, TmGrid};
use libm::{atan2, cos, sin, sqrt};

pub use crate::spcs83_zones::ZONES;

/// GRS 80, the NAD83 ellipsoid.
pub const GRS80_A: f64 = 6_378_137.0;
pub const GRS80_F: f64 = 1.0 / 298.257_222_101;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Proj {
    Tm {
        lat0: f64,
        lon0: f64,
        k0: f64,
        fe: f64,
        fn_: f64,
    },
    Lcc {
        lat0: f64,
        lon0: f64,
        lat1: f64,
        lat2: f64,
        fe: f64,
        fn_: f64,
    },
    OmercA {
        latc: f64,
        lonc: f64,
        alpha: f64,
        gamma: f64,
        k0: f64,
        fe: f64,
        fn_: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// NGS zone code, like "3702".
    pub fips: &'static str,
    pub epsg: u32,
    pub name: &'static str,
    pub proj: Proj,
    /// The feet unit EPSG defines for the zone ("ftUS" or "ft"), if any.
    pub feet: Option<&'static str>,
    /// Area of use: west, south, east, north (degrees).
    pub bbox: [f64; 4],
}

/// A projected point: easting, northing (m), convergence (degrees, the bearing
/// of grid north clockwise from true north), and point scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub e: f64,
    pub n: f64,
    pub convergence: f64,
    pub k: f64,
}

fn e2() -> f64 {
    GRS80_F * (2.0 - GRS80_F)
}

fn omerc_a(latc: f64, lonc: f64, alpha: f64, gamma: f64, k0: f64, fe: f64, fn_: f64) -> Hotine {
    Hotine::new(
        GRS80_A, GRS80_F, latc, lonc, alpha, gamma, k0, fe, fn_, false,
    )
}

/// A Hotine zone's grid point, with convergence and scale from derivatives
/// by Richardson-extrapolated central differences: steps of 1e-3° keep
/// cancellation and the O(h⁴) truncation near 1e-9° (1e-6° steps lost 1e-7°
/// to cancellation).
fn omerc_grid(p: &Hotine, lat: f64, lon: f64) -> Grid {
    let (x, y) = p.en(lat, lon);
    let h = 1e-3;
    let diff = |dlat: f64, dlon: f64| {
        let at = |s: f64| {
            let (xp, yp) = p.en(lat + s * dlat, lon + s * dlon);
            let (xm, ym) = p.en(lat - s * dlat, lon - s * dlon);
            ((xp - xm) / (2.0 * s), (yp - ym) / (2.0 * s))
        };
        let (x1, y1) = at(h);
        let (x2, y2) = at(h / 2.0);
        ((4.0 * x2 - x1) / 3.0, (4.0 * y2 - y1) / 3.0)
    };
    let (xn, yn) = diff(1.0, 0.0);
    let (xe, ye) = diff(0.0, 1.0);
    let e2 = e2();
    let phi = lat.to_radians();
    let w = sqrt(1.0 - e2 * sin(phi) * sin(phi));
    // Length of one degree along the parallel on the ellipsoid.
    let par = 1f64.to_radians() * GRS80_A * cos(phi) / w;
    Grid {
        e: x,
        n: y,
        convergence: -atan2(xn, yn).to_degrees(),
        k: xe.hypot(ye) / par,
    }
}

impl Zone {
    pub fn forward(&self, lat: f64, lon: f64) -> Grid {
        match self.proj {
            Proj::Tm {
                lat0,
                lon0,
                k0,
                fe,
                fn_,
            } => {
                let g = TmGrid::new(GRS80_A, GRS80_F, lat0, lon0, k0, fe, fn_).forward(lat, lon);
                Grid {
                    e: g.e,
                    n: g.n,
                    convergence: g.convergence,
                    k: g.k,
                }
            }
            Proj::Lcc {
                lat0,
                lon0,
                lat1,
                lat2,
                fe,
                fn_,
            } => {
                let g = Lcc::two_sp(GRS80_A, GRS80_F, lat0, lon0, lat1, lat2, fe, fn_)
                    .forward(lat, lon);
                Grid {
                    e: g.e,
                    n: g.n,
                    convergence: g.convergence,
                    k: g.k,
                }
            }
            Proj::OmercA {
                latc,
                lonc,
                alpha,
                gamma,
                k0,
                fe,
                fn_,
            } => omerc_grid(&omerc_a(latc, lonc, alpha, gamma, k0, fe, fn_), lat, lon),
        }
    }

    /// (lat, lon) in degrees from easting and northing in meters.
    pub fn inverse(&self, e: f64, n: f64) -> (f64, f64) {
        match self.proj {
            Proj::Tm {
                lat0,
                lon0,
                k0,
                fe,
                fn_,
            } => TmGrid::new(GRS80_A, GRS80_F, lat0, lon0, k0, fe, fn_).inverse(e, n),
            Proj::Lcc {
                lat0,
                lon0,
                lat1,
                lat2,
                fe,
                fn_,
            } => Lcc::two_sp(GRS80_A, GRS80_F, lat0, lon0, lat1, lat2, fe, fn_).inverse(e, n),
            Proj::OmercA {
                latc,
                lonc,
                alpha,
                gamma,
                k0,
                fe,
                fn_,
            } => omerc_a(latc, lonc, alpha, gamma, k0, fe, fn_).inverse(e, n),
        }
    }

    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        let [w, s, e, n] = self.bbox;
        lat >= s
            && lat <= n
            && if w <= e {
                lon >= w && lon <= e
            } else {
                lon >= w || lon <= e
            }
    }

    /// "Pennsylvania South" (without the trailing "zone").
    pub fn short_name(&self) -> &'static str {
        self.name.strip_suffix(" zone").unwrap_or(self.name)
    }
}

/// Finds a zone by NGS code ("3702"), EPSG code, or name ("Pennsylvania South").
pub fn find(key: &str) -> Option<&'static Zone> {
    let k = key.trim();
    let norm = |s: &str| {
        s.to_ascii_lowercase()
            .replace(" zone", "")
            .replace(['&', '.'], "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    ZONES.iter().find(|z| {
        z.fips == k
            || (k.len() == 3 && k.bytes().all(|b| b.is_ascii_digit()) && z.fips == format!("0{k}"))
            || z.epsg.to_string() == k
            || norm(z.name) == norm(k)
    })
}

/// Zones whose area-of-use box contains the point.
pub fn candidates(lat: f64, lon: f64) -> Vec<&'static Zone> {
    ZONES.iter().filter(|z| z.contains(lat, lon)).collect()
}
