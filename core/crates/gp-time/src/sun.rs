//! The NOAA solar algorithm (after Meeus, Astronomical Algorithms, ch. 25 and
//! 28): declination, equation of time, and position, about 0.01° in
//! position; UT1 is taken as UTC. The times the sun crosses an altitude use
//! the NREL SPA (see [`crossing`]), because 0.01° decides grazing twilights.

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

/// One side of a day's crossing of an altitude: the time it happens, or why
/// it does not (the sun stays above, or never reaches, the altitude).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    /// Julian date (UT) of the crossing.
    At(f64),
    Above,
    Below,
}

/// Geometric (unrefracted) elevation from the NREL SPA, with ΔT for the date.
fn spa_elevation(lat: f64, lon: f64, jd: f64) -> f64 {
    let year = 2000.0 + (jd - 2_451_545.0) / 365.25;
    let o = crate::spa::Observer {
        lat,
        lon,
        elevation: 0.0,
        pressure: 1013.25,
        temperature: 12.0,
        atmos_refract: 0.5667,
    };
    crate::spa::position(jd, crate::spa::delta_t(year, 0.5), o).elevation_true
}

/// Golden-section search for the extreme of `f` in [a, b] (the maximum when
/// `max`), to about a second.
fn extreme(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64, max: bool) -> f64 {
    let g = 0.618_033_988_749_894_9;
    let key = |x: f64| if max { -f(x) } else { f(x) };
    let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
    let (mut fc, mut fd) = (key(c), key(d));
    while b - a > 1e-5 {
        if fc < fd {
            (b, d, fd) = (d, c, fc);
            c = b - g * (b - a);
            fc = key(c);
        } else {
            (a, c, fc) = (c, d, fd);
            d = a + g * (b - a);
            fd = key(d);
        }
    }
    f((a + b) / 2.0)
}

/// The rising (`sign` −1) or setting (+1) crossing of `altitude` on one side
/// of the transit near `noon_jd`, by the NREL SPA. The sun's highest and
/// lowest points on that side decide whether it crosses at all (so a sun that
/// grazes the altitude by hundredths of a degree is placed correctly), and
/// bisection between them finds the time to about 0.1 s.
pub fn crossing(lat: f64, lon: f64, noon_jd: f64, altitude: f64, sign: f64) -> Side {
    let e = |jd: f64| spa_elevation(lat, lon, jd) - altitude;
    let noon = transit(lon, noon_jd);
    let low = noon + sign * 0.5;
    // Fast path: the NOAA series (good to about 0.01°) settles every case
    // that is not within 0.05° of grazing, and seeds a 20-minute bracket.
    let culmination = |jd: f64, upper: bool| {
        let (d, _) = declination_eot(jd);
        if upper {
            90.0 - (lat - d).abs()
        } else {
            (lat + d).abs() - 90.0
        }
    };
    let (hi, lo) = (culmination(noon, true), culmination(low, false));
    if hi < altitude - 0.05 {
        return Side::Below;
    }
    if lo > altitude + 0.05 {
        return Side::Above;
    }
    if hi > altitude + 0.05 && lo < altitude - 0.05 {
        let (phi, mut t) = (lat * RAD, noon);
        for _ in 0..4 {
            let d = declination_eot(t).0 * RAD;
            let x =
                ((sin(altitude * RAD) - sin(phi) * sin(d)) / (cos(phi) * cos(d))).clamp(-1.0, 1.0);
            t = noon + sign * acos(x) / RAD * 4.0 / 1440.0;
        }
        let span = 10.0 / 1440.0;
        let (mut below, mut above) = if sign < 0.0 {
            (t - span, t + span)
        } else {
            (t + span, t - span)
        };
        if e(below) < 0.0 && e(above) > 0.0 {
            for _ in 0..16 {
                let mid = (below + above) / 2.0;
                if e(mid) < 0.0 {
                    below = mid;
                } else {
                    above = mid;
                }
            }
            return Side::At((below + above) / 2.0);
        }
    }
    let window = 0.1;
    if extreme(e, noon - window, noon + window, true) < 0.0 {
        return Side::Below;
    }
    if extreme(e, low - window, low + window, false) > 0.0 {
        return Side::Above;
    }
    // Between the lowest and highest points the elevation is monotonic.
    let (mut below, mut above) = (low, noon);
    for _ in 0..40 {
        let mid = (below + above) / 2.0;
        if e(mid) < 0.0 {
            below = mid;
        } else {
            above = mid;
        }
    }
    Side::At((below + above) / 2.0)
}

/// The rising and setting crossings of `altitude` around the transit at
/// `noon_jd` (see [`crossing`]).
pub fn crossings(lat: f64, lon: f64, noon_jd: f64, altitude: f64) -> Crossing {
    match (
        crossing(lat, lon, noon_jd, altitude, -1.0),
        crossing(lat, lon, noon_jd, altitude, 1.0),
    ) {
        (Side::At(r), Side::At(s)) => Crossing::Times(r, s),
        (Side::Below, _) | (_, Side::Below) => Crossing::AlwaysBelow,
        _ => Crossing::AlwaysAbove,
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
