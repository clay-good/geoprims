//! Time: the time scales and formats that aviation, GNSS, surveying, and
//! astronomy use (add-practitioner-essentials, time-scales spec), without the
//! host clock or time-zone database. UTC offsets are explicit; the
//! leap-second table is embedded and echoed in `meta.assets`.

pub mod civil;
pub mod solar;
pub mod spa;
mod spa_tables;
pub mod sun;
pub mod tz;

use civil::{Stamp, TAI_MINUS_GPS};
use gp_base::ErrorCode;
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};

const IS_GPS_200: Reference = Reference {
    title: "IS-GPS-200N, NAVSTAR GPS Space Segment/Navigation User Segment Interfaces",
    issuer: "U.S. Space Force, Space Systems Command",
    year: 2022,
    edition: "Revision N",
    locator: "Sections 3.3.4 (GPS time, epoch 1980 January 6) and 20.3.3.3.1.1 (10-bit transmission week number)",
    url: "https://www.gps.gov/technical/icwg/IS-GPS-200N.pdf",
};
const BULLETIN_C: Reference = Reference {
    title: "IERS Bulletin C 72",
    issuer: "International Earth Rotation and Reference Systems Service",
    year: 2026,
    edition: "Paris, 6 July 2026",
    locator: "UTC − TAI = −37 s from 2017 January 1 until further notice; no leap second at the end of December 2026",
    url: "https://hpiers.obspm.fr/iers/bul/bulc/bulletinc.dat",
};
const USNO_JD: Reference = Reference {
    title: "Julian Date Converter",
    issuer: "U.S. Naval Observatory, Astronomical Applications Department",
    year: 2024,
    edition: "Online data service",
    locator: "Julian date definition: JD 2451545.0 is 2000 January 1, 12:00 UT; MJD = JD − 2400000.5",
    url: "https://aa.usno.navy.mil/data/JulianDate",
};
const RFC_3339: Reference = Reference {
    title: "RFC 3339, Date and Time on the Internet: Timestamps",
    issuer: "Internet Engineering Task Force",
    year: 2002,
    edition: "Proposed standard",
    locator: "Section 5.6 (timestamp format with a numeric UTC offset or Z)",
    url: "https://www.rfc-editor.org/rfc/rfc3339",
};
const IANA_TZ: Reference = Reference {
    title: "Time Zone Database (tzdb)",
    issuer: "Internet Assigned Numbers Authority",
    year: 2026,
    edition: "Release 2026d, compiled with zic -b slim",
    locator: "Zone and Link entries for every region; TZif per RFC 8536",
    url: "https://www.iana.org/time-zones",
};
const CFR_1_1: Reference = Reference {
    title: "14 CFR 1.1, General definitions (flight time)",
    issuer: "Federal Aviation Administration",
    year: 2024,
    edition: "Current as of the review date",
    locator: "\"Flight time\": from moving under its own power for flight until coming to rest after landing",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-A/part-1/section-1.1",
};

const SECONDS_PER_WEEK: f64 = 604_800.0;

fn secs(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Time, "s").expect("registered unit"),
    }
}

const fn plain(name: &'static str, title: &'static str, help: &'static str, decimals: u8) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e12,
            max: 1e12,
        },
    )
    .precision(Precision::Plain(decimals))
}

pub(crate) const fn text(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 40 })
}

fn stamp(ctx: &Ctx, name: &str) -> Result<Stamp, ToolError> {
    let s = ctx.text(name)?.expect("required");
    civil::parse_stamp(&s).map_err(|m| ToolError::invalid(&format!("/{name}"), m))
}

/// A stamp in UTC: applies its offset (none means it is already UTC).
pub(crate) fn to_utc(s: Stamp) -> (i64, f64) {
    match s.offset {
        None | Some(0) => (s.days, s.secs),
        Some(off) => {
            let t = s.secs - f64::from(off) * 60.0;
            let shift = (t / 86_400.0).floor();
            (s.days + shift as i64, t - shift * 86_400.0)
        }
    }
}

fn leap_asset(ctx: &mut Ctx, days: i64) {
    ctx.assets.push(AssetRef {
        id: civil::LEAP_TABLE.0.into(),
        version: civil::LEAP_TABLE.1.into(),
    });
    if days >= civil::leap_table_expires() {
        ctx.warnings.push(Warning::new(
            "LEAP_SECOND_TABLE_EXPIRED",
            "This date is past the leap-second table (IERS Bulletin C 72 covers through 2027-06-30). A leap second added later would shift the result by 1 s.",
        ));
    }
}

/// A time zone input: a fixed UTC offset, or an IANA zone from the embedded
/// tzdb (echoed in `meta.assets`).
pub(crate) enum ZoneSpec {
    Fixed(i32),
    Named(tz::Zone),
}

impl ZoneSpec {
    pub(crate) fn parse(ctx: &mut Ctx, name: &str) -> Result<ZoneSpec, ToolError> {
        let s = ctx.text(name)?.expect("required");
        let at = format!("/{name}");
        let t = s.trim();
        let looks_named = t.contains('/')
            || (t.len() > 3 && t.bytes().all(|b| b.is_ascii_alphabetic() || b == b'_'));
        if looks_named {
            let z = tz::zone(t).ok_or_else(|| {
                ToolError::invalid(
                    &at,
                    format!(
                        "\"{t}\" is not an IANA time zone in tzdb {}.",
                        tz::version()
                    ),
                )
                .hint(
                    "Use a name like America/Denver or Europe/London, or a UTC offset like -06:00.",
                )
            })?;
            ctx.assets.push(AssetRef {
                id: "tzdb".into(),
                version: tz::version().into(),
            });
            return Ok(ZoneSpec::Named(z));
        }
        match civil::parse_offset(t) {
            Ok(m) => Ok(ZoneSpec::Fixed(m)),
            // Short and legacy tzdb names such as CET, EET, or EST5EDT.
            Err(m) => match tz::zone(t) {
                Some(z) => {
                    ctx.assets.push(AssetRef {
                        id: "tzdb".into(),
                        version: tz::version().into(),
                    });
                    Ok(ZoneSpec::Named(z))
                }
                None => Err(ToolError::invalid(&at, m)),
            },
        }
    }

    /// Offset in minutes at a UTC instant (Unix seconds).
    pub(crate) fn minutes_at(&self, unix: i64) -> i32 {
        match self {
            ZoneSpec::Fixed(m) => *m,
            ZoneSpec::Named(z) => z.at(unix).utoff / 60,
        }
    }

    pub(crate) fn abbr_at(&self, unix: i64) -> Option<&'static str> {
        match self {
            ZoneSpec::Fixed(_) => None,
            ZoneSpec::Named(z) => Some(z.at(unix).abbr),
        }
    }
}

/// GPS seconds since 1980-01-06 for a UTC day and seconds of day.
fn gps_seconds(days: i64, s: f64) -> f64 {
    let tai = civil::tai_minus_utc(days).expect("after 1980");
    (days - civil::gps_epoch()) as f64 * 86_400.0 + s + f64::from(tai - TAI_MINUS_GPS)
}

/// UTC day and seconds of day (86,400 or more inside a leap second) for GPS seconds.
fn utc_from_gps(g: f64) -> (i64, f64) {
    let d0 = civil::gps_epoch() + (g / 86_400.0).floor() as i64;
    for d in [d0 - 1, d0] {
        let s = g - gps_seconds(d, 0.0);
        let len = if civil::leap_second_after(d) {
            86_401.0
        } else {
            86_400.0
        };
        if (0.0..len).contains(&s) {
            return (d, s);
        }
    }
    (d0, g - gps_seconds(d0, 0.0))
}

// ---------------------------------------------------------------- GPS week

pub static GPS_WEEK: ToolDef = ToolDef {
    id: "time.scale.gps-week",
    title: "UTC to GPS week and seconds",
    summary: "The GPS week, seconds of week, 10-bit week and rollover era, and the GPS − UTC and TAI − UTC offsets for a UTC time.",
    aliases: &[
        "GPS week calculator",
        "GPS time converter",
        "UTC to GPS time",
    ],
    keywords: &[
        "GPS week",
        "seconds of week",
        "rollover",
        "leap seconds",
        "TAI",
        "GNSS time",
    ],
    inputs: &[text(
        "utc",
        "UTC time",
        "Like 2026-09-18T00:00:00Z (an offset like -05:00 is applied)",
    )
    .required()
    .core()],
    outputs: &[
        plain(
            "gps_week",
            "GPS week",
            "Full week number since 1980-01-06",
            0,
        ),
        plain(
            "seconds_of_week",
            "Seconds of week",
            "From Sunday 00:00 GPS time",
            3,
        ),
        plain(
            "week_10bit",
            "10-bit week",
            "As broadcast in the legacy navigation message (0 to 1023)",
            0,
        ),
        plain(
            "rollover_era",
            "Rollover era",
            "Number of 1,024-week rollovers",
            0,
        ),
        Field::new(
            "gps_minus_utc",
            "GPS − UTC",
            "Leap seconds since 1980",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "tai_minus_utc",
            "TAI − UTC",
            "From IERS Bulletin C",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(0)),
        text(
            "gps_time",
            "GPS time",
            "The same instant on the GPS time scale",
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["LEAP_SECOND_TABLE_EXPIRED", "EXPERIMENTAL_TOOL"],
    model: "GPS time = UTC + (TAI − UTC) − 19 s; week = ⌊GPS seconds since 1980-01-06 / 604,800⌋",
    accuracy: "Exact for dates the leap-second table covers",
    references: &[IS_GPS_200, BULLETIN_C],
    examples: &[Example {
        id: "primary",
        title: "2026-09-18 at 00:00 UTC",
        input: r#"{"utc":"2026-09-18T00:00:00Z"}"#,
        source: "add-practitioner-essentials scenario: week 2436, seconds of week 432018, GPS − UTC 18 s",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "time.scale.gps-to-utc",
        reason: "inverse",
    }],
    sentence: "It is GPS week {gps_week}, second {seconds_of_week}.",
    limits: &[("batchRows", 10_000)],
    run: run_gps_week,
    ..ToolDef::BLANK
};

fn run_gps_week(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (days, s) = to_utc(stamp(ctx, "utc")?);
    if days < civil::gps_epoch() {
        return Err(
            ToolError::new(ErrorCode::OutOfDomain, "GPS time starts on 1980-01-06.").at("/utc"),
        );
    }
    leap_asset(ctx, days);
    let tai = civil::tai_minus_utc(days).expect("after 1980");
    let g = gps_seconds(days, s);
    let week = (g / SECONDS_PER_WEEK).floor();
    let (gd, gs) = (
        civil::gps_epoch() + (g / 86_400.0).floor() as i64,
        g.rem_euclid(86_400.0),
    );
    Ok(Json::obj([
        ("gps_week", Json::Num(week)),
        ("seconds_of_week", Json::Num(g - week * SECONDS_PER_WEEK)),
        ("week_10bit", Json::Num(week % 1024.0)),
        ("rollover_era", Json::Num((week / 1024.0).floor())),
        (
            "gps_minus_utc",
            ctx.out("gps_minus_utc", secs(f64::from(tai - TAI_MINUS_GPS))),
        ),
        (
            "tai_minus_utc",
            ctx.out("tai_minus_utc", secs(f64::from(tai))),
        ),
        ("gps_time", Json::str(civil::iso(gd, gs, " GPST"))),
    ]))
}

pub static GPS_TO_UTC: ToolDef = ToolDef {
    id: "time.scale.gps-to-utc",
    title: "GPS week and seconds to UTC",
    summary: "The UTC time for a GPS week and seconds of week, including 10-bit weeks with their rollover era.",
    aliases: &["GPS week to date", "GPS time to UTC"],
    keywords: &[
        "GPS week",
        "seconds of week",
        "rollover",
        "10-bit week",
        "UTC",
    ],
    inputs: &[
        Field::new(
            "week",
            "GPS week",
            "Full week like 2436, or a 10-bit week (0 to 1023) with its era",
            Kind::Number {
                min: 0.0,
                max: 100_000.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "seconds_of_week",
            "Seconds of week",
            "0 to 604,799.999, like 432018",
            Kind::Number {
                min: 0.0,
                max: 604_799.999_999,
            },
        )
        .required()
        .core(),
        Field::new(
            "era",
            "Rollover era",
            "For a 10-bit week: 0 (1980–1999), 1 (1999–2019), 2 (2019–2038)",
            Kind::Number {
                min: 0.0,
                max: 60.0,
            },
        )
        .core(),
    ],
    outputs: &[
        text("utc", "UTC", "ISO 8601 UTC time"),
        plain("gps_week", "GPS week", "Full week number", 0),
        Field::new(
            "gps_minus_utc",
            "GPS − UTC",
            "Leap seconds since 1980",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    errors: &[],
    warnings: &["LEAP_SECOND_TABLE_EXPIRED", "EXPERIMENTAL_TOOL"],
    model: "UTC = GPS time − (TAI − UTC − 19 s); full week = 10-bit week + 1,024 × era",
    accuracy: "Exact for dates the leap-second table covers; instants inside a leap second print as 23:59:60",
    references: &[IS_GPS_200, BULLETIN_C],
    examples: &[Example {
        id: "primary",
        title: "Week 2436, second 432018",
        input: r#"{"week":2436,"seconds_of_week":432018}"#,
        source: "Inverse of the add-practitioner-essentials GPS week scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "time.scale.gps-week",
        reason: "inverse",
    }],
    sentence: "That is {utc}.",
    limits: &[("batchRows", 10_000)],
    run: run_gps_to_utc,
    ..ToolDef::BLANK
};

fn run_gps_to_utc(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let week = ctx.number("week")?.expect("required");
    let sow = ctx.number("seconds_of_week")?.expect("required");
    if week.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/week",
            "The GPS week is a whole number.",
        ));
    }
    let full = match ctx.number("era")? {
        Some(era) if era.fract() != 0.0 => {
            return Err(ToolError::invalid("/era", "The era is a whole number."));
        }
        Some(_) if week >= 1024.0 => {
            return Err(ToolError::invalid(
                "/era",
                "An era goes only with a 10-bit week (0 to 1023). This week is already a full week number.",
            ));
        }
        Some(era) => week + 1024.0 * era,
        None if week < 1024.0 => {
            return Err(ToolError::invalid(
                "/era",
                "A week below 1024 may be a 10-bit week, which repeats every 19.6 years. Give the era: 0 for 1980–1999, 1 for 1999–2019, 2 for 2019–2038.",
            ));
        }
        None => week,
    };
    let (d, s) = utc_from_gps(full * SECONDS_PER_WEEK + sow);
    leap_asset(ctx, d);
    let tai = civil::tai_minus_utc(d).expect("after 1980");
    Ok(Json::obj([
        ("utc", Json::str(civil::iso(d, s, "Z"))),
        ("gps_week", Json::Num(full)),
        (
            "gps_minus_utc",
            ctx.out("gps_minus_utc", secs(f64::from(tai - TAI_MINUS_GPS))),
        ),
    ]))
}

// ---------------------------------------------------------------- Julian date

pub static JULIAN_DATE: ToolDef = ToolDef {
    id: "time.scale.julian-date",
    title: "Julian date, MJD, and day of year",
    summary: "The Julian date, modified Julian date, and day of year for a UTC time (or the time for a JD or MJD), with the RINEX daily file name.",
    aliases: &[
        "Julian date calculator",
        "day of year calculator",
        "MJD converter",
        "GPS day of year",
    ],
    keywords: &[
        "Julian date",
        "JD",
        "MJD",
        "day of year",
        "DOY",
        "RINEX",
        "OPUS",
    ],
    inputs: &[
        text("utc", "UTC time", "Like 2026-09-18T00:00:00Z").core(),
        Field::new(
            "jd",
            "Julian date",
            "Instead of a time: like 2461301.5",
            Kind::Number {
                min: 1_721_425.5,
                max: 5_373_484.5,
            },
        )
        .core(),
        Field::new(
            "mjd",
            "Modified Julian date",
            "Instead of a time: like 61301",
            Kind::Number {
                min: -678_575.0,
                max: 2_973_484.0,
            },
        ),
        Field::new(
            "station",
            "Station ID",
            "Four characters naming the station in the daily file name, like PIT1 (default ssss)",
            Kind::Text { max_len: 4 },
        ),
    ],
    outputs: &[
        plain(
            "jd",
            "Julian date",
            "Days since noon, 4713 BC January 1 (Julian calendar)",
            5,
        ),
        plain("mjd", "Modified Julian date", "JD − 2400000.5", 5),
        plain("day_of_year", "Day of year", "1 to 366", 0),
        plain("year", "Year", "Calendar year", 0),
        text("utc", "UTC", "ISO 8601 UTC time"),
        text(
            "rinex_name",
            "RINEX daily file",
            "ssssdddf.yyo (RINEX 2 short name, session 0)",
        ),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "JD = 2440587.5 + days since 1970-01-01 + fraction of the UTC day; MJD = JD − 2400000.5",
    accuracy: "JD and MJD to about 1 ms; UTC-based (leap seconds are not counted, as is conventional)",
    references: &[USNO_JD],
    examples: &[Example {
        id: "primary",
        title: "2026-09-18 at 00:00 UTC",
        input: r#"{"utc":"2026-09-18T00:00:00Z"}"#,
        source: "add-practitioner-essentials scenario: JD 2461301.5, MJD 61301, day 261",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "time.scale.gps-week",
        reason: "next",
    }],
    sentence: "The Julian date is {jd}, and it is day {day_of_year} of {year}.",
    limits: &[("batchRows", 10_000)],
    run: run_julian,
    ..ToolDef::BLANK
};

const JD_UNIX: f64 = 2_440_587.5;
const MJD_OFFSET: f64 = 2_400_000.5;

fn run_julian(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let given = ["utc", "jd", "mjd"]
        .iter()
        .filter(|n| ctx.is_set(n))
        .count();
    if given != 1 {
        return Err(ToolError::invalid(
            "/utc",
            "Give exactly one of a UTC time, a Julian date, or a modified Julian date.",
        ));
    }
    let (days, s) = if ctx.is_set("utc") {
        to_utc(stamp(ctx, "utc")?)
    } else {
        let jd = match ctx.number("jd")? {
            Some(jd) => jd,
            None => ctx.number("mjd")?.expect("set") + MJD_OFFSET,
        };
        let x = jd - JD_UNIX;
        let d = x.floor();
        (d as i64, ((x - d) * 86_400.0 * 1000.0).round() / 1000.0)
    };
    let (days, s) = if s >= 86_400.0 && !civil::leap_second_after(days) {
        (days + 1, s - 86_400.0)
    } else {
        (days, s)
    };
    let jd = JD_UNIX + days as f64 + s.min(86_400.0) / 86_400.0;
    let station = ctx.text("station")?.unwrap_or_else(|| "ssss".into());
    if station.chars().count() != 4 || !station.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ToolError::invalid(
            "/station",
            "The station ID is four letters or digits, like PIT1.",
        ));
    }
    let (y, _, _) = civil::civil_from_days(days);
    let doy = civil::day_of_year(days);
    Ok(Json::obj([
        ("jd", Json::Num(jd)),
        ("mjd", Json::Num(jd - MJD_OFFSET)),
        ("day_of_year", Json::Num(f64::from(doy))),
        ("year", Json::Num(y as f64)),
        ("utc", Json::str(civil::iso(days, s, "Z"))),
        (
            "rinex_name",
            Json::str(format!(
                "{}{doy:03}0.{:02}o",
                station.to_ascii_lowercase(),
                y.rem_euclid(100)
            )),
        ),
    ]))
}

// ---------------------------------------------------------------- durations

/// Hours and minutes text: `1 h 18 min`, `45 min`, `3 h`.
fn duration_text(total_min: i64) -> String {
    let (h, m) = (total_min / 60, total_min % 60);
    match (h, m) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn duration_outputs(total_min: i64) -> Vec<(&'static str, Json)> {
    vec![
        ("duration", Json::str(duration_text(total_min))),
        (
            "hm",
            Json::str(format!("{}:{:02}", total_min / 60, total_min % 60)),
        ),
        ("hours", Json::Num(total_min as f64 / 60.0)),
        ("minutes", Json::Num(total_min as f64)),
    ]
}

const DURATION_OUT: [Field; 4] = [
    text("duration", "Duration", "Hours and minutes"),
    text("hm", "h:mm", "Logbook clock format"),
    Field::new(
        "hours",
        "Decimal hours",
        "Hours as a decimal",
        Kind::Number { min: 0.0, max: 1e9 },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "minutes",
        "Minutes",
        "Total minutes",
        Kind::Number {
            min: 0.0,
            max: 1e11,
        },
    )
    .precision(Precision::Decimals(0)),
];

pub static DECIMAL_HOURS: ToolDef = ToolDef {
    id: "time.scale.decimal-hours",
    title: "Decimal hours to hours and minutes",
    summary: "Converts decimal hours to hours and minutes (1.3 h is 1:18, not 1:30) and back.",
    aliases: &[
        "decimal hours converter",
        "tenths to minutes",
        "hobbs time converter",
    ],
    keywords: &[
        "decimal hours",
        "tenths",
        "h:mm",
        "logbook",
        "Hobbs",
        "tach",
    ],
    inputs: &[
        text("time", "Time", "Decimal hours like 1.3, or h:mm like 1:18")
            .required()
            .core(),
    ],
    outputs: &[
        DURATION_OUT[0],
        DURATION_OUT[1],
        DURATION_OUT[2],
        DURATION_OUT[3],
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "minutes = round(hours × 60); hours = h + mm / 60",
    accuracy: "Decimal hours round to the nearest minute",
    references: &[CFR_1_1],
    examples: &[Example {
        id: "primary",
        title: "1.3 hours",
        input: r#"{"time":"1.3"}"#,
        source: "add-practitioner-essentials scenario: 1.3 h is 1 h 18 min",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "time.scale.block-time",
        reason: "next",
    }],
    sentence: "That is {duration}, or {hours} hours.",
    limits: &[("batchRows", 10_000)],
    run: run_decimal_hours,
    ..ToolDef::BLANK
};

/// Parses `1:18` or `1.3` to whole minutes.
fn parse_duration(s: &str, at: &str) -> Result<i64, ToolError> {
    let t = s.trim();
    let bad = || {
        ToolError::invalid(
            at,
            format!("\"{s}\" is not decimal hours like 1.3 or h:mm like 1:18."),
        )
    };
    if let Some((h, m)) = t.split_once(':') {
        let ok = |x: &str| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit());
        if !ok(h) || m.len() != 2 || !ok(m) || h.len() > 6 {
            return Err(bad());
        }
        let (h, m): (i64, i64) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
        if m > 59 {
            return Err(ToolError::invalid(
                at,
                format!("\"{s}\" has {m} minutes; minutes run 00 to 59."),
            ));
        }
        return Ok(h * 60 + m);
    }
    let x: f64 = t.parse().map_err(|_| bad())?;
    if !x.is_finite() || !(0.0..=1e6).contains(&x) || t.starts_with(['+', '-']) {
        return Err(bad());
    }
    Ok((x * 60.0).round() as i64)
}

fn run_decimal_hours(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let t = ctx.text("time")?.expect("required");
    Ok(Json::obj(duration_outputs(parse_duration(&t, "/time")?)))
}

pub static BLOCK_TIME: ToolDef = ToolDef {
    id: "time.scale.block-time",
    title: "Block time (out to in)",
    summary: "The time between out and in (block time or flight time), across midnight and across UTC dates, in hours and minutes and decimal hours.",
    aliases: &[
        "block time calculator",
        "flight time calculator",
        "time between two times",
    ],
    keywords: &[
        "block time",
        "flight time",
        "out",
        "in",
        "OOOI",
        "logbook",
        "midnight",
    ],
    inputs: &[
        text(
            "out_time",
            "Out",
            "Like 2215, 22:15, 2215Z, or 2026-09-18T22:15Z",
        )
        .required()
        .core(),
        text(
            "in_time",
            "In",
            "Like 0140 (the next day is assumed when earlier)",
        )
        .required()
        .core(),
    ],
    outputs: &[
        DURATION_OUT[0],
        DURATION_OUT[1],
        DURATION_OUT[2],
        DURATION_OUT[3],
        Field::new(
            "note",
            "Note",
            "When the times cross midnight",
            Kind::Text { max_len: 120 },
        )
        .optional(),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "in − out, adding 24 h when a clock-only in time is earlier than out",
    accuracy: "Exact to the minute (to the second with full timestamps)",
    references: &[CFR_1_1],
    examples: &[Example {
        id: "primary",
        title: "Out 2215Z, in 0140Z",
        input: r#"{"out_time":"2215Z","in_time":"0140Z"}"#,
        source: "Across-midnight arithmetic: 3 h 25 min",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "time.scale.decimal-hours",
        reason: "alternative",
    }],
    sentence: "Block time is {duration} ({hours} hours).",
    limits: &[("batchRows", 10_000)],
    run: run_block_time,
    ..ToolDef::BLANK
};

/// Clock-only `HHMM`, `HH:MM`, optionally with `Z`, to minutes of day.
fn clock(s: &str) -> Option<i64> {
    let t = s.trim().trim_end_matches(['Z', 'z']).replace(':', "");
    if t.len() != 4 || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (h, m): (i64, i64) = (t[..2].parse().ok()?, t[2..].parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn run_block_time(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let out_s = ctx.text("out_time")?.expect("required");
    let in_s = ctx.text("in_time")?.expect("required");
    let mut note = None;
    let total_min = match (clock(&out_s), clock(&in_s)) {
        (Some(o), Some(i)) => {
            if i < o {
                note = Some("The in time is earlier than out, so it is taken as the next day.");
            }
            (i - o).rem_euclid(1440)
        }
        (None, None) => {
            let (od, os) = to_utc(stamp(ctx, "out_time")?);
            let (id, is) = to_utc(stamp(ctx, "in_time")?);
            let diff = (id - od) as f64 * 86_400.0 + (is - os);
            if diff < 0.0 {
                return Err(ToolError::invalid(
                    "/in_time",
                    "The in time is before the out time.",
                ));
            }
            if id != od {
                note = Some("The times cross midnight UTC.");
            }
            (diff / 60.0).round() as i64
        }
        _ => {
            return Err(ToolError::invalid(
                "/in_time",
                "Give both times as clock times (like 2215) or both as full dates and times.",
            ));
        }
    };
    let mut out = duration_outputs(total_min);
    if let Some(n) = note {
        out.push(("note", Json::str(n)));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- UTC offsets

pub static UTC_OFFSET: ToolDef = ToolDef {
    id: "time.scale.utc-offset",
    stability: gp_base::tool::Stability::Stable,
    title: "Local time to UTC (Zulu)",
    summary: "Converts a local time to UTC and Zulu time with an IANA time zone or a UTC offset, or UTC to local, handling daylight saving gaps and overlaps and showing the date change.",
    aliases: &["Zulu time converter", "local to UTC", "UTC to local time"],
    keywords: &["Zulu", "UTC", "local time", "time zone", "offset", "Z time"],
    inputs: &[
        text("time", "Time", "Like 2026-07-01T14:05")
            .required()
            .core(),
        text(
            "offset",
            "Time zone or UTC offset",
            "Like America/Chicago, or an offset like -05:00",
        )
        .required()
        .core(),
        Field::new(
            "direction",
            "Direction",
            "local-to-utc (default) or utc-to-local",
            Kind::Choice(&["local-to-utc", "utc-to-local"]),
        )
        .core(),
    ],
    outputs: &[
        text("zulu", "Zulu", "HHMMZ"),
        text("utc", "UTC", "ISO 8601 UTC time"),
        text("local", "Local", "ISO 8601 with the offset"),
        text("abbr", "Zone abbreviation", "Like CDT, for named zones").optional(),
        text(
            "date_change",
            "Date change",
            "When UTC and local dates differ",
        )
        .optional(),
        Field::new(
            "day_shift",
            "UTC day shift",
            "UTC date minus local date, in days (−1, 0, or 1)",
            Kind::Number {
                min: -1.0,
                max: 1.0,
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    errors: &[],
    warnings: &["AMBIGUOUS_INPUT", "EXPERIMENTAL_TOOL"],
    model: "UTC = local − offset; named zones from the embedded IANA tzdb (TZif with POSIX rules)",
    accuracy: "Exact for the tzdb release echoed in meta.assets",
    references: &[RFC_3339, IANA_TZ],
    examples: &[Example {
        id: "primary",
        title: "14:05 in Chicago on a summer date",
        input: r#"{"time":"2026-07-01T14:05","offset":"America/Chicago"}"#,
        source: "add-practitioner-essentials Zulu scenario: 1905Z",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.scale.block-time",
            reason: "next",
        },
        Related {
            id: "time.scale.zone-info",
            reason: "parent",
        },
        Related {
            id: "time.sun.events",
            reason: "next",
        },
    ],
    sentence: "That is {zulu}.{if day_shift != 0} The UTC date is {date_change}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_utc_offset,
    ..ToolDef::BLANK
};

fn run_utc_offset(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let zone = ZoneSpec::parse(ctx, "offset")?;
    let to_utc_dir = ctx.choice("direction")?.unwrap_or("local-to-utc") == "local-to-utc";
    let s = stamp(ctx, "time")?;
    let (ud, us, off) = match (&zone, to_utc_dir) {
        (ZoneSpec::Fixed(off), true) => {
            if let Some(given) = s.offset
                && given != *off
            {
                return Err(ToolError::invalid(
                    "/time",
                    format!(
                        "The time already carries offset {}; remove it or make it match.",
                        civil::offset_text(given, true)
                    ),
                ));
            }
            let (d, sec) = to_utc(Stamp {
                offset: Some(*off),
                ..s
            });
            (d, sec, *off)
        }
        (ZoneSpec::Named(z), true) => {
            if s.offset.is_some() {
                return Err(ToolError::invalid(
                    "/time",
                    "Remove the offset from the time; the zone supplies it.",
                ));
            }
            let whole = s.secs.floor();
            let local = s.days * 86_400 + whole as i64;
            let found = z.from_local(local);
            let Some(&first) = found.first() else {
                return Err(ToolError::invalid(
                    "/time",
                    format!(
                        "{} does not exist in {}: the clocks jump forward past it that day.",
                        civil::iso(s.days, s.secs, "").replace('T', " "),
                        z.name
                    ),
                ));
            };
            if found.len() > 1 {
                let z1 =
                    |t: i64| civil::iso(t.div_euclid(86_400), t.rem_euclid(86_400) as f64, "Z");
                ctx.warnings.push(Warning::new(
                    "AMBIGUOUS_INPUT",
                    format!("This local time happens twice as the clocks fall back: {} and {}. The first is used.", z1(found[0]), z1(found[1])),
                ));
            }
            let off = ((local - first) / 60) as i32;
            (
                first.div_euclid(86_400),
                first.rem_euclid(86_400) as f64 + (s.secs - whole),
                off,
            )
        }
        (_, false) => {
            if s.offset.is_some_and(|o| o != 0) {
                return Err(ToolError::invalid(
                    "/time",
                    "For utc-to-local, give the time in UTC (Z or no offset).",
                ));
            }
            (
                s.days,
                s.secs,
                zone.minutes_at(s.days * 86_400 + s.secs as i64),
            )
        }
    };
    let (ld, ls) = to_utc(Stamp {
        days: ud,
        secs: us,
        offset: Some(-off),
    });
    let unix = ud * 86_400 + us as i64;
    let mut out = vec![
        (
            "zulu",
            Json::str(format!(
                "{:02}{:02}Z",
                (us as i64) / 3600,
                (us as i64) / 60 % 60
            )),
        ),
        ("utc", Json::str(civil::iso(ud, us, "Z"))),
        (
            "local",
            Json::str(civil::iso(ld, ls, &civil::offset_text(off, false))),
        ),
    ];
    if let Some(a) = zone.abbr_at(unix) {
        out.push(("abbr", Json::str(a)));
    }
    if ud != ld {
        let which = if ud > ld {
            "the next day"
        } else {
            "the previous day"
        };
        out.push((
            "date_change",
            Json::str(format!("{} ({which})", civil::date(ud))),
        ));
    }
    out.push(("day_shift", Json::Num((ud - ld) as f64)));
    Ok(Json::obj(out))
}

pub static ZONE_INFO: ToolDef = ToolDef {
    id: "time.scale.zone-info",
    title: "Time zone rules at a date",
    summary: "The UTC offset, abbreviation, and daylight saving state of an IANA time zone at a moment, and when it next changes.",
    aliases: &[
        "what time zone offset",
        "when does daylight saving time change",
        "DST dates",
    ],
    keywords: &[
        "time zone",
        "IANA",
        "tzdb",
        "daylight saving",
        "DST",
        "offset",
        "abbreviation",
    ],
    inputs: &[
        text("zone", "Time zone", "IANA name, like America/Denver")
            .required()
            .core(),
        text("time", "At (UTC)", "Like 2026-09-18T00:00Z")
            .required()
            .core(),
    ],
    outputs: &[
        text("offset", "UTC offset", "Like -06:00"),
        text("abbr", "Abbreviation", "Like MDT"),
        text("dst", "Daylight saving", "yes or no"),
        text(
            "next_change",
            "Next change (UTC)",
            "When the offset or DST state next changes",
        )
        .optional(),
        text("next_offset", "Offset after the change", "Like -07:00").optional(),
        text("zone_name", "Zone", "Canonical spelling"),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Embedded IANA tzdb (TZif with POSIX rules past the last listed transition)",
    accuracy: "Exact for the tzdb release echoed in meta.assets; checked against Python zoneinfo at 238,800 instants",
    references: &[IANA_TZ],
    examples: &[Example {
        id: "primary",
        title: "Denver in September 2026",
        input: r#"{"zone":"America/Denver","time":"2026-09-18T00:00Z"}"#,
        source: "IANA tzdb 2026d: MDT (-06:00) until 2026-11-01 08:00 UTC",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "time.scale.utc-offset",
        reason: "next",
    }],
    sentence: "Clocks there read {abbr}, which is {offset} from UTC.",
    limits: &[("batchRows", 10_000)],
    run: run_zone_info,
    ..ToolDef::BLANK
};

fn run_zone_info(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let name = ctx.text("zone")?.expect("required");
    let z = tz::zone(name.trim()).ok_or_else(|| {
        ToolError::invalid(
            "/zone",
            format!(
                "\"{}\" is not an IANA time zone in tzdb {}.",
                name.trim(),
                tz::version()
            ),
        )
        .hint("Use a name like America/Denver, Europe/London, or Asia/Kolkata.")
    })?;
    ctx.assets.push(AssetRef {
        id: "tzdb".into(),
        version: tz::version().into(),
    });
    let (d, s) = to_utc(stamp(ctx, "time")?);
    let t = d * 86_400 + s as i64;
    let now = z.at(t);
    let off = |secs: i32| civil::offset_text(secs / 60, false);
    let mut out = vec![
        ("offset", Json::str(off(now.utoff))),
        ("abbr", Json::str(now.abbr)),
        ("dst", Json::str(if now.dst { "yes" } else { "no" })),
    ];
    if let Some(next) = z.next_change(t) {
        out.push((
            "next_change",
            Json::str(civil::iso(
                next.div_euclid(86_400),
                next.rem_euclid(86_400) as f64,
                "Z",
            )),
        ));
        out.push(("next_offset", Json::str(off(z.at(next).utoff))));
    }
    out.push(("zone_name", Json::str(z.name)));
    Ok(Json::obj(out))
}

pub static TOOLS: &[&ToolDef] = &[
    &GPS_WEEK,
    &GPS_TO_UTC,
    &JULIAN_DATE,
    &DECIMAL_HOURS,
    &BLOCK_TIME,
    &UTC_OFFSET,
    &ZONE_INFO,
    &solar::POSITION,
    &solar::EVENTS,
    &solar::AVIATION_NIGHTS,
    &solar::MAPPING_WINDOW,
];

pub static REGISTRY: Registry = Registry {
    module: "time",
    tools: TOOLS,
};

gp_base::export_module!("time", REGISTRY);
