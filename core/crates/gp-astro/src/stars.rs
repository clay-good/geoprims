//! Apparent geocentric places of the navigational stars: Hipparcos ICRS
//! positions carried from epoch J1991.25 by proper motion (radial velocity
//! taken as zero), to the true equator and equinox of date by bias,
//! precession, and nutation, then annual parallax, the Sun's light
//! deflection, and annual aberration from the Earth's VSOP87 position and
//! velocity.

pub use crate::star_tables::{STARS, Star};

use crate::apparent::{Place, earth_true_with};
use crate::nutation::{AS2R, Frame};
use std::f64::consts::TAU;

const J2000: f64 = 2_451_545.0;
/// The Hipparcos epoch J1991.25 (TT).
const J1991_25: f64 = 2_448_349.062_5;
/// Light's speed in au per day.
const C_AU_PER_DAY: f64 = 173.144_632_684_7;

/// The Earth's heliocentric position (au) and velocity (au a day) on the
/// true equator and equinox of date, shared by every star at one instant.
pub struct Earth {
    t: f64,
    frame: Frame,
    pos: [f64; 3],
    vel: [f64; 3],
}

impl Earth {
    pub fn at(jd_tt: f64) -> Earth {
        let h = 0.05;
        let t = (jd_tt - J2000) / 36_525.0;
        // One frame for the instant; over ±0.05 day the nutation and
        // obliquity change by less than 1e-9 rad.
        let frame = Frame::at(t);
        let at = |jd| earth_true_with(jd, frame.dpsi, frame.eps_true);
        let a = at(jd_tt - h);
        let b = at(jd_tt + h);
        Earth {
            t,
            pos: at(jd_tt),
            vel: [
                (b[0] - a[0]) / (2.0 * h),
                (b[1] - a[1]) / (2.0 * h),
                (b[2] - a[2]) / (2.0 * h),
            ],
            frame,
        }
    }
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

/// The apparent place of a star (distance in au from the parallax).
pub fn star(s: &Star, earth: &Earth) -> Place {
    let years = (earth.t * 36_525.0 + J2000 - J1991_25) / 365.25;
    let (ra, dec) = (s.ra.to_radians(), s.dec.to_radians());
    let (sa, ca, sd, cd) = (libm::sin(ra), libm::cos(ra), libm::sin(dec), libm::cos(dec));
    let mas = AS2R / 1000.0;
    let (mu_a, mu_d) = (s.pm_ra * mas * years, s.pm_dec * mas * years);
    let v = [
        cd * ca - mu_a * sa - mu_d * sd * ca,
        cd * sa + mu_a * ca - mu_d * sd * sa,
        sd + mu_d * cd,
    ];
    let u = unit(earth.frame.apply(unit(v)));
    // Annual parallax: the star seen from the Earth, not the Sun.
    let p = s.parallax * mas;
    let u = unit([
        u[0] - p * earth.pos[0],
        u[1] - p * earth.pos[1],
        u[2] - p * earth.pos[2],
    ]);
    // Light deflection by the Sun (2GM/c² over 1 au is 1.974e-8 rad), from
    // the Earth's heliocentric direction e: u + g/E (e − (u·e)u)/(1 + u·e).
    let r = (earth.pos[0].powi(2) + earth.pos[1].powi(2) + earth.pos[2].powi(2)).sqrt();
    let e = earth.pos.map(|x| x / r);
    let ue = u[0] * e[0] + u[1] * e[1] + u[2] * e[2];
    let g = 1.974_125_7e-8 / r / (1.0 + ue);
    let u = unit([
        u[0] + g * (e[0] - ue * u[0]),
        u[1] + g * (e[1] - ue * u[1]),
        u[2] + g * (e[2] - ue * u[2]),
    ]);
    // Annual aberration, first order in v/c.
    let b = earth.vel.map(|x| x / C_AU_PER_DAY);
    let ub = u[0] * b[0] + u[1] * b[1] + u[2] * b[2];
    let u = unit([
        u[0] + b[0] - ub * u[0],
        u[1] + b[1] - ub * u[1],
        u[2] + b[2] - ub * u[2],
    ]);
    Place {
        ra: libm::atan2(u[1], u[0]).rem_euclid(TAU),
        dec: libm::asin(u[2]),
        distance: if p > 0.0 { 1.0 / p } else { f64::INFINITY },
    }
}
