//! Apparent geocentric places of the Sun and the navigational planets from
//! VSOP87D, and their Greenwich hour angle and declination: the planet at
//! t − τ less the Earth at t (light time), annual aberration (Meeus eq.
//! 23.2), the VSOP-to-FK5 correction (Meeus eq. 32.3), nutation in
//! longitude, and the true obliquity (IAU 2006/2000A). The Moon comes from
//! ELP 2000-82B at t − τ, carried from the J2000 ecliptic to the true equator
//! of date; for a geocentric body that close, light time from the Moon's
//! geocentric motion is the whole of the aberration.

use crate::moon::moon_j2000;
use crate::nutation::{AS2R, ecliptic_j2000_to_true, mean_obliquity, nutation};
use crate::vsop_tables::*;
use std::f64::consts::{PI, TAU};

/// The bodies with VSOP87 series here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Sun,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Moon,
}

const J2000: f64 = 2_451_545.0;
/// Light time for one au, in days.
const LIGHT_DAY_PER_AU: f64 = 0.005_775_518_3;
/// The astronomical unit (km, IAU 2012) and light's speed (km per day).
const AU_KM: f64 = 149_597_870.7;
const LIGHT_KM_PER_DAY: f64 = 299_792.458 * 86_400.0;

fn eval(s: &Series, tau: f64) -> f64 {
    let mut total = 0.0;
    let mut power = 1.0;
    for terms in s {
        let sum: f64 = terms
            .iter()
            .map(|[a, b, c]| a * libm::cos(b + c * tau))
            .sum();
        total += sum * power;
        power *= tau;
    }
    total
}

/// Heliocentric ecliptic longitude, latitude (radians), and radius (au) of
/// date, at tau Julian millennia of TDB from J2000.0.
fn heliocentric(body: Body, tau: f64) -> (f64, f64, f64) {
    let (l, b, r) = match body {
        Body::Sun => (&EARTH_L, &EARTH_B, &EARTH_R),
        Body::Venus => (&VENUS_L, &VENUS_B, &VENUS_R),
        Body::Mars => (&MARS_L, &MARS_B, &MARS_R),
        Body::Jupiter => (&JUPITER_L, &JUPITER_B, &JUPITER_R),
        Body::Saturn => (&SATURN_L, &SATURN_B, &SATURN_R),
        Body::Moon => unreachable!("the Moon has no heliocentric series"),
    };
    (eval(l, tau), eval(b, tau), eval(r, tau))
}

fn cart(l: f64, b: f64, r: f64) -> [f64; 3] {
    [
        r * libm::cos(b) * libm::cos(l),
        r * libm::cos(b) * libm::sin(l),
        r * libm::sin(b),
    ]
}

/// The Earth's heliocentric position (au) on the true equator and equinox of
/// date: VSOP87D's ecliptic of date, nutated in longitude, by the true
/// obliquity.
pub fn earth_true(jd_tt: f64) -> [f64; 3] {
    let t = (jd_tt - J2000) / 36_525.0;
    let (dpsi, deps) = nutation(t);
    earth_true_with(jd_tt, dpsi, mean_obliquity(t) + deps)
}

/// `earth_true` with the nutation in longitude and true obliquity given,
/// for several nearby instants that share them.
pub fn earth_true_with(jd_tt: f64, dpsi: f64, eps: f64) -> [f64; 3] {
    let tau = (jd_tt - J2000) / 365_250.0;
    let (l, b, r) = heliocentric(Body::Sun, tau);
    let v = cart(l + dpsi, b, r);
    let (s, c) = (libm::sin(eps), libm::cos(eps));
    [v[0], c * v[1] - s * v[2], s * v[1] + c * v[2]]
}

/// An apparent geocentric place: right ascension and declination (radians,
/// true equator and equinox of date) and the distance (au).
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub ra: f64,
    pub dec: f64,
    pub distance: f64,
}

/// The apparent geocentric place of a body at a TT Julian date.
pub fn apparent(body: Body, jd_tt: f64) -> Place {
    if body == Body::Moon {
        return moon(jd_tt);
    }
    let tau = (jd_tt - J2000) / 365_250.0;
    let t = tau * 10.0;
    let (l0, b0, r0) = heliocentric(Body::Sun, tau);
    // Geometric geocentric ecliptic longitude, latitude, and distance.
    let (mut lam, mut beta, distance) = if body == Body::Sun {
        (l0 + PI, -b0, r0)
    } else {
        let e = cart(l0, b0, r0);
        let mut light = 0.0;
        let mut d = [0.0; 3];
        for _ in 0..3 {
            let (l, b, r) = heliocentric(body, tau - light / 365_250.0);
            let p = cart(l, b, r);
            d = [p[0] - e[0], p[1] - e[1], p[2] - e[2]];
            light = LIGHT_DAY_PER_AU * (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        }
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        (
            libm::atan2(d[1], d[0]),
            libm::atan2(d[2], libm::hypot(d[0], d[1])),
            dist,
        )
    };
    // Annual aberration: the Sun's -20.4898″/R, or Meeus 23.2 for a planet.
    if body == Body::Sun {
        lam -= 20.489_8 / r0 * AS2R;
    } else {
        let sun = l0 + PI;
        let k = 20.495_52 * AS2R;
        let e = 0.016_708_634 - 0.000_042_037 * t - 0.000_000_126_7 * t * t;
        let p = (102.937_35 + 1.719_46 * t + 0.000_46 * t * t).to_radians();
        let dl = (-k * libm::cos(sun - lam) + e * k * libm::cos(p - lam)) / libm::cos(beta);
        let db = -k * libm::sin(beta) * (libm::sin(sun - lam) - e * libm::sin(p - lam));
        lam += dl;
        beta += db;
    }
    // VSOP87 dynamical frame to FK5 (Meeus 32.3).
    let lp = lam - (1.397 * t + 0.000_31 * t * t).to_radians();
    lam += (-0.090_33 + 0.039_16 * (libm::cos(lp) + libm::sin(lp)) * libm::tan(beta)) * AS2R;
    beta += 0.039_16 * (libm::cos(lp) - libm::sin(lp)) * AS2R;
    // Nutation and the true obliquity.
    let (dpsi, deps) = nutation(t);
    lam += dpsi;
    let eps = mean_obliquity(t) + deps;
    let ra = libm::atan2(
        libm::sin(lam) * libm::cos(eps) - libm::tan(beta) * libm::sin(eps),
        libm::cos(lam),
    );
    let dec = libm::asin(
        libm::sin(beta) * libm::cos(eps) + libm::cos(beta) * libm::sin(eps) * libm::sin(lam),
    );
    Place {
        ra: ra.rem_euclid(TAU),
        dec,
        distance,
    }
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn moon(jd_tt: f64) -> Place {
    let d = norm(moon_j2000(jd_tt));
    let v = moon_j2000(jd_tt - d / LIGHT_KM_PER_DAY);
    let v = ecliptic_j2000_to_true(v, (jd_tt - J2000) / 36_525.0);
    let d = norm(v);
    Place {
        ra: libm::atan2(v[1], v[0]).rem_euclid(TAU),
        dec: libm::asin(v[2] / d),
        distance: d / AU_KM,
    }
}

/// Greenwich hour angle (radians, 0 to 2π) of a right ascension at an
/// apparent sidereal time.
pub fn gha(gast: f64, ra: f64) -> f64 {
    (gast - ra).rem_euclid(TAU)
}
