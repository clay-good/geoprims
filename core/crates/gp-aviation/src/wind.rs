//! Wind inputs and geometry shared by the wind tools: METAR wind groups,
//! runway designators, and references.

use gp_base::error::ToolError;
use libm::{atan2, cos, sin, sqrt};

/// A parsed wind: direction the wind blows FROM (None when variable), speed and
/// gust in knots, and an optional variable range (from, to) clockwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    pub dir: Option<f64>,
    pub speed: f64,
    pub gust: Option<f64>,
    pub range: Option<(f64, f64)>,
}

const MPS_TO_KT: f64 = 3600.0 / 1852.0;

/// Parses a METAR wind group: `30015KT`, `30015G25KT`, `VRB05KT`, `00000KT`,
/// `27008MPS`. Direction is degrees true (METAR convention).
pub fn parse_metar_wind(s: &str, field: &str) -> Result<Wind, ToolError> {
    let bad = || {
        ToolError::invalid(field, format!("\"{s}\" is not a METAR wind group."))
            .hint("Examples: 30015KT, 30015G25KT, VRB05KT, 00000KT, 27008MPS")
    };
    let u = s.trim().to_ascii_uppercase();
    let (body, k) = if let Some(b) = u.strip_suffix("KT") {
        (b, 1.0)
    } else if let Some(b) = u.strip_suffix("MPS") {
        (b, MPS_TO_KT)
    } else {
        return Err(bad());
    };
    if body.len() < 5 {
        return Err(bad());
    }
    let (d, rest) = body.split_at(3);
    let dir = match d {
        "VRB" => None,
        _ if d.bytes().all(|b| b.is_ascii_digit()) => {
            let v: f64 = d.parse().map_err(|_| bad())?;
            if v > 360.0 {
                return Err(bad());
            }
            Some(v % 360.0)
        }
        _ => return Err(bad()),
    };
    let (sp, gust) = match rest.split_once('G') {
        Some((a, g)) => (a, Some(g)),
        None => (rest, None),
    };
    let num = |t: &str| -> Result<f64, ToolError> {
        if (2..=3).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_digit()) {
            Ok(t.parse::<f64>().map_err(|_| bad())? * k)
        } else {
            Err(bad())
        }
    };
    Ok(Wind {
        dir,
        speed: num(sp)?,
        gust: gust.map(num).transpose()?,
        range: None,
    })
}

/// Parses a variable-direction group such as `280V340`.
pub fn parse_range(s: &str, field: &str) -> Result<(f64, f64), ToolError> {
    let bad = || {
        ToolError::invalid(
            field,
            format!("\"{s}\" is not a variable-wind range like 280V340."),
        )
    };
    let (a, b) = s
        .trim()
        .to_ascii_uppercase()
        .split_once('V')
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .ok_or_else(bad)?;
    let deg = |t: &str| -> Result<f64, ToolError> {
        if t.len() == 3 && t.bytes().all(|b| b.is_ascii_digit()) {
            let v: f64 = t.parse().map_err(|_| bad())?;
            if v <= 360.0 {
                Ok(v % 360.0)
            } else {
                Err(bad())
            }
        } else {
            Err(bad())
        }
    };
    Ok((deg(&a)?, deg(&b)?))
}

/// A runway: its designator heading in degrees and whether it is true (`T` suffix).
pub struct Runway {
    pub heading: f64,
    pub true_ref: bool,
}

/// Parses a designator: `27`, `09L`, `9`, `36T`, `RWY 27R`.
pub fn parse_designator(s: &str, field: &str) -> Result<Runway, ToolError> {
    let u = s.trim().to_ascii_uppercase();
    let u = u
        .strip_prefix("RUNWAY")
        .or_else(|| u.strip_prefix("RWY"))
        .unwrap_or(&u)
        .trim()
        .to_owned();
    let digits: String = u.chars().take_while(char::is_ascii_digit).collect();
    let suffix = &u[digits.len()..];
    let n: u32 = digits.parse().unwrap_or(0);
    if !(1..=36).contains(&n) || digits.len() > 2 || !["", "L", "R", "C", "T"].contains(&suffix) {
        return Err(ToolError::invalid(
            field,
            format!("\"{s}\" is not a runway designator (01 to 36, optional L, C, R, or T)."),
        ));
    }
    Ok(Runway {
        heading: f64::from(n * 10) % 360.0,
        true_ref: suffix == "T",
    })
}

/// Headwind (positive) or tailwind (negative), and crosswind (positive = from
/// the right), for wind FROM `wind_dir` at `speed` on a runway heading.
pub fn components(runway: f64, wind_dir: f64, speed: f64) -> (f64, f64) {
    let d = (wind_dir - runway).to_radians();
    (speed * cos(d), speed * sin(d))
}

/// Worst-case crosswind magnitude and tailwind (≥ 0) over wind directions in
/// the clockwise range `from`→`to` (or all directions when `range` is None).
pub fn worst_case(runway: f64, speed: f64, range: Option<(f64, f64)>) -> (f64, f64) {
    let Some((from, to)) = range else {
        return (speed, speed);
    };
    let span = (to - from).rem_euclid(360.0);
    // Extremes occur at the range ends or where the wind is square to or along the runway.
    let mut cands = vec![from, to];
    for k in 0..4 {
        let a = (runway + 90.0 * f64::from(k)).rem_euclid(360.0);
        if (a - from).rem_euclid(360.0) <= span {
            cands.push(a);
        }
    }
    let mut cross: f64 = 0.0;
    let mut tail: f64 = 0.0;
    for a in cands {
        let (h, x) = components(runway, a, speed);
        cross = cross.max(x.abs());
        tail = tail.max(-h);
    }
    (cross, tail)
}

/// The wind triangle: wind correction angle (deg, signed) and groundspeed for a
/// course, TAS, and wind FROM `wd` at `ws`. None when the crosswind exceeds TAS.
pub fn heading_groundspeed(course: f64, tas: f64, wd: f64, ws: f64) -> Option<(f64, f64)> {
    let d = (wd - course).to_radians();
    let s = ws / tas * sin(d);
    if s.abs() > 1.0 {
        return None;
    }
    let wca = libm::asin(s);
    Some((wca.to_degrees(), tas * cos(wca) - ws * cos(d)))
}

/// Wind FROM direction (deg) and speed from air and ground vectors.
pub fn find_wind(heading: f64, tas: f64, track: f64, gs: f64) -> (f64, f64) {
    let (h, t) = (heading.to_radians(), track.to_radians());
    let (we, wn) = (gs * sin(t) - tas * sin(h), gs * cos(t) - tas * cos(h));
    let speed = sqrt(we * we + wn * wn);
    let toward = atan2(we, wn).to_degrees();
    (toward + 180.0, speed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metar_groups() {
        assert_eq!(
            parse_metar_wind("30015G25KT", "/w").unwrap(),
            Wind {
                dir: Some(300.0),
                speed: 15.0,
                gust: Some(25.0),
                range: None
            }
        );
        assert_eq!(parse_metar_wind("VRB05KT", "/w").unwrap().dir, None);
        assert_eq!(parse_metar_wind("00000KT", "/w").unwrap().speed, 0.0);
        assert!(
            (parse_metar_wind("27010MPS", "/w").unwrap().speed - 19.438_444_924_406).abs() < 1e-9
        );
        for bad in ["300KT", "37015KT", "30015", "ABC15KT", "30015G2KT"] {
            assert!(parse_metar_wind(bad, "/w").is_err(), "{bad}");
        }
        assert_eq!(parse_range("280V340", "/v").unwrap(), (280.0, 340.0));
    }

    #[test]
    fn designators() {
        assert_eq!(parse_designator("27", "/r").unwrap().heading, 270.0);
        assert_eq!(parse_designator("RWY 09L", "/r").unwrap().heading, 90.0);
        assert_eq!(parse_designator("36", "/r").unwrap().heading, 0.0);
        assert!(parse_designator("36T", "/r").unwrap().true_ref);
        for bad in ["0", "37", "27X", "270"] {
            assert!(parse_designator(bad, "/r").is_err(), "{bad}");
        }
    }

    #[test]
    fn runway_27_scenarios() {
        let (h, x) = components(270.0, 300.0, 15.0);
        assert!((h - 12.990_381).abs() < 1e-6 && (x - 7.5).abs() < 1e-12);
        assert!((components(270.0, 300.0, 25.0).1 - 12.5).abs() < 1e-12);
        assert_eq!(worst_case(270.0, 8.0, None), (8.0, 8.0));
        let (x, t) = worst_case(270.0, 10.0, Some((280.0, 340.0)));
        assert!((x - 10.0 * (70f64).to_radians().sin()).abs() < 1e-12 && t == 0.0);
        let (x, t) = worst_case(270.0, 10.0, Some((350.0, 100.0)));
        assert!(
            (x - 10.0).abs() < 1e-12 && (t - 10.0).abs() < 1e-12,
            "{x} {t}"
        );
    }

    #[test]
    fn wind_triangle_scenarios() {
        let (wca, gs) = heading_groundspeed(90.0, 120.0, 30.0, 20.0).unwrap();
        assert!((wca + 8.298_921).abs() < 1e-5 && (gs - 108.743_42).abs() < 1e-4);
        assert!(heading_groundspeed(90.0, 30.0, 0.0, 40.0).is_none());
        let (d, s) = find_wind(90.0 + wca, 120.0, 90.0, gs);
        assert!(
            ((d - 30.0).rem_euclid(360.0)).min((30.0 - d).rem_euclid(360.0)) < 1e-9
                && (s - 20.0).abs() < 1e-9
        );
    }
}
