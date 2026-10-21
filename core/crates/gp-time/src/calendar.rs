//! Calendar forms software uses (add-celestial-and-moon, time/sidereal-and-
//! calendar): Unix time by the POSIX expression, and ISO 8601 week dates.
//! Both are integer day arithmetic on the proleptic Gregorian calendar, with
//! no clock and no time zone.

use crate::civil;
use crate::{gps_seconds, plain, text};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Reference, Related, ToolDef};

const POSIX: Reference = Reference {
    title: "POSIX.1-2024 (IEEE Std 1003.1-2024), Base Definitions",
    issuer: "IEEE and The Open Group",
    year: 2024,
    edition: "IEEE Std 1003.1-2024",
    locator: "Section 4.19, Seconds Since the Epoch: the expression from a UTC calendar time, with every day counted as 86,400 seconds",
    url: "https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_19",
};
const ISO_8601: Reference = Reference {
    title: "ISO 8601-1:2019, Date and time, Part 1: Basic rules",
    issuer: "International Organization for Standardization",
    year: 2019,
    edition: "ISO 8601-1:2019 with Amd 1:2022",
    locator: "Week dates: weeks begin on Monday, week 01 holds the year's first Thursday (so 4 January), and a year has 52 or 53 weeks",
    url: "https://www.iso.org/standard/70907.html",
};

const SECONDS_PER_DAY: f64 = 86_400.0;
const JD_UNIX: f64 = 2_440_587.5;
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

// ------------------------------------------------------------- Unix time

pub static UNIX_TIME: ToolDef = ToolDef {
    id: "time.scale.unix-time",
    title: "Unix time converter",
    summary: "Converts Unix time (seconds since 1970-01-01 UTC, or milliseconds, microseconds, or nanoseconds) to a UTC date and time and back, by the POSIX expression, with the Julian date and GPS week.",
    aliases: &["epoch converter", "Unix timestamp to date", "POSIX time"],
    keywords: &["Unix", "epoch", "timestamp", "POSIX", "milliseconds", "UTC"],
    inputs: &[
        Field::new(
            "unix",
            "Unix time",
            "Seconds since 1970-01-01T00:00:00Z, like 1760832000; milliseconds and finer are recognized by their digits",
            Kind::Text { max_len: 32 },
        )
        .core(),
        text("utc", "UTC time", "Instead of a Unix time: like 2026-10-19T00:00:00Z").core(),
        Field::new(
            "unit",
            "Unix time unit",
            "s, ms, us, or ns, to override the digit count, like ms",
            Kind::Choice(&["s", "ms", "us", "ns"]),
        ),
    ],
    outputs: &[
        text("utc", "UTC", "ISO 8601 UTC time"),
        plain("unix", "Unix seconds", "Seconds since the epoch, leap seconds not counted", 3),
        plain("unix_ms", "Unix milliseconds", "The same in milliseconds", 0),
        plain("jd", "Julian date", "2440587.5 + Unix seconds / 86,400", 6),
        plain("gps_week", "GPS week", "Weeks since 1980-01-06", 0).optional(),
        plain("gps_seconds", "Seconds of the GPS week", "GPS time, leap seconds counted", 3).optional(),
        text("note", "Leap seconds", "How the count treats them"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED"],
    model: "POSIX.1-2024 §4.19: seconds = (days since 1970-01-01) × 86,400 + seconds of the UTC day; every day is 86,400 s, so leap seconds are not counted; JD = 2440587.5 + seconds / 86,400",
    accuracy: "Exact to the millisecond for any date from year 0001 to 9999",
    when_to_use: "Use this to read a timestamp from a log, a database, or an API as a date and time, or to make one from a date. Paste the number as it is; the tool reads seconds, milliseconds, microseconds, or nanoseconds from its length and says which. It also gives the Julian date and the GPS week for the same instant.",
    limitations: "Unix time counts every day as 86,400 seconds, so it skips leap seconds: the second 23:59:60 has no Unix time, and the tool refuses it. POSIX leaves times before 1970 undefined; the tool extends the same 86,400-second day count backward, as most systems do, so they come out negative. Reading the unit from the digit count can be wrong for times near 1970 or far in the future; choose the unit to be sure.",
    references: &[POSIX],
    examples: &[Example {
        id: "primary",
        title: "1760832000000, a time in milliseconds",
        input: r#"{"unix":"1760832000000"}"#,
        source: "add-celestial-and-moon time spec scenario: read as milliseconds, 2025-10-19T00:00:00Z (the spec printed 2026; 1,760,832,000 s is 20,380 days, 2025-10-19)",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "time.scale.julian-date", reason: "alternative" },
        Related { id: "time.scale.gps-week", reason: "alternative" },
        Related { id: "time.scale.utc-offset", reason: "next" },
    ],
    sentence: "That is {utc}.",
    limits: &[("batchRows", 10_000)],
    run: run_unix_time,
    ..ToolDef::BLANK
};

/// Days and seconds of the day for Unix seconds.
fn split(unix: f64) -> (i64, f64) {
    let days = (unix / SECONDS_PER_DAY).floor();
    (days as i64, unix - days * SECONDS_PER_DAY)
}

fn run_unix_time(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let given = ["unix", "utc"].iter().filter(|n| ctx.is_set(n)).count();
    if given != 1 {
        return Err(ToolError::invalid(
            "/unix",
            "Give either a Unix time or a UTC time.",
        ));
    }
    let unix = if let Some(raw) = ctx.text("unix")? {
        let t = raw.trim().replace(['_', ','], "");
        let x: f64 = t.parse().map_err(|_| {
            ToolError::invalid("/unix", "A Unix time is a number, like 1760832000.")
        })?;
        if !x.is_finite() {
            return Err(ToolError::invalid(
                "/unix",
                "A Unix time is a finite number.",
            ));
        }
        let digits = t
            .trim_start_matches(['-', '+'])
            .split('.')
            .next()
            .unwrap_or("")
            .trim_start_matches('0')
            .len();
        let (scale, name) = match ctx.choice("unit")? {
            Some("ms") => (1e3, "milliseconds"),
            Some("us") => (1e6, "microseconds"),
            Some("ns") => (1e9, "nanoseconds"),
            Some(_) => (1.0, "seconds"),
            None => match digits {
                0..=11 => (1.0, "seconds"),
                12..=14 => (1e3, "milliseconds"),
                15..=17 => (1e6, "microseconds"),
                _ => (1e9, "nanoseconds"),
            },
        };
        if scale != 1.0 && !ctx.is_set("unit") {
            ctx.warnings.push(Warning::new(
                "INPUT_NORMALIZED",
                format!(
                    "Read as {name} from its {digits} digits; choose the unit if that is wrong."
                ),
            ));
        }
        x / scale
    } else {
        let s = civil::parse_stamp(&ctx.text("utc")?.expect("set"))
            .map_err(|m| ToolError::invalid("/utc", m))?;
        let (days, secs) = crate::to_utc(s);
        if secs >= SECONDS_PER_DAY {
            return Err(ToolError::invalid(
                "/utc",
                "That is a leap second, which Unix time cannot represent: POSIX counts every day as 86,400 seconds.",
            )
            .hint("Use TAI or GPS time for an instant inside a leap second."));
        }
        days as f64 * SECONDS_PER_DAY + secs
    };
    // Years 0001 to 9999.
    if !(-62_135_596_800.0..253_402_300_800.0).contains(&unix) {
        return Err(ToolError::invalid(
            "/unix",
            "Unix times from year 0001 to 9999 only.",
        ));
    }
    let (days, secs) = split(unix);
    let mut out = vec![
        ("utc", Json::str(civil::iso(days, secs, "Z"))),
        ("unix", Json::Num(unix)),
        ("unix_ms", Json::Num((unix * 1e3).round())),
        ("jd", Json::Num(JD_UNIX + unix / SECONDS_PER_DAY)),
    ];
    if days >= civil::gps_epoch() {
        let g = gps_seconds(days, secs);
        let week = (g / 604_800.0).floor();
        out.push(("gps_week", Json::Num(week)));
        out.push(("gps_seconds", Json::Num(g - week * 604_800.0)));
    }
    out.push((
        "note",
        Json::str("Unix time counts every day as 86,400 seconds; leap seconds are not counted."),
    ));
    Ok(Json::obj(out))
}

// -------------------------------------------------------------- ISO week

pub static ISO_WEEK: ToolDef = ToolDef {
    id: "time.scale.iso-week",
    title: "ISO week date",
    summary: "The ISO 8601 week date of a calendar date (week-year, week, and weekday, like 2026-W53-5), or the calendar date of a week date, with the week's Monday and Sunday.",
    aliases: &[
        "week number calculator",
        "ISO week number",
        "what week is it",
    ],
    keywords: &["ISO 8601", "week", "week number", "calendar", "weekday"],
    inputs: &[
        text("date", "Date", "Like 2027-01-01").core(),
        text(
            "week_date",
            "Week date",
            "Instead of a date: like 2026-W53-5 or 2026-W53",
        )
        .core(),
    ],
    outputs: &[
        text("week_date", "ISO week date", "YYYY-Www-D"),
        plain("week_year", "Week-year", "The year the week belongs to", 0),
        plain("week", "Week", "01 to 53", 0),
        text(
            "weekday",
            "Weekday",
            "Monday to Sunday; the week date's last digit is 1 to 7",
        ),
        text("date", "Calendar date", "YYYY-MM-DD"),
        text("monday", "Monday of the week", ""),
        text("sunday", "Sunday of the week", ""),
        plain("weeks_in_year", "Weeks in the week-year", "52 or 53", 0),
    ],
    errors: &[ErrorCode::InvalidInput],
    model: "ISO 8601-1:2019 week dates: weekday 1 (Monday) to 7 (Sunday); the week-year and week are those of the week's Thursday; week 01 is the week holding 4 January; a year has 53 weeks when it starts on a Thursday, or on a Wednesday in a leap year",
    accuracy: "Exact",
    when_to_use: "Use this to find the week number of a date for a schedule, a payroll period, a sprint plan, or a log file, or to turn a week number someone gave you back into dates. Enter a date to get its week, or a week like 2026-W53 to get its Monday and Sunday. Weeks run Monday to Sunday, so the first days of January can belong to the last week of the year before, and the last days of December to week 1 of the next.",
    limitations: "It uses only the ISO 8601 rule. US calendars that start weeks on Sunday, and systems that count the week holding 1 January as week 1, number some weeks differently, most often in the first and last week of a year. The week-year can differ from the calendar year for up to three days at either end of it. Dates run from year 0001 to 9999 on the proleptic Gregorian calendar, which does not match historical calendars before 1582.",
    references: &[ISO_8601],
    examples: &[Example {
        id: "primary",
        title: "1 January 2027",
        input: r#"{"date":"2027-01-01"}"#,
        source: "add-celestial-and-moon time spec scenario: 2027-01-01 is 2026-W53-5",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.scale.julian-date",
            reason: "alternative",
        },
        Related {
            id: "time.scale.unix-time",
            reason: "alternative",
        },
        Related {
            id: "time.scale.zone-info",
            reason: "alternative",
        },
    ],
    sentence: "{date} is {week_date}, a {weekday}.",
    limits: &[("batchRows", 10_000)],
    run: run_iso_week,
    ..ToolDef::BLANK
};

/// ISO weekday, 1 (Monday) to 7, of a day number (1970-01-01 was a Thursday).
fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7) + 1
}

/// Weeks in an ISO week-year: 53 when it starts on a Thursday, or on a
/// Wednesday in a leap year.
fn weeks_in(year: i64) -> i64 {
    let jan1 = weekday(civil::days_from_civil(year, 1, 1));
    if jan1 == 4 || (jan1 == 3 && civil::is_leap_year(year)) {
        53
    } else {
        52
    }
}

/// The day number of the Monday of week 1 of an ISO week-year.
fn week1_monday(year: i64) -> i64 {
    let jan4 = civil::days_from_civil(year, 1, 4);
    jan4 - (weekday(jan4) - 1)
}

fn run_iso_week(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let given = ["date", "week_date"]
        .iter()
        .filter(|n| ctx.is_set(n))
        .count();
    if given != 1 {
        return Err(ToolError::invalid(
            "/date",
            "Give either a date or a week date.",
        ));
    }
    let days = if let Some(d) = ctx.text("date")? {
        let s = civil::parse_stamp(&d).map_err(|m| ToolError::invalid("/date", m))?;
        s.days
    } else {
        let w = ctx.text("week_date")?.expect("set");
        let bad = || {
            ToolError::invalid(
                "/week_date",
                "A week date looks like 2026-W53-5 (or 2026-W53 for its Monday).",
            )
            .hint("Example: 2026-W01-4")
        };
        let t = w.trim().to_ascii_uppercase().replace('-', "");
        // YYYYWwwD or YYYYWww
        let (y, rest) = t.split_once('W').ok_or_else(bad)?;
        let year: i64 = y.parse().map_err(|_| bad())?;
        let (wk, d) = match rest.len() {
            2 => (rest, "1"),
            3 => (&rest[..2], &rest[2..]),
            _ => return Err(bad()),
        };
        let (week, day): (i64, i64) = (
            wk.parse().map_err(|_| bad())?,
            d.parse().map_err(|_| bad())?,
        );
        if !(1..=9999).contains(&year)
            || !(1..=7).contains(&day)
            || week < 1
            || week > weeks_in(year)
        {
            return Err(ToolError::invalid(
                "/week_date",
                format!(
                    "{year} has weeks 01 to {:02}, and weekdays run 1 (Monday) to 7 (Sunday).",
                    weeks_in(year.clamp(1, 9999))
                ),
            ));
        }
        week1_monday(year) + (week - 1) * 7 + (day - 1)
    };
    let wd = weekday(days);
    let thursday = days - wd + 4;
    let (wy, _, _) = civil::civil_from_days(thursday);
    let week = (thursday - civil::days_from_civil(wy, 1, 1)) / 7 + 1;
    let monday = days - wd + 1;
    Ok(Json::obj([
        ("week_date", Json::str(format!("{wy:04}-W{week:02}-{wd}"))),
        ("week_year", Json::Num(wy as f64)),
        ("week", Json::Num(week as f64)),
        ("weekday", Json::str(WEEKDAYS[(wd - 1) as usize])),
        ("date", Json::str(civil::date(days))),
        ("monday", Json::str(civil::date(monday))),
        ("sunday", Json::str(civil::date(monday + 6))),
        ("weeks_in_year", Json::Num(weeks_in(wy) as f64)),
    ]))
}
