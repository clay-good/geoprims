//! Sidereal time by IAU 2006/2000A (IERS Conventions 2010, chapter 5): the
//! Earth rotation angle (eq. 5.15), Greenwich mean sidereal time (eq. 5.32),
//! and apparent sidereal time with the equation of the equinoxes; and
//! Meeus's IAU 1982 mean sidereal time for comparison.

use crate::nutation::{AS2R, equinox_complement, mean_obliquity, nutation_longitude};
use std::f64::consts::TAU;

const J2000: f64 = 2_451_545.0;

/// Sidereal angles, radians in [0, 2π), and the equation of the equinoxes.
pub struct Sidereal {
    pub era: f64,
    pub gmst: f64,
    pub gast: f64,
    pub ee: f64,
    pub gmst82: f64,
}

/// Sidereal angles for a UT1 instant (a Julian day number and a fraction,
/// kept apart for precision) and TT Julian centuries since J2000.0.
pub fn sidereal(ut1_day: f64, ut1_frac: f64, t_tt: f64) -> Sidereal {
    let tu = (ut1_day - J2000) + ut1_frac;
    // IERS 2010 eq. 5.15: the day fraction kept apart from the rest.
    let frac = ut1_day.fract() + ut1_frac;
    let era = (TAU * (frac + 0.779_057_273_264_0 + 0.002_737_811_911_354_48 * tu)).rem_euclid(TAU);
    let t = t_tt;
    let poly = (((((-0.000_000_036_8 * t - 0.000_029_956) * t - 0.000_000_44) * t + 1.391_581_7)
        * t
        + 4_612.156_534)
        * t
        + 0.014_506)
        * AS2R;
    let gmst = (era + poly).rem_euclid(TAU);
    let ee = nutation_longitude(t) * libm::cos(mean_obliquity(t)) + equinox_complement(t);
    let gast = (gmst + ee).rem_euclid(TAU);
    // Meeus 12.4, with T in UT1 Julian centuries.
    let tu_c = tu / 36_525.0;
    let gmst82 = (280.460_618_37 + 360.985_647_366_29 * tu + 0.000_387_933 * tu_c * tu_c
        - tu_c * tu_c * tu_c / 38_710_000.0)
        .to_radians()
        .rem_euclid(TAU);
    Sidereal {
        era,
        gmst,
        gast,
        ee,
        gmst82,
    }
}
