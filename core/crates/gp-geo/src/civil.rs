//! Calendar arithmetic without the host clock or time-zone data (time-scales
//! spec, "Explicit time zones, deterministic data"): proleptic Gregorian day
//! numbers, ISO 8601 parsing and formatting, and the leap-second table.

/// Days since 1970-01-01 for a proleptic Gregorian date (Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date for a day number since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}

pub fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if is_leap_year(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Day of year, 1 to 366.
pub fn day_of_year(days: i64) -> u32 {
    let (y, _, _) = civil_from_days(days);
    (days - days_from_civil(y, 1, 1) + 1) as u32
}

/// A parsed timestamp: the day number, seconds into that day (may reach 60 on
/// a leap-second day), and the UTC offset in minutes when one was written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stamp {
    pub days: i64,
    pub secs: f64,
    pub offset: Option<i32>,
}

/// Parses a UTC offset: `Z`, `+05:30`, `-0500`, `-05`, `UTC-5`, `UTC+5:45`.
pub fn parse_offset(s: &str) -> Result<i32, String> {
    let t = s.trim();
    let t = t
        .strip_prefix("UTC")
        .or_else(|| t.strip_prefix("GMT"))
        .unwrap_or(t)
        .trim();
    if t.is_empty() || t == "Z" || t == "z" {
        return Ok(0);
    }
    let bad = || format!("\"{s}\" is not a UTC offset like -05:00, +0530, or UTC-7.");
    let (sign, rest) = match t.as_bytes()[0] {
        b'+' => (1, &t[1..]),
        b'-' => (-1, &t[1..]),
        _ => return Err(bad()),
    };
    let digits = |x: &str| !x.is_empty() && x.len() <= 2 && x.bytes().all(|b| b.is_ascii_digit());
    let (h, m) = match rest.split_once(':') {
        Some((h, m)) if digits(h) && m.len() == 2 && digits(m) => (h, m),
        None if rest.len() == 4 && rest.bytes().all(|b| b.is_ascii_digit()) => {
            (&rest[..2], &rest[2..])
        }
        None if digits(rest) => (rest, "0"),
        _ => return Err(bad()),
    };
    let (h, m): (i32, i32) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
    if h > 14 || m > 59 || (h == 14 && m > 0) {
        return Err(format!(
            "\"{s}\" is outside the UTC offsets in use (−12:00 to +14:00)."
        ));
    }
    Ok(sign * (h * 60 + m))
}

fn num<T: core::str::FromStr>(s: &str, len: usize) -> Option<T> {
    (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

/// Parses `YYYY-MM-DD`, optionally followed by `T` or a space and
/// `HH:MM[:SS[.fff]]`, and optionally a zone (`Z` or an offset).
pub fn parse_stamp(s: &str) -> Result<Stamp, String> {
    let t = s.trim();
    let bad = || format!("\"{s}\" is not a date and time like 2026-09-18T14:05:00Z.");
    if t.len() < 10 {
        return Err(bad());
    }
    let (date, rest) = t.split_at(10);
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(bad());
    }
    let (y, m, d): (i64, u32, u32) = (
        num(parts[0], 4).ok_or_else(bad)?,
        num(parts[1], 2).ok_or_else(bad)?,
        num(parts[2], 2).ok_or_else(bad)?,
    );
    if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return Err(format!("\"{date}\" is not a calendar date."));
    }
    let days = days_from_civil(y, m, d);
    let rest = rest.trim_start_matches(['T', 't', ' ']);
    if rest.is_empty() {
        return Ok(Stamp {
            days,
            secs: 0.0,
            offset: None,
        });
    }
    // Split the clock from a trailing zone.
    let zone_at = rest.find(['Z', 'z', '+', '-']).unwrap_or(rest.len());
    let (clock, zone) = rest.split_at(zone_at);
    let offset = if zone.is_empty() {
        None
    } else {
        Some(parse_offset(zone)?)
    };
    let c: Vec<&str> = clock.trim().split(':').collect();
    if !(2..=3).contains(&c.len()) {
        return Err(bad());
    }
    let h: u32 = num(c[0], 2).ok_or_else(bad)?;
    let mi: u32 = num(c[1], 2).ok_or_else(bad)?;
    let sec: f64 = match c.get(2) {
        None => 0.0,
        Some(x) => {
            let (whole, frac) = x.split_once('.').unwrap_or((x, ""));
            let w: u32 = num(whole, 2).ok_or_else(bad)?;
            if !frac.bytes().all(|b| b.is_ascii_digit()) || (x.contains('.') && frac.is_empty()) {
                return Err(bad());
            }
            f64::from(w)
                + if frac.is_empty() {
                    0.0
                } else {
                    format!("0.{frac}").parse::<f64>().map_err(|_| bad())?
                }
        }
    };
    if h > 23 || mi > 59 || sec >= 61.0 {
        return Err(format!("\"{clock}\" is not a time of day."));
    }
    if sec >= 60.0 && !(h == 23 && mi == 59 && leap_second_after(days)) {
        return Err(format!(
            "\"{s}\" has a 60th second, but no leap second was inserted then."
        ));
    }
    Ok(Stamp {
        days,
        secs: f64::from(h * 3600 + mi * 60) + sec,
        offset,
    })
}

/// Formats a date.
pub fn date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Formats `days` and seconds of day as ISO 8601 with the given suffix, to
/// the millisecond (trailing zeros dropped). Seconds from 86,400 print as
/// 23:59:60 (a leap second).
pub fn iso(days: i64, secs: f64, suffix: &str) -> String {
    let ms = (secs * 1000.0).round() as i64;
    let (days, ms) = if (86_400_000..86_401_000).contains(&ms) && secs < 86_400.0 {
        (days + 1, ms - 86_400_000)
    } else {
        (days, ms)
    };
    let (h, mi, s, frac) = if ms >= 86_400_000 {
        (23, 59, 60, (ms - 86_400_000).min(999))
    } else {
        (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
    };
    let frac = if frac == 0 {
        String::new()
    } else {
        format!(".{frac:03}").trim_end_matches('0').to_owned()
    };
    format!("{}T{h:02}:{mi:02}:{s:02}{frac}{suffix}", date(days))
}

/// Formats an offset in minutes as `+05:30` (or `Z` for zero when `z`).
pub fn offset_text(min: i32, z: bool) -> String {
    if min == 0 && z {
        return "Z".into();
    }
    let a = min.abs();
    format!(
        "{}{:02}:{:02}",
        if min < 0 { '-' } else { '+' },
        a / 60,
        a % 60
    )
}

/// TAI − UTC from 1972 (IERS Bulletin C): (first day, as y, m, d; seconds).
const LEAP: &[(i64, u32, u32, i32)] = &[
    (1972, 1, 1, 10),
    (1972, 7, 1, 11),
    (1973, 1, 1, 12),
    (1974, 1, 1, 13),
    (1975, 1, 1, 14),
    (1976, 1, 1, 15),
    (1977, 1, 1, 16),
    (1978, 1, 1, 17),
    (1979, 1, 1, 18),
    (1980, 1, 1, 19),
    (1981, 7, 1, 20),
    (1982, 7, 1, 21),
    (1983, 7, 1, 22),
    (1985, 7, 1, 23),
    (1988, 1, 1, 24),
    (1990, 1, 1, 25),
    (1991, 1, 1, 26),
    (1992, 7, 1, 27),
    (1993, 7, 1, 28),
    (1994, 7, 1, 29),
    (1996, 1, 1, 30),
    (1997, 7, 1, 31),
    (1999, 1, 1, 32),
    (2006, 1, 1, 33),
    (2009, 1, 1, 34),
    (2012, 7, 1, 35),
    (2015, 7, 1, 36),
    (2017, 1, 1, 37),
];

/// The bulletin the table reflects, echoed in `meta.assets`.
pub const LEAP_TABLE: (&str, &str) = ("leap-seconds", "IERS Bulletin C 72 (2026-07-06)");

/// Bulletin C 72 rules out a leap second at the end of December 2026, so the
/// table is known through 2027-06-30; the next possible insertion follows.
pub fn leap_table_expires() -> i64 {
    days_from_civil(2027, 7, 1)
}

/// TAI − UTC on a UTC day, or `None` before 1972 (no integer-second UTC).
pub fn tai_minus_utc(days: i64) -> Option<i32> {
    LEAP.iter()
        .rev()
        .find(|(y, m, d, _)| days >= days_from_civil(*y, *m, *d))
        .map(|e| e.3)
}

/// True when a leap second was inserted at the end of this UTC day.
pub fn leap_second_after(days: i64) -> bool {
    match (tai_minus_utc(days), tai_minus_utc(days + 1)) {
        (Some(a), Some(b)) => b > a,
        _ => false,
    }
}

/// GPS epoch 1980-01-06 as a day number.
pub fn gps_epoch() -> i64 {
    days_from_civil(1980, 1, 6)
}

/// GPS − UTC is TAI − UTC − 19 s.
pub const TAI_MINUS_GPS: i32 = 19;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trip() {
        for z in -800_000..800_000 {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(day_of_year(days_from_civil(2024, 12, 31)), 366);
        assert_eq!(day_of_year(days_from_civil(2026, 9, 18)), 261);
    }

    #[test]
    fn parse_and_format() {
        let s = parse_stamp("2026-09-18T14:05:30.25-05:00").unwrap();
        assert_eq!(s.offset, Some(-300));
        assert_eq!(iso(s.days, s.secs, "Z"), "2026-09-18T14:05:30.25Z");
        assert_eq!(parse_stamp("2026-09-18").unwrap().secs, 0.0);
        assert!(parse_stamp("2026-02-29").is_err());
        assert!(parse_stamp("2026-09-18T24:00").is_err());
        assert!(parse_stamp("2026-09-18T23:59:60Z").is_err());
        let leap = parse_stamp("2016-12-31T23:59:60Z").unwrap();
        assert_eq!(iso(leap.days, leap.secs, "Z"), "2016-12-31T23:59:60Z");
        assert_eq!(iso(0, 86_399.999_6, "Z"), "1970-01-02T00:00:00Z");
        for (t, m) in [
            ("Z", 0),
            ("+05:30", 330),
            ("-0500", -300),
            ("UTC-7", -420),
            ("UTC+5:45", 345),
        ] {
            assert_eq!(parse_offset(t).unwrap(), m, "{t}");
        }
        assert!(parse_offset("+15:00").is_err());
        assert!(parse_offset("America/Denver").is_err());
    }

    #[test]
    fn leap_seconds() {
        assert_eq!(tai_minus_utc(days_from_civil(2026, 9, 18)), Some(37));
        assert_eq!(tai_minus_utc(days_from_civil(2016, 12, 31)), Some(36));
        assert_eq!(tai_minus_utc(days_from_civil(1971, 12, 31)), None);
        assert!(leap_second_after(days_from_civil(2016, 12, 31)));
        assert!(!leap_second_after(days_from_civil(2017, 1, 1)));
    }
}
