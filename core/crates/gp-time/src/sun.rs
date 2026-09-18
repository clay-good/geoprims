//! The NOAA solar algorithm (after Meeus, Astronomical Algorithms, ch. 25 and
//! 28): declination, equation of time, position, and the times the sun
//! crosses an altitude. About 0.01° in position and well under a minute in
//! rise and set times between ±72° latitude; UT1 is taken as UTC.

use libm::{acos, asin, atan2, cos, sin, sqrt, tan};

const RAD: f64 = core::f64::consts::PI / 180.0;

/// Apparent sunrise and sunset altitude: refraction 34′ plus semidiameter 16′.
pub const SUNRISE_ALTITUDE: f64 = -0.833;

/// Declination (deg) and equation of time (minutes) at Julian date `jd` (UT).
pub fn declination_eot(jd: f64) -> (f64, f64) {
    let t = (jd - 2_451_545.0) / 36_525.0;
    let l0 = (280.466_46 + t * (36_000.769_83 + 0.000_303_2 * t)).rem_euclid(360.0);
    let m = 357.529_11 + t * (35_999.050_29 - 0.000_153_7 * t);
    let e = 0.016_708_634 - t * (0.000_042_037 + 0.000_000_126_7 * t);
    let c = sin(m * RAD) * (1.914_602 - t * (0.004_817 + 0.000_014 * t))
        + sin(2.0 * m * RAD) * (0.019_993 - 0.000_101 * t)
        + sin(3.0 * m * RAD) * 0.000_289;
    let omega = 125.04 - 1934.136 * t;
    let lambda = l0 + c - 0.005_69 - 0.004_78 * sin(omega * RAD);
    let eps0 =
        23.0 + (26.0 + (21.448 - t * (46.815 + t * (0.000_59 - t * 0.001_813))) / 60.0) / 60.0;
    let eps = eps0 + 0.002_56 * cos(omega * RAD);
    let decl = asin(sin(eps * RAD) * sin(lambda * RAD)) / RAD;
    let y = tan(eps * RAD / 2.0).powi(2);
    let (l0r, mr) = (l0 * RAD, m * RAD);
    let eot = y * sin(2.0 * l0r) - 2.0 * e * sin(mr) + 4.0 * e * y * sin(mr) * cos(2.0 * l0r)
        - 0.5 * y * y * sin(4.0 * l0r)
        - 1.25 * e * e * sin(2.0 * mr);
    (decl, 4.0 * eot / RAD)
}

/// Position of the sun for an observer at `jd` (UT).
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub elevation: f64,
    pub refraction: f64,
    pub azimuth: f64,
    pub declination: f64,
    pub eot: f64,
    pub hour_angle: f64,
}

/// NOAA atmospheric refraction (deg) for a true elevation (deg).
pub fn refraction(e: f64) -> f64 {
    let arcsec = if e > 85.0 {
        0.0
    } else if e > 5.0 {
        let t = tan(e * RAD);
        58.1 / t - 0.07 / t.powi(3) + 0.000_086 / t.powi(5)
    } else if e > -0.575 {
        1735.0 + e * (-518.2 + e * (103.4 + e * (-12.79 + e * 0.711)))
    } else {
        -20.772 / tan(e * RAD)
    };
    arcsec / 3600.0
}

pub fn position(lat: f64, lon: f64, jd: f64) -> Position {
    let (decl, eot) = declination_eot(jd);
    let minutes = (jd + 0.5).fract() * 1440.0;
    let tst = (minutes + eot + 4.0 * lon).rem_euclid(1440.0);
    let ha = tst / 4.0 - 180.0;
    let (phi, d, h) = (lat * RAD, decl * RAD, ha * RAD);
    let cz = (sin(phi) * sin(d) + cos(phi) * cos(d) * cos(h)).clamp(-1.0, 1.0);
    let zenith = acos(cz) / RAD;
    let az = (atan2(sin(h), cos(h) * sin(phi) - tan(d) * cos(phi)) / RAD + 180.0).rem_euclid(360.0);
    let elevation = 90.0 - zenith;
    Position {
        elevation,
        refraction: refraction(elevation),
        azimuth: az,
        declination: decl,
        eot,
        hour_angle: ha,
    }
}

/// Where the sun stands relative to an altitude all day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Crossing {
    /// Minutes from `day_jd` (UT) of the rising and setting crossings.
    Times(f64, f64),
    AlwaysAbove,
    AlwaysBelow,
}

/// Transit (solar noon) nearest `near_jd`, as a Julian date (UT).
pub fn transit(lon: f64, near_jd: f64) -> f64 {
    let mut t = near_jd;
    for _ in 0..3 {
        let day0 = (t - 0.5).floor() + 0.5;
        let (_, eot) = declination_eot(t);
        let mut noon = day0 + (720.0 - 4.0 * lon - eot) / 1440.0;
        while noon - near_jd > 0.5 {
            noon -= 1.0;
        }
        while near_jd - noon > 0.5 {
            noon += 1.0;
        }
        t = noon;
    }
    t
}

/// The rising and setting crossings of `altitude` around the transit at
/// `noon_jd`, each refined by recomputing the sun at the event time.
pub fn crossings(lat: f64, lon: f64, noon_jd: f64, altitude: f64) -> Crossing {
    let ha_at = |jd: f64| {
        let (decl, _) = declination_eot(jd);
        let (phi, d) = (lat * RAD, decl * RAD);
        let x = (sin(altitude * RAD) - sin(phi) * sin(d)) / (cos(phi) * cos(d));
        if x < -1.0 {
            Err(Crossing::AlwaysAbove)
        } else if x > 1.0 {
            Err(Crossing::AlwaysBelow)
        } else {
            Ok(acos(x) / RAD)
        }
    };
    let event = |sign: f64| -> Result<f64, Crossing> {
        let mut jd = noon_jd;
        let mut ha = ha_at(jd)?;
        for _ in 0..4 {
            jd = noon_jd + sign * ha * 4.0 / 1440.0;
            ha = ha_at(jd)?;
        }
        // Account for the transit drift between noon and the event.
        let local_noon = transit(lon, jd);
        Ok(local_noon + sign * ha * 4.0 / 1440.0)
    };
    match (event(-1.0), event(1.0)) {
        (Ok(r), Ok(s)) => Crossing::Times(r, s),
        (Err(c), _) | (_, Err(c)) => c,
    }
}

/// Dip of the sea horizon (deg) for an observer `h` meters up.
pub fn horizon_dip(h: f64) -> f64 {
    if h <= 0.0 { 0.0 } else { 0.0293 * sqrt(h) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equinox_and_solstice() {
        // 2026-06-21 12:00 UT: declination near +23.44°.
        let (d, _) = declination_eot(2_461_213.0);
        assert!((d - 23.44).abs() < 0.02, "{d}");
        // Early November: equation of time near +16.4 min.
        let (_, e) = declination_eot(2_461_347.0);
        assert!((e - 16.4).abs() < 0.2, "{e}");
    }

    #[test]
    fn polar_states() {
        let noon = transit(-156.79, 2_461_396.0);
        assert_eq!(
            crossings(71.29, -156.79, noon, SUNRISE_ALTITUDE),
            Crossing::AlwaysBelow
        );
        assert!(matches!(
            crossings(71.29, -156.79, noon, -6.0),
            Crossing::Times(_, _)
        ));
    }
}
