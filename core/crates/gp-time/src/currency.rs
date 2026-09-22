//! Night currency counter (add-practitioner-essentials, time/solar-and-
//! twilight "Night currency counter"): which logged night takeoffs and
//! full-stop landings fall inside the §61.57(b) period (1 hour after sunset
//! to 1 hour before sunrise, from the sun at each place), and, per aircraft
//! category and class, whether three of each lie within the preceding 90
//! days of a date and when that currency lapses.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::regulation::rule;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};

use crate::solar::{CFR_61_57, jd, local_noon};
use crate::sun::{self, SUNRISE_ALTITUDE};
use crate::{ZoneSpec, civil, text};

const ROW: &[Field] = &[
    text(
        "when",
        "Local date and time with UTC offset",
        "Like 2026-05-01T21:40-06:00",
    )
    .required(),
    Field::new(
        "lat",
        "Latitude",
        "Where it happened, like 39.86",
        Kind::Number {
            min: -90.0,
            max: 90.0,
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Like -104.67",
        Kind::Number {
            min: -180.0,
            max: 180.0,
        },
    )
    .required(),
    Field::new(
        "takeoffs",
        "Takeoffs",
        "Like 1",
        Kind::Number {
            min: 0.0,
            max: 50.0,
        },
    ),
    Field::new(
        "landings",
        "Full-stop landings",
        "Like 1",
        Kind::Number {
            min: 0.0,
            max: 50.0,
        },
    ),
    text(
        "aircraft",
        "Category and class",
        "Like ASEL, or a type if one is required",
    )
    .required(),
];

const OUT_GROUP: &[Field] = &[
    text("aircraft", "Category and class", "As entered"),
    Field::new(
        "takeoffs",
        "Qualifying takeoffs in 90 days",
        "Counted",
        Kind::Number {
            min: 0.0,
            max: 10_000.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "landings",
        "Qualifying landings in 90 days",
        "Counted",
        Kind::Number {
            min: 0.0,
            max: 10_000.0,
        },
    )
    .precision(Precision::Decimals(0)),
    text("current", "Current", "yes or no"),
    text("through", "Current through", "The last day, or none"),
];

const OUT_EVENT: &[Field] = &[
    text("when", "Logged", "As entered"),
    text("counts", "Counts for §61.57(b)", "yes or no"),
    text("period", "Currency period that night", "Local times"),
];

pub static NIGHT_CURRENCY: ToolDef = ToolDef {
    id: "time.sun.night-currency",
    title: "Night passenger currency",
    summary: "Which of your logged night takeoffs and full-stop landings count toward 14 CFR 61.57(b), whether you are current to carry passengers at night on a date, and the day that currency lapses, per category and class.",
    aliases: &[
        "night currency calculator",
        "night currency",
        "61.57(b) currency",
        "night landing currency",
        "passenger currency at night",
    ],
    keywords: &["night", "currency", "61.57", "landings", "takeoffs", "90 days", "logbook", "passengers"],
    inputs: &[
        text("as_of", "Date to check", "Like 2026-07-15").required().core(),
        Field::new(
            "events",
            "Night takeoffs and landings",
            "One per line: local time with offset, place, counts, and category and class, like 2026-05-01T21:40-06:00, 39.86, -104.67, 1, 1, ASEL",
            Kind::List { items: ROW, min: 1, max: 200 },
        )
        .required()
        .core(),
    ],
    outputs: &[
        text("through", "Current through", "The last day it holds, or none, for the first category and class"),
        text("current", "Current on that date", "yes or no"),
        Field::new(
            "state",
            "Currency state",
            "1 if current on that date, 2 if not",
            Kind::Number { min: 1.0, max: 2.0 },
        )
        .precision(Precision::Decimals(0)),
        text("rule", "Rule", "The regulation, its counting, and the date these rules were checked"),
        Field::new("aircraft", "By category and class", "Qualifying events and currency", Kind::List { items: OUT_GROUP, min: 1, max: 200 }),
        Field::new("events", "Each event", "Whether it counts, with that night's period", Kind::List { items: OUT_EVENT, min: 1, max: 200 }),
    ],
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "Each event counts when its time lies from 1 hour after that evening's sunset to 1 hour before the next sunrise at its place (the sun's centre at −0.8333°, NREL SPA); a time before noon belongs to the night that began the evening before. Per category and class, current through the 90th day after the older of the third most recent qualifying takeoff and the third most recent qualifying full-stop landing on or before the date",
    accuracy: "Sun times within about a minute of USNO; an event within a minute of the boundary is worth checking against the Air Almanac. Planning aid: your logbook and 61.57 govern, including simulator credit this does not count",
    references: &[CFR_61_57],
    examples: &[Example {
        id: "primary",
        title: "Three night landings near Denver, checked on 2026-07-15",
        input: r#"{"as_of":"2026-07-15","events":[{"when":"2026-05-01T22:30-06:00","lat":39.86,"lon":-104.67,"takeoffs":1,"landings":1,"aircraft":"ASEL"},{"when":"2026-05-10T22:30-06:00","lat":39.86,"lon":-104.67,"takeoffs":1,"landings":1,"aircraft":"ASEL"},{"when":"2026-06-02T23:00-06:00","lat":39.86,"lon":-104.67,"takeoffs":1,"landings":1,"aircraft":"ASEL"}]}"#,
        source: "add-practitioner-essentials currency scenario: landings on May 1, May 10, and June 2 give currency through July 30, 2026",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[Related { id: "time.sun.aviation-nights", reason: "parent" }],
    sentence: "{if state == 1}You are current to carry passengers at night through {through}.{/if}{if state == 2}You are not current to carry passengers at night on that date.{/if}",
    limits: &[("batchRows", 100)],
    run: run_currency,
    ..ToolDef::BLANK
};

fn run_currency(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let as_of_s = ctx.text("as_of")?.expect("required");
    let as_of = civil::parse_stamp(as_of_s.trim())
        .map(|s| s.days)
        .map_err(|m| ToolError::invalid("/as_of", m))?;
    let rows = ctx.rows("events")?;
    // (aircraft, local day, takeoffs, landings) for qualifying events.
    let mut good: Vec<(String, i64, u32, u32)> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut events = Vec::with_capacity(rows.len());
    let hour = 1.0 / 24.0;
    for (i, r) in rows.iter().enumerate() {
        let at = |f: &str| format!("/events/{i}/{f}");
        let when = ctx.row_text("events", i, r, "when")?.expect("required");
        let stamp =
            civil::parse_stamp(when.trim()).map_err(|m| ToolError::invalid(&at("when"), m))?;
        let Some(off) = stamp.offset else {
            return Err(ToolError::invalid(
                &at("when"),
                "Add the UTC offset to the time, like 2026-05-01T21:40-06:00, so the sun can be placed.",
            ));
        };
        let count = |f: &str| -> Result<u32, ToolError> {
            match r.get(f).and_then(|v| v.as_f64()) {
                None => Ok(0),
                Some(v) if v.fract() == 0.0 && (0.0..=50.0).contains(&v) => Ok(v as u32),
                Some(_) => Err(ToolError::invalid(
                    &at(f),
                    "Counts are whole numbers from 0 to 50.",
                )),
            }
        };
        let lat = r.get("lat").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        let lon = r.get("lon").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(ToolError::invalid(
                &at("lat"),
                "Give the place as a latitude and longitude in degrees.",
            ));
        }
        let (tk, ld) = (count("takeoffs")?, count("landings")?);
        let aircraft = ctx
            .row_text("events", i, r, "aircraft")?
            .expect("required")
            .trim()
            .to_ascii_uppercase();
        // The night this event belongs to: before local noon, the one that began the evening before.
        let local_day = stamp.days;
        let night = if stamp.secs < 43_200.0 {
            local_day - 1
        } else {
            local_day
        };
        let zone = ZoneSpec::Fixed(off);
        let set = sun::crossing(
            lat,
            lon,
            local_noon(lon, night, &zone),
            SUNRISE_ALTITUDE,
            1.0,
        );
        let rise = sun::crossing(
            lat,
            lon,
            local_noon(lon, night + 1, &zone),
            SUNRISE_ALTITUDE,
            -1.0,
        );
        let t = jd(local_day, stamp.secs / 60.0 - f64::from(off));
        let local = |j: f64| {
            let m = ((j - jd(0, 0.0)) * 1440.0).round() as i64 + i64::from(off);
            format!(
                "{:02}:{:02}",
                m.rem_euclid(1440) / 60,
                m.rem_euclid(1440) % 60
            )
        };
        let (counts, period) = match (set, rise) {
            (sun::Side::At(s), sun::Side::At(rr)) => {
                let (a, b) = (s + hour, rr - hour);
                (t >= a && t <= b, format!("{} to {}", local(a), local(b)))
            }
            (sun::Side::Below, _) | (_, sun::Side::Below) => {
                (true, "the sun does not rise".to_owned())
            }
            _ => (false, "the sun does not set".to_owned()),
        };
        if counts && (tk > 0 || ld > 0) {
            good.push((aircraft.clone(), local_day, tk, ld));
        }
        if !order.contains(&aircraft) {
            order.push(aircraft);
        }
        events.push(Json::obj([
            ("when", Json::str(when.trim())),
            ("counts", Json::str(if counts { "yes" } else { "no" })),
            ("period", Json::str(period)),
        ]));
    }
    let r = rule("faa-61-57-b");
    let window = r.value as i64;
    let mut groups = Vec::new();
    let mut first: Option<(bool, Option<i64>)> = None;
    for a in &order {
        let mine: Vec<&(String, i64, u32, u32)> =
            good.iter().filter(|e| &e.0 == a && e.1 <= as_of).collect();
        // Each takeoff and landing on its own day, most recent first.
        let mut tks: Vec<i64> = mine
            .iter()
            .flat_map(|e| std::iter::repeat_n(e.1, e.2 as usize))
            .collect();
        let mut lds: Vec<i64> = mine
            .iter()
            .flat_map(|e| std::iter::repeat_n(e.1, e.3 as usize))
            .collect();
        tks.sort_unstable_by(|x, y| y.cmp(x));
        lds.sort_unstable_by(|x, y| y.cmp(x));
        let through = match (tks.get(2), lds.get(2)) {
            (Some(t3), Some(l3)) => Some((*t3).min(*l3) + window),
            _ => None,
        };
        let current = through.is_some_and(|d| as_of <= d);
        let recent = |v: &[i64]| v.iter().filter(|&&d| as_of - d <= window).count();
        first.get_or_insert((current, through));
        groups.push(Json::obj([
            ("aircraft", Json::str(a)),
            ("takeoffs", Json::Num(recent(&tks) as f64)),
            ("landings", Json::Num(recent(&lds) as f64)),
            ("current", Json::str(if current { "yes" } else { "no" })),
            (
                "through",
                Json::str(through.map_or("none".to_owned(), civil::date)),
            ),
        ]));
    }
    let (current, through) = first.unwrap_or((false, None));
    if ctx.explaining() {
        ctx.step(
            "Qualifying events",
            "inside 1 hour after sunset to 1 hour before sunrise at each place",
            format!("{} logged", rows.len()),
            format!("{} qualify", good.len()),
        );
        ctx.step(
            "Current through",
            "older of the third most recent qualifying takeoff and landing + 90 days",
            format!("as of {}", civil::date(as_of)),
            through.map_or("none".to_owned(), civil::date),
        );
    }
    Ok(Json::obj([
        (
            "through",
            Json::str(through.map_or("none".to_owned(), civil::date)),
        ),
        ("current", Json::str(if current { "yes" } else { "no" })),
        ("state", Json::Num(if current { 1.0 } else { 2.0 })),
        (
            "rule",
            Json::str(format!(
                "{}: {} days, counted as described; rules as of {}. Not legal advice: {}",
                r.citation, window, r.reviewed, r.url
            )),
        ),
        ("aircraft", Json::Arr(groups)),
        ("events", Json::Arr(events)),
    ]))
}
