//! Weather decoding (aviation/weather-decoding spec): METAR/SPECI and FB winds
//! and temperatures aloft, from text the user pastes. Nothing is fetched and
//! the tools never read a clock: an observation is flagged old only against
//! a current time the user supplies. Winds are true-referenced.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

pub const JO_7900_5: Reference = Reference {
    title: "Surface Weather Observing, FAA Order JO 7900.5E",
    issuer: "Federal Aviation Administration",
    year: 2022,
    edition: "JO 7900.5E Change 1",
    locator: "Chapter 14 (METAR/SPECI coding) and Chapter 15 (remarks: AO1/AO2, SLP, T group, PK WND, WSHFT, PRESRR/PRESFR, $)",
    url: "https://www.faa.gov/documentLibrary/media/Order/JO_7900.5E_CHANGE_1.pdf",
};

const SAFETY: &str =
    "Decoded from the text you pasted. Get a current official briefing before flight.";

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: unit(qt, s),
    }
}

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    qt: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q: qt, unit: u })
}

const fn text(name: &'static str, title: &'static str, help: &'static str, n: usize) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: n })
}

// ---------------------------------------------------------------- time

/// Day, hour, minute of a `DDHHMMZ` group.
fn ddhhmm(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.strip_suffix('Z')?;
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (d, h, m) = (
        s[0..2].parse().ok()?,
        s[2..4].parse().ok()?,
        s[4..6].parse().ok()?,
    );
    ((1..=31).contains(&d) && h < 24 && m < 60).then_some((d, h, m))
}

/// Reads the user's current time: `DDHHMMZ` or an ISO `YYYY-MM-DDTHH:MMZ`.
/// Returns (day, minutes of day, days in the previous month when known).
fn current_time(s: &str) -> Option<(u32, u32, Option<u32>)> {
    let s = s.trim();
    if let Some((d, h, m)) = ddhhmm(s) {
        return Some((d, h * 60 + m, None));
    }
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let p: Vec<u32> = date
        .split('-')
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    let t: Vec<u32> = time
        .split(':')
        .take(2)
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    if p.len() != 3 || t.len() != 2 || !(1..=12).contains(&p[1]) || t[0] > 23 || t[1] > 59 {
        return None;
    }
    let (y, mo) = (p[0], p[1]);
    let prev = if mo == 1 { 12 } else { mo - 1 };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let dim = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    ((1..=dim[mo as usize - 1]).contains(&p[2])).then_some((
        p[2],
        t[0] * 60 + t[1],
        Some(dim[prev as usize - 1]),
    ))
}

/// Minutes from the report time to the current time, handling month wrap.
fn age_minutes(report: (u32, u32, u32), now: (u32, u32, Option<u32>)) -> i64 {
    let (rd, rh, rm) = report;
    let (nd, nmin, prev_len) = now;
    let mut days = nd as i64 - rd as i64;
    if days < -15 {
        days += prev_len.unwrap_or(31) as i64;
    }
    days * 1440 + nmin as i64 - (rh * 60 + rm) as i64
}

fn old_check(ctx: &mut Ctx, t: (u32, u32, u32)) -> Result<(), ToolError> {
    if let Some(now) = ctx.text("current_time")? {
        let now = current_time(&now).ok_or_else(|| {
            ToolError::invalid(
                "/current_time",
                "Use a time like 182053Z or 2026-09-18T20:53Z.",
            )
        })?;
        let age = age_minutes(t, now);
        if age < -10 {
            ctx.warnings.push(
                Warning::new(
                    "SUSPECT_VALUE",
                    "The report time is after the current time you gave. Check both.",
                )
                .at("/current_time"),
            );
        } else if age > 120 {
            ctx.warnings.push(
                Warning::new(
                    "OBSERVATION_OLD",
                    format!(
                        "This report is {} h {} min old: more than 2 hours. Get a current report.",
                        age / 60,
                        age % 60
                    ),
                )
                .at("/current_time"),
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- METAR parts

/// A decoded wind group: direction (None for VRB), speed, gust (kt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindGroup {
    pub dir: Option<f64>,
    pub speed: f64,
    pub gust: Option<f64>,
}

/// `dddff(f)(Gff(f))KT`, `VRBffKT`, `00000KT`; also MPS and KMH units.
pub fn parse_wind(s: &str) -> Option<WindGroup> {
    let (body, factor) = if let Some(b) = s.strip_suffix("KT") {
        (b, 1.0)
    } else if let Some(b) = s.strip_suffix("MPS") {
        (b, 3600.0 / 1852.0)
    } else {
        let b = s.strip_suffix("KMH")?;
        (b, 1000.0 / 1852.0)
    };
    if body.len() < 5 {
        return None;
    }
    let (d, rest) = body.split_at(3);
    let dir = if d == "VRB" {
        None
    } else if d.bytes().all(|b| b.is_ascii_digit()) {
        let v: f64 = d.parse().ok()?;
        if v > 360.0 {
            return None;
        }
        Some(v)
    } else {
        return None;
    };
    let (sp, gust) = match rest.split_once('G') {
        Some((a, g)) => (a, Some(g)),
        None => (rest, None),
    };
    let num = |x: &str| {
        let x = x.strip_prefix('P').unwrap_or(x);
        ((2..=3).contains(&x.len()) && x.bytes().all(|b| b.is_ascii_digit()))
            .then(|| x.parse::<f64>().ok())
            .flatten()
    };
    Some(WindGroup {
        dir,
        speed: num(sp)? * factor,
        gust: match gust {
            Some(g) => Some(num(g)? * factor),
            None => None,
        },
    })
}

/// `n/dSM`, `wSM`, `M1/4SM`, `P6SM`; returns (statute miles, qualifier).
fn parse_sm(s: &str) -> Option<(f64, &'static str)> {
    let b = s.strip_suffix("SM")?;
    let (b, qual) = if let Some(x) = b.strip_prefix('M') {
        (x, "less than ")
    } else if let Some(x) = b.strip_prefix('P') {
        (x, "more than ")
    } else {
        (b, "")
    };
    let v = match b.split_once('/') {
        Some((n, d)) => n.parse::<f64>().ok()? / d.parse::<f64>().ok().filter(|d| *d > 0.0)?,
        None => b.parse::<f64>().ok()?,
    };
    Some((v, qual))
}

fn is_whole(s: &str) -> bool {
    (1..=2).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

const INTENSITY: &[(&str, &str)] = &[("-", "light"), ("+", "heavy"), ("VC", "in the vicinity:")];
const DESCRIPTORS: &[(&str, &str)] = &[
    ("MI", "shallow"),
    ("PR", "partial"),
    ("BC", "patches of"),
    ("DR", "low drifting"),
    ("BL", "blowing"),
    ("SH", "showers of"),
    ("TS", "thunderstorm"),
    ("FZ", "freezing"),
];
const PHENOMENA: &[(&str, &str)] = &[
    ("DZ", "drizzle"),
    ("RA", "rain"),
    ("SN", "snow"),
    ("SG", "snow grains"),
    ("IC", "ice crystals"),
    ("PL", "ice pellets"),
    ("GR", "hail"),
    ("GS", "small hail"),
    ("UP", "unknown precipitation"),
    ("BR", "mist"),
    ("FG", "fog"),
    ("FU", "smoke"),
    ("VA", "volcanic ash"),
    ("DU", "dust"),
    ("SA", "sand"),
    ("HZ", "haze"),
    ("PY", "spray"),
    ("PO", "dust whirls"),
    ("SQ", "squalls"),
    ("FC", "funnel cloud"),
    ("SS", "sandstorm"),
    ("DS", "duststorm"),
];

/// A present-weather group in plain words, like `-TSRA` → "light thunderstorm rain".
pub fn parse_weather(s: &str) -> Option<String> {
    let mut rest = s;
    let mut words: Vec<&str> = Vec::new();
    if rest == "+FC" {
        return Some("tornado or waterspout".into());
    }
    for (code, w) in INTENSITY {
        if let Some(r) = rest.strip_prefix(code) {
            words.push(w);
            rest = r;
            break;
        }
    }
    let mut any = false;
    if let Some((_, w)) = DESCRIPTORS.iter().find(|(c, _)| rest.starts_with(c)) {
        words.push(w);
        rest = &rest[2..];
        any = true;
    }
    while !rest.is_empty() {
        let (_, w) = PHENOMENA.iter().find(|(c, _)| rest.starts_with(c))?;
        words.push(w);
        rest = &rest[2..];
        any = true;
    }
    (any && !(words.len() == 1 && INTENSITY.iter().any(|(_, w)| *w == words[0])))
        .then(|| words.join(" "))
}

/// A cloud layer: cover, base (ft AGL), and convective type.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudLayer {
    pub cover: &'static str,
    pub code: &'static str,
    pub base_ft: Option<f64>,
    pub kind: Option<&'static str>,
}

fn parse_cloud(s: &str) -> Option<CloudLayer> {
    for (code, cover) in [
        ("SKC", "sky clear"),
        ("CLR", "clear below 12,000 ft"),
        ("NSC", "no significant cloud"),
        ("NCD", "no cloud detected"),
    ] {
        if s == code {
            return Some(CloudLayer {
                cover,
                code,
                base_ft: None,
                kind: None,
            });
        }
    }
    for (code, cover) in [
        ("FEW", "few"),
        ("SCT", "scattered"),
        ("BKN", "broken"),
        ("OVC", "overcast"),
        ("VV", "vertical visibility"),
    ] {
        if let Some(r) = s.strip_prefix(code) {
            let (h, kind) = if let Some(h) = r.strip_suffix("TCU") {
                (h, Some("towering cumulus"))
            } else if let Some(h) = r.strip_suffix("CB") {
                (h, Some("cumulonimbus"))
            } else {
                (r, None)
            };
            if h.len() != 3 {
                return None;
            }
            let base = if h == "///" {
                None
            } else if h.bytes().all(|b| b.is_ascii_digit()) {
                Some(h.parse::<f64>().ok()? * 100.0)
            } else {
                return None;
            };
            return Some(CloudLayer {
                cover,
                code,
                base_ft: base,
                kind,
            });
        }
    }
    None
}

/// `TT/DD` with `M` for minus; the dew point may be missing.
fn parse_temps(s: &str) -> Option<(f64, Option<f64>)> {
    let (a, b) = s.split_once('/')?;
    let t = |x: &str| {
        let (neg, x) = match x.strip_prefix('M') {
            Some(r) => (true, r),
            None => (false, x),
        };
        (x.len() == 2 && x.bytes().all(|b| b.is_ascii_digit())).then(|| {
            let v: f64 = x.parse().unwrap_or(0.0);
            if neg { -v } else { v }
        })
    };
    Some((t(a)?, if b.is_empty() { None } else { Some(t(b)?) }))
}

/// `RVR`: `R28L/2400FT`, `R06/1200V1800FT`, with `P`/`M` and trends `U`/`D`/`N`.
fn parse_rvr(s: &str) -> Option<String> {
    let body = s.strip_prefix('R')?;
    let (rwy, val) = body.split_once('/')?;
    if rwy.is_empty() || !rwy.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let (val, trend) = match val.chars().last() {
        Some('U') => (&val[..val.len() - 1], " rising"),
        Some('D') => (&val[..val.len() - 1], " falling"),
        Some('N') => (&val[..val.len() - 1], " steady"),
        _ => (val, ""),
    };
    let (val, unit) = match val.strip_suffix("FT") {
        Some(v) => (v, "ft"),
        None => (val, "m"),
    };
    let one = |x: &str| -> Option<String> {
        let (p, x) = if let Some(r) = x.strip_prefix('P') {
            ("more than ", r)
        } else if let Some(r) = x.strip_prefix('M') {
            ("less than ", r)
        } else {
            ("", x)
        };
        (x.len() == 4 && x.bytes().all(|b| b.is_ascii_digit()))
            .then(|| format!("{p}{} {unit}", x.trim_start_matches('0')))
    };
    let range = match val.split_once('V') {
        Some((a, b)) => format!("{} to {}", one(a)?, one(b)?),
        None => one(val)?,
    };
    Some(format!("runway {rwy} visual range {range}{trend}"))
}

/// Sea-level pressure from `SLPppp`: the value nearest 1000 hPa.
pub fn slp(ppp: &str) -> Option<f64> {
    if ppp.len() != 3 || !ppp.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: f64 = ppp.parse::<f64>().ok()? / 10.0;
    Some(if v < 50.0 { 1000.0 + v } else { 900.0 + v })
}

/// `Tsnnnsnnn`: precise temperature and dew point (sign digit 1 = negative).
fn t_group(s: &str) -> Option<(f64, Option<f64>)> {
    let b = s.strip_prefix('T')?;
    if !(b.len() == 4 || b.len() == 8) || !b.bytes().all(|x| x.is_ascii_digit()) {
        return None;
    }
    let one = |x: &str| -> Option<f64> {
        let v = x[1..].parse::<f64>().ok()? / 10.0;
        match &x[..1] {
            "0" => Some(v),
            "1" => Some(-v),
            _ => None,
        }
    };
    Some((
        one(&b[..4])?,
        if b.len() == 8 {
            Some(one(&b[4..])?)
        } else {
            None
        },
    ))
}

/// FAA flight category from ceiling (ft) and visibility (SM).
pub fn flight_category(ceiling: Option<f64>, vis: Option<f64>) -> &'static str {
    let c = ceiling.unwrap_or(f64::INFINITY);
    let v = vis.unwrap_or(f64::INFINITY);
    if c < 500.0 || v < 1.0 {
        "LIFR"
    } else if c < 1000.0 || v < 3.0 {
        "IFR"
    } else if c <= 3000.0 || v <= 5.0 {
        "MVFR"
    } else {
        "VFR"
    }
}

fn note(g: &str, m: String, groups: &mut Vec<Json>) {
    groups.push(Json::obj([
        ("group", Json::str(g)),
        ("meaning", Json::str(m)),
    ]));
}

// ---------------------------------------------------------------- METAR tool

const CLOUD_ROW: &[Field] = &[
    text(
        "cover",
        "Cover",
        "few, scattered, broken, overcast, or vertical visibility",
        40,
    ),
    qty("base", "Base", "Above ground level", QT::Length, "ft")
        .precision(Precision::Decimals(0))
        .optional(),
    text("type", "Type", "cumulonimbus or towering cumulus", 20).optional(),
];

const GROUP_ROW: &[Field] = &[
    text("group", "Group", "The coded group", 40),
    text("meaning", "Meaning", "In plain words", 200),
];

const UNDECODED_ROW: &[Field] = &[
    text("group", "Group", "The group as pasted", 40),
    Field::new(
        "position",
        "Position",
        "Its place in the report, counting from 1",
        Kind::Number {
            min: 1.0,
            max: 1000.0,
        },
    )
    .precision(Precision::Decimals(0)),
];

pub static METAR: ToolDef = ToolDef {
    id: "aviation.weather.metar-decode",
    title: "METAR decoder",
    summary: "Turns a pasted METAR or SPECI into plain language: wind (true), visibility, weather, clouds and ceiling, temperature, altimeter, US remarks, and the flight category. Groups it cannot read are listed, never dropped.",
    aliases: &[
        "METAR decoder",
        "decode METAR",
        "METAR translator",
        "SPECI decoder",
    ],
    keywords: &[
        "METAR",
        "SPECI",
        "weather report",
        "ceiling",
        "visibility",
        "altimeter",
        "flight category",
        "VFR",
        "IFR",
        "remarks",
        "SLP",
    ],
    inputs: &[
        text(
            "report",
            "Report",
            "Paste a METAR, like KDEN 181753Z 30015G25KT 10SM FEW080 30/08 A2980",
            2000,
        )
        .required()
        .core(),
        text(
            "current_time",
            "Current time (UTC)",
            "Optional, like 182053Z, to flag an old report",
            24,
        )
        .core(),
    ],
    outputs: &[
        text("station", "Station", "ICAO identifier", 8),
        text("report_type", "Type", "METAR or SPECI", 8),
        text("time", "Observed", "Day and time, UTC", 24),
        text(
            "modifier",
            "Modifier",
            "AUTO (automated) or COR (corrected)",
            12,
        )
        .optional(),
        qty(
            "wind_direction",
            "Wind direction",
            "True north reference",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(0))
        .optional(),
        qty("wind_speed", "Wind speed", "Sustained", QT::Speed, "kt")
            .precision(Precision::Decimals(0))
            .optional(),
        qty("wind_gust", "Gust", "Peak gust", QT::Speed, "kt")
            .precision(Precision::Decimals(0))
            .optional(),
        text("wind", "Wind", "In plain words, true-referenced", 120).optional(),
        qty("visibility", "Visibility", "Prevailing", QT::Distance, "mi")
            .precision(Precision::Significant(3))
            .optional(),
        text("visibility_text", "Visibility", "In plain words", 60).optional(),
        text("weather", "Weather", "Present weather in plain words", 300).optional(),
        Field::new(
            "clouds",
            "Clouds",
            "Each layer",
            Kind::List {
                items: CLOUD_ROW,
                min: 0,
                max: 12,
            },
        )
        .optional(),
        qty(
            "ceiling",
            "Ceiling",
            "Lowest broken, overcast, or vertical visibility",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0))
        .optional(),
        text(
            "flight_category",
            "Flight category",
            "VFR, MVFR, IFR, or LIFR",
            8,
        )
        .optional(),
        qty(
            "temperature",
            "Temperature",
            "Precise T group when present",
            QT::Temperature,
            "degC",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "dew_point",
            "Dew point",
            "Precise T group when present",
            QT::Temperature,
            "degC",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "altimeter",
            "Altimeter",
            "A or Q group",
            QT::Pressure,
            "inHg",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty(
            "sea_level_pressure",
            "Sea-level pressure",
            "From the SLP remark",
            QT::Pressure,
            "hPa",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "groups",
            "Decoded groups",
            "Every group and its meaning",
            Kind::List {
                items: GROUP_ROW,
                min: 0,
                max: 200,
            },
        ),
        Field::new(
            "not_decoded",
            "Not decoded",
            "Groups this decoder does not read",
            Kind::List {
                items: UNDECODED_ROW,
                min: 0,
                max: 200,
            },
        ),
        text("notice", "Notice", "Safety framing", 120),
    ],
    warnings: &["OBSERVATION_OLD", "SUSPECT_VALUE", "EXPERIMENTAL_TOOL"],
    model: "WMO FM 15/16 METAR and SPECI as coded in the US under FAA Order JO 7900.5E, with its remarks; flight categories from the FAA Aviation Weather Handbook",
    accuracy: "Decodes the text as written. It does not check the report against the station or fetch anything.",
    references: &[JO_7900_5, WEATHER_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "Denver on a gusty afternoon",
        input: r#"{"report":"KDEN 181753Z 30015G25KT 10SM FEW080 SCT200 30/08 A2980 RMK AO2 SLP052 T03000083"}"#,
        source: "add-practitioner-essentials weather scenario: 300° true at 15 gusting 25, 10 SM, FEW080 SCT200, 30.0/8.3 °C, 29.80 inHg, SLP 1005.2 hPa",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.runway-components",
            reason: "next",
        },
        Related {
            id: "aviation.altimetry.density-altitude",
            reason: "next",
        },
        Related {
            id: "aviation.weather.fb-winds-decode",
            reason: "alternative",
        },
    ],
    sentence: "{station} is {flight_category} with wind {wind} and visibility {visibility_text}.{if ceiling > 0} The ceiling is {ceiling}.{/if}{warn OBSERVATION_OLD} This report is old.{/warn} Get a current official briefing before flight.",
    limits: &[("batchRows", 1_000)],
    run: run_metar,
    ..ToolDef::BLANK
};

fn run_metar(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let raw = ctx.text("report")?.unwrap_or_default().to_ascii_uppercase();
    let raw = raw.trim_end_matches('=').to_owned();
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(
            ToolError::invalid("/report", "Paste a METAR or SPECI report.")
                .hint("Example: KDEN 181753Z 30015G25KT 10SM FEW080 30/08 A2980"),
        );
    }
    let fmt = ctx.options.format;
    let mut i = 0;
    let mut groups: Vec<Json> = Vec::new();
    let mut undecoded: Vec<Json> = Vec::new();
    let mut o: Vec<(&str, Json)> = Vec::new();
    let mut report_type = "METAR";
    if matches!(tokens[0], "METAR" | "SPECI") {
        report_type = if tokens[0] == "SPECI" {
            "SPECI"
        } else {
            "METAR"
        };
        i += 1;
    }
    let station = tokens.get(i).copied().filter(|s| {
        s.len() == 4
            && s.bytes().all(|b| b.is_ascii_alphanumeric())
            && s.as_bytes()[0].is_ascii_alphabetic()
    });
    let Some(station) = station else {
        return Err(ToolError::invalid(
            "/report",
            "The report must start with a four-letter station identifier, like KDEN.",
        ));
    };
    note(station, format!("{report_type} for {station}"), &mut groups);
    i += 1;
    let t = tokens.get(i).and_then(|s| ddhhmm(s)).ok_or_else(|| {
        ToolError::invalid(
            "/report",
            "The station must be followed by the observation time, like 181753Z.",
        )
    })?;
    let time = format!("day {} at {:02}{:02}Z", t.0, t.1, t.2);
    note(tokens[i], format!("observed {time}"), &mut groups);
    i += 1;
    o.push(("station", Json::str(station)));
    o.push(("report_type", Json::str(report_type)));
    o.push(("time", Json::str(&time)));
    if let Some(m) = tokens.get(i).filter(|s| matches!(**s, "AUTO" | "COR")) {
        note(
            m,
            if *m == "AUTO" {
                "automated report".into()
            } else {
                "corrected report".into()
            },
            &mut groups,
        );
        o.push(("modifier", Json::str(*m)));
        i += 1;
    }
    let mut vis_sm: Option<f64> = None;
    let mut weather: Vec<String> = Vec::new();
    let mut clouds: Vec<Json> = Vec::new();
    let mut ceiling: Option<f64> = None;
    let mut temps: Option<(f64, Option<f64>)> = None;
    let mut remarks = false;
    let mut cavok = false;
    while i < tokens.len() {
        let g = tokens[i];
        let pos = i + 1;
        if g == "RMK" {
            remarks = true;
            note(g, "remarks follow".into(), &mut groups);
            i += 1;
            continue;
        }
        if !remarks {
            if let Some(w) = parse_wind(g) {
                let mut text = match w.dir {
                    None if w.speed == 0.0 => "calm".to_owned(),
                    None => format!(
                        "variable at {} kt",
                        display::number(w.speed, Precision::Decimals(0), fmt)
                    ),
                    Some(_) if w.speed == 0.0 => "calm".to_owned(),
                    Some(d) => format!(
                        "{:03}° true at {} kt",
                        d as i64,
                        display::number(w.speed, Precision::Decimals(0), fmt)
                    ),
                };
                if let Some(gu) = w.gust {
                    text.push_str(&format!(
                        ", gusting {} kt",
                        display::number(gu, Precision::Decimals(0), fmt)
                    ));
                }
                if let Some(d) = w.dir {
                    o.push((
                        "wind_direction",
                        ctx.out("wind_direction", q(d, QT::Angle, "deg")),
                    ));
                }
                o.push((
                    "wind_speed",
                    ctx.out("wind_speed", q(w.speed, QT::Speed, "kt")),
                ));
                if let Some(gu) = w.gust {
                    o.push(("wind_gust", ctx.out("wind_gust", q(gu, QT::Speed, "kt"))));
                }
                // A variable-direction range follows as its own group.
                if let Some(v) = tokens.get(i + 1).filter(|s| {
                    s.len() == 7
                        && s.as_bytes()[3] == b'V'
                        && s[..3]
                            .bytes()
                            .chain(s[4..].bytes())
                            .all(|b| b.is_ascii_digit())
                }) {
                    text.push_str(&format!(", varying from {}° to {}°", &v[..3], &v[4..]));
                    note(
                        v,
                        format!(
                            "wind direction varies from {}° to {}° true",
                            &v[..3],
                            &v[4..]
                        ),
                        &mut groups,
                    );
                    note(g, format!("wind {text}"), &mut groups);
                    let n = groups.len();
                    groups.swap(n - 1, n - 2);
                    o.push(("wind", Json::str(&text)));
                    i += 2;
                    continue;
                }
                note(g, format!("wind {text}"), &mut groups);
                o.push(("wind", Json::str(&text)));
                i += 1;
                continue;
            }
            if g == "CAVOK" {
                cavok = true;
                vis_sm = Some(10.0 * 1000.0 / 1609.344);
                note(g, "ceiling and visibility OK: visibility 10 km or more, no cloud below 5,000 ft or the highest minimum sector altitude, no significant weather".into(), &mut groups);
                o.push(("visibility_text", Json::str("10 km or more")));
                i += 1;
                continue;
            }
            // Visibility: a whole number followed by a fraction ("1 1/2SM"), or one group.
            let two = tokens
                .get(i + 1)
                .filter(|n| is_whole(g) && n.contains('/'))
                .and_then(|n| parse_sm(n));
            if let Some((frac, qual)) = two {
                let v = g.parse::<f64>().unwrap_or(0.0) + frac;
                let txt = format!("{qual}{} {}", g, &tokens[i + 1][..tokens[i + 1].len() - 2]);
                vis_sm = Some(v);
                note(
                    &format!("{g} {}", tokens[i + 1]),
                    format!("visibility {txt} statute miles"),
                    &mut groups,
                );
                o.push(("visibility_text", Json::str(format!("{txt} SM"))));
                i += 2;
                continue;
            }
            if let Some((v, qual)) = parse_sm(g) {
                vis_sm = Some(v);
                let shown = &g[if qual.is_empty() { 0 } else { 1 }..g.len() - 2];
                note(
                    g,
                    format!("visibility {qual}{shown} statute miles"),
                    &mut groups,
                );
                o.push(("visibility_text", Json::str(format!("{qual}{shown} SM"))));
                i += 1;
                continue;
            }
            if g.len() == 4 && g.bytes().all(|b| b.is_ascii_digit()) && vis_sm.is_none() {
                let m: f64 = g.parse().unwrap_or(0.0);
                let txt = if g == "9999" {
                    "10 km or more".to_owned()
                } else {
                    format!("{} m", display::number(m, Precision::Decimals(0), fmt))
                };
                vis_sm = Some(if g == "9999" { 10_000.0 } else { m } / 1609.344);
                note(g, format!("visibility {txt}"), &mut groups);
                o.push(("visibility_text", Json::str(txt)));
                i += 1;
                continue;
            }
            if let Some(r) = parse_rvr(g) {
                note(g, r, &mut groups);
                i += 1;
                continue;
            }
            if let Some(w) = parse_weather(g) {
                note(g, w.clone(), &mut groups);
                weather.push(w);
                i += 1;
                continue;
            }
            if let Some(c) = parse_cloud(g) {
                let mut m = c.cover.to_owned();
                if let Some(b) = c.base_ft {
                    m.push_str(&format!(
                        " at {} ft",
                        display::number(b, Precision::Decimals(0), fmt)
                    ));
                }
                if let Some(k) = c.kind {
                    m.push_str(&format!(", {k}"));
                }
                note(g, m, &mut groups);
                if matches!(c.code, "BKN" | "OVC" | "VV") && ceiling.is_none() {
                    ceiling = c.base_ft;
                }
                let mut row = vec![("cover", Json::str(c.cover))];
                if let Some(b) = c.base_ft {
                    row.push(("base", ctx.out("ceiling", q(b, QT::Length, "ft"))));
                }
                if let Some(k) = c.kind {
                    row.push(("type", Json::str(k)));
                }
                clouds.push(Json::obj(row));
                i += 1;
                continue;
            }
            if let Some(tp) = parse_temps(g) {
                temps = Some(tp);
                note(
                    g,
                    match tp.1 {
                        Some(d) => format!("temperature {} °C, dew point {} °C", tp.0, d),
                        None => format!("temperature {} °C, dew point missing", tp.0),
                    },
                    &mut groups,
                );
                i += 1;
                continue;
            }
            if let Some(v) = g
                .strip_prefix('A')
                .filter(|v| v.len() == 4 && v.bytes().all(|b| b.is_ascii_digit()))
            {
                let x = v.parse::<f64>().unwrap_or(0.0) / 100.0;
                o.push((
                    "altimeter",
                    ctx.out("altimeter", q(x, QT::Pressure, "inHg")),
                ));
                note(
                    g,
                    format!(
                        "altimeter {} inHg",
                        display::number(x, Precision::Decimals(2), fmt)
                    ),
                    &mut groups,
                );
                i += 1;
                continue;
            }
            if let Some(v) = g
                .strip_prefix('Q')
                .filter(|v| v.len() == 4 && v.bytes().all(|b| b.is_ascii_digit()))
            {
                let x = v.parse::<f64>().unwrap_or(0.0);
                o.push((
                    "altimeter",
                    ctx.emit(
                        "altimeter",
                        q(x, QT::Pressure, "hPa"),
                        unit(QT::Pressure, "hPa"),
                    ),
                ));
                note(
                    g,
                    format!(
                        "altimeter (QNH) {} hPa",
                        display::number(x, Precision::Decimals(0), fmt)
                    ),
                    &mut groups,
                );
                i += 1;
                continue;
            }
            if matches!(g, "NOSIG" | "BECMG" | "TEMPO") {
                note(
                    g,
                    match g {
                        "NOSIG" => "no significant change expected".into(),
                        _ => "trend forecast follows".into(),
                    },
                    &mut groups,
                );
                i += 1;
                continue;
            }
        } else {
            // Remarks.
            if matches!(g, "AO1" | "AO2") {
                note(
                    g,
                    if g == "AO1" {
                        "automated station without a precipitation discriminator".into()
                    } else {
                        "automated station with a precipitation discriminator (rain or snow)".into()
                    },
                    &mut groups,
                );
                i += 1;
                continue;
            }
            if let Some(v) = g.strip_prefix("SLP") {
                if v == "NO" {
                    note(g, "sea-level pressure not available".into(), &mut groups);
                    i += 1;
                    continue;
                }
                if let Some(p) = slp(v) {
                    o.push((
                        "sea_level_pressure",
                        ctx.out("sea_level_pressure", q(p, QT::Pressure, "hPa")),
                    ));
                    note(
                        g,
                        format!(
                            "sea-level pressure {} hPa",
                            display::number(p, Precision::Decimals(1), fmt)
                        ),
                        &mut groups,
                    );
                    i += 1;
                    continue;
                }
            }
            if let Some(tp) = t_group(g) {
                temps = Some(tp);
                note(
                    g,
                    match tp.1 {
                        Some(d) => format!(
                            "precise temperature {} °C, dew point {} °C",
                            display::number(tp.0, Precision::Decimals(1), fmt),
                            display::number(d, Precision::Decimals(1), fmt)
                        ),
                        None => format!(
                            "precise temperature {} °C",
                            display::number(tp.0, Precision::Decimals(1), fmt)
                        ),
                    },
                    &mut groups,
                );
                i += 1;
                continue;
            }
            if g == "PK"
                && tokens.get(i + 1) == Some(&"WND")
                && let Some(v) = tokens.get(i + 2).and_then(|v| v.split_once('/'))
            {
                let (w, tm) = v;
                if (5..=6).contains(&w.len())
                    && w.bytes().all(|b| b.is_ascii_digit())
                    && (tm.len() == 2 || tm.len() == 4)
                    && tm.bytes().all(|b| b.is_ascii_digit())
                {
                    let when = if tm.len() == 2 {
                        format!("at {tm} minutes past the hour")
                    } else {
                        format!("at {tm}Z")
                    };
                    note(
                        &format!("PK WND {}", tokens[i + 2]),
                        format!(
                            "peak wind {}° true at {} kt {when}",
                            &w[..3],
                            w[3..].trim_start_matches('0')
                        ),
                        &mut groups,
                    );
                    i += 3;
                    continue;
                }
            }
            if g == "WSHFT"
                && let Some(tm) = tokens.get(i + 1).filter(|t| {
                    (t.len() == 2 || t.len() == 4) && t.bytes().all(|b| b.is_ascii_digit())
                })
            {
                let fropa = tokens.get(i + 2) == Some(&"FROPA");
                let when = if tm.len() == 2 {
                    format!("at {tm} minutes past the hour")
                } else {
                    format!("at {tm}Z")
                };
                note(
                    &format!("WSHFT {tm}{}", if fropa { " FROPA" } else { "" }),
                    format!(
                        "wind shift {when}{}",
                        if fropa {
                            ", with a frontal passage"
                        } else {
                            ""
                        }
                    ),
                    &mut groups,
                );
                i += if fropa { 3 } else { 2 };
                continue;
            }
            if matches!(g, "PRESRR" | "PRESFR") {
                note(
                    g,
                    if g == "PRESRR" {
                        "pressure rising rapidly".into()
                    } else {
                        "pressure falling rapidly".into()
                    },
                    &mut groups,
                );
                i += 1;
                continue;
            }
            if g == "$" {
                note(g, "the station needs maintenance".into(), &mut groups);
                i += 1;
                continue;
            }
        }
        undecoded.push(Json::obj([
            ("group", Json::str(g)),
            ("position", Json::Num(pos as f64)),
        ]));
        i += 1;
    }
    if let Some(v) = vis_sm {
        o.push((
            "visibility",
            ctx.out("visibility", q(v, QT::Distance, "mi")),
        ));
    }
    if !weather.is_empty() {
        o.push(("weather", Json::str(weather.join("; "))));
    }
    if !clouds.is_empty() {
        o.push(("clouds", Json::Arr(clouds)));
    }
    if let Some(c) = ceiling {
        o.push(("ceiling", ctx.out("ceiling", q(c, QT::Length, "ft"))));
    }
    if vis_sm.is_some() || ceiling.is_some() || cavok {
        o.push((
            "flight_category",
            Json::str(flight_category(ceiling, vis_sm)),
        ));
    }
    if let Some((t, d)) = temps {
        o.push((
            "temperature",
            ctx.out("temperature", q(t, QT::Temperature, "degC")),
        ));
        if let Some(d) = d {
            o.push((
                "dew_point",
                ctx.out("dew_point", q(d, QT::Temperature, "degC")),
            ));
        }
    }
    o.push(("groups", Json::Arr(groups)));
    o.push(("not_decoded", Json::Arr(undecoded)));
    o.push(("notice", Json::str(SAFETY)));
    old_check(ctx, t)?;
    Ok(obj(o))
}

// ---------------------------------------------------------------- FB winds

/// Standard FB levels (ft) for the contiguous US product.
pub const FB_LEVELS: &[f64] = &[
    3000.0, 6000.0, 9000.0, 12000.0, 18000.0, 24000.0, 30000.0, 34000.0, 39000.0,
];

/// A decoded FB group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FbGroup {
    /// None for light and variable.
    pub dir: Option<f64>,
    pub speed: f64,
    pub temp: Option<f64>,
}

/// Decodes one FB group at `level_ft`: `ddss`, `ddss±tt`, or `ddsstt` above 24,000 ft.
pub fn decode_fb(g: &str, level_ft: f64) -> Result<FbGroup, String> {
    let bad = || format!("{g} is not an FB winds group (like 2714+05, 731960, or 9900).");
    if g.len() < 4 || !g[..4].bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let dd: u32 = g[..2].parse().map_err(|_| bad())?;
    let ss: f64 = g[2..4].parse().map_err(|_| bad())?;
    let rest = &g[4..];
    let temp = match rest.len() {
        0 => None,
        3 if rest.starts_with(['+', '-']) && rest[1..].bytes().all(|b| b.is_ascii_digit()) => Some(
            rest[1..].parse::<f64>().map_err(|_| bad())?
                * if rest.starts_with('-') { -1.0 } else { 1.0 },
        ),
        // Above 24,000 ft temperatures are always negative and the sign is omitted.
        2 if rest.bytes().all(|b| b.is_ascii_digit()) && level_ft > 24_000.0 => {
            Some(-rest.parse::<f64>().map_err(|_| bad())?)
        }
        _ => return Err(bad()),
    };
    if dd == 99 && ss == 0.0 {
        return Ok(FbGroup {
            dir: None,
            speed: 0.0,
            temp,
        });
    }
    let (dir, speed) = match dd {
        1..=36 => (dd as f64 * 10.0, ss),
        51..=86 => ((dd - 50) as f64 * 10.0, ss + 100.0),
        _ => {
            return Err(format!(
                "{g}: direction code {dd:02} is not 01-36 or 51-86."
            ));
        }
    };
    Ok(FbGroup {
        dir: Some(dir),
        speed,
        temp,
    })
}

const FB_ROW: &[Field] = &[
    qty(
        "level",
        "Level",
        "Altitude (MSL) or flight level in feet",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0)),
    qty(
        "direction",
        "Direction",
        "True north reference",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(0))
    .optional(),
    qty("speed", "Speed", "Wind speed", QT::Speed, "kt").precision(Precision::Decimals(0)),
    qty(
        "temperature",
        "Temperature",
        "Air temperature",
        QT::Temperature,
        "degC",
    )
    .precision(Precision::Decimals(0))
    .optional(),
    text("text", "Decoded", "In plain words", 120),
];

pub static FB_WINDS: ToolDef = ToolDef {
    id: "aviation.weather.fb-winds-decode",
    title: "Winds aloft (FB) decoder",
    summary: "Decodes FB winds and temperatures aloft: one group at a level, or a whole station line, with light-and-variable winds, speeds over 100 kt, and the implied minus sign above 24,000 ft.",
    aliases: &[
        "winds aloft decoder",
        "FB winds decoder",
        "FD winds decoder",
        "winds and temperatures aloft",
    ],
    keywords: &[
        "winds aloft",
        "FB",
        "FD",
        "temperatures aloft",
        "9900",
        "light and variable",
        "flight level",
    ],
    inputs: &[
        text(
            "report",
            "Station line or group",
            "Like DEN 2714 2725+00 2635-08 or one group like 731960",
            400,
        )
        .required()
        .core(),
        qty(
            "level",
            "Level",
            "For a single group: its altitude, like 34000 ft",
            QT::Length,
            "ft",
        )
        .core(),
        text(
            "levels",
            "Levels",
            "Optional header levels, like 3000 6000 9000 12000 18000 24000 30000 34000 39000",
            120,
        ),
    ],
    outputs: &[
        text("station", "Station", "Identifier on the line", 8).optional(),
        Field::new(
            "winds",
            "Winds aloft",
            "One row per level",
            Kind::List {
                items: FB_ROW,
                min: 0,
                max: 20,
            },
        ),
        text("notice", "Notice", "Safety framing", 120),
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "FB winds and temperatures aloft coding: direction in tens of degrees true, 51-86 means add 100 kt, 9900 is light and variable, and signs are omitted above 24,000 ft (all negative)",
    accuracy: "Decodes the text as written. Levels missing at the left of a station line are the ones within 1,500 ft of the station (winds) or at 3,000 ft (temperature).",
    references: &[WEATHER_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "The group 731960 at FL340",
        input: r#"{"report":"731960","level":"34000 ft"}"#,
        source: "add-practitioner-essentials FB scenario: 230° true at 119 kt, -60 °C",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "next",
        },
        Related {
            id: "aviation.weather.metar-decode",
            reason: "alternative",
        },
    ],
    sentence: "Decoded the winds aloft. Winds are true. Get a current official briefing before flight.",
    limits: &[("batchRows", 1_000)],
    run: run_fb,
    ..ToolDef::BLANK
};

fn run_fb(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let raw = ctx.text("report")?.unwrap_or_default().to_ascii_uppercase();
    let mut toks: Vec<&str> = raw.split_whitespace().collect();
    let fmt = ctx.options.format;
    let mut o: Vec<(&str, Json)> = Vec::new();
    let station = toks
        .first()
        .filter(|t| t.bytes().all(|b| b.is_ascii_alphabetic()) && (3..=4).contains(&t.len()))
        .copied();
    if station.is_some() {
        toks.remove(0);
    }
    if toks.is_empty() {
        return Err(ToolError::invalid(
            "/report",
            "Paste an FB station line or one group.",
        ));
    }
    let levels: Vec<f64> = match (ctx.text("levels")?, ctx.quantity("level")?) {
        (Some(l), _) => l
            .split_whitespace()
            .filter(|t| *t != "FT")
            .map(|t| {
                t.parse::<f64>().map_err(|_| {
                    ToolError::invalid("/levels", format!("{t} is not a level in feet."))
                })
            })
            .collect::<Result<_, _>>()?,
        (None, Some(lv)) if toks.len() == 1 => vec![lv.to(unit(QT::Length, "ft"))],
        (None, _) => FB_LEVELS.to_vec(),
    };
    if toks.len() > levels.len() {
        return Err(ToolError::invalid(
            "/report",
            format!(
                "The line has {} groups but only {} levels.",
                toks.len(),
                levels.len()
            ),
        )
        .hint("Give the header levels, like 3000 6000 9000 12000 18000 24000 30000 34000 39000."));
    }
    // Missing groups are the lowest levels, so the groups line up from the right.
    let lv = &levels[levels.len() - toks.len()..];
    let mut rows = Vec::new();
    for (k, (g, level)) in toks.iter().zip(lv).enumerate() {
        let d = decode_fb(g, *level).map_err(|m| {
            ToolError::invalid("/report", m).hint(format!("Group {} of the line.", k + 1))
        })?;
        let wind = match d.dir {
            None => "light and variable (less than 5 kt)".to_owned(),
            Some(dir) => format!(
                "{:03}° true at {} kt",
                dir as i64,
                display::number(d.speed, Precision::Decimals(0), fmt)
            ),
        };
        let txt = match d.temp {
            Some(t) => format!(
                "{wind}, {} °C",
                display::number(t, Precision::Decimals(0), fmt)
            ),
            None => wind,
        };
        let mut row = vec![(
            "level",
            ctx.emit("level", q(*level, QT::Length, "ft"), unit(QT::Length, "ft")),
        )];
        if let Some(dir) = d.dir {
            row.push((
                "direction",
                ctx.emit(
                    "direction",
                    q(dir, QT::Angle, "deg"),
                    unit(QT::Angle, "deg"),
                ),
            ));
        }
        row.push((
            "speed",
            ctx.emit("speed", q(d.speed, QT::Speed, "kt"), unit(QT::Speed, "kt")),
        ));
        if let Some(t) = d.temp {
            row.push((
                "temperature",
                ctx.emit(
                    "temperature",
                    q(t, QT::Temperature, "degC"),
                    unit(QT::Temperature, "degC"),
                ),
            ));
        }
        row.push(("text", Json::str(txt)));
        rows.push(Json::obj(row));
    }
    if let Some(s) = station {
        o.push(("station", Json::str(s)));
    }
    o.push(("winds", Json::Arr(rows)));
    o.push(("notice", Json::str(SAFETY)));
    Ok(obj(o))
}

// ---------------------------------------------------------------- TAF

/// A TAF time `DDHH` (or `DDHHMM` for FM), in hours after the validity start.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TafTime {
    day: u32,
    hour: u32,
    minute: u32,
}

fn taf_time(s: &str) -> Option<TafTime> {
    if !(s.len() == 4 || s.len() == 6) || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let day: u32 = s[..2].parse().ok()?;
    let hour: u32 = s[2..4].parse().ok()?;
    let minute: u32 = if s.len() == 6 {
        s[4..].parse().ok()?
    } else {
        0
    };
    ((1..=31).contains(&day) && hour <= 24 && minute < 60).then_some(TafTime { day, hour, minute })
}

fn taf_range(s: &str) -> Option<(TafTime, TafTime)> {
    let (a, b) = s.split_once('/')?;
    Some((taf_time(a)?, taf_time(b)?))
}

impl TafTime {
    fn text(self) -> String {
        format!("day {} at {:02}{:02}Z", self.day, self.hour, self.minute)
    }

    /// Hours from `start`, with a day number below the start's meaning the next month.
    fn hours_from(self, start: TafTime, month_len: u32) -> i64 {
        let day = if self.day < start.day {
            self.day + month_len
        } else {
            self.day
        };
        (day as i64 - start.day as i64) * 24 + self.hour as i64 - start.hour as i64
    }

    /// Local time of day for a UTC offset in minutes, and the day relative to UTC.
    fn local(self, offset_min: i32) -> String {
        let mins = (self.hour * 60 + self.minute) as i32 + offset_min;
        let day = mins.div_euclid(1440);
        let m = mins.rem_euclid(1440);
        format!(
            "{:02}:{:02} local{}",
            m / 60,
            m % 60,
            match day {
                0 => "",
                d if d > 0 => ", next day",
                _ => ", previous day",
            }
        )
    }
}

fn parse_offset(s: &str) -> Option<i32> {
    let s = s.trim().trim_start_matches("UTC").trim_start_matches("utc");
    let (sign, rest) = match s.chars().next()? {
        '+' => (1, &s[1..]),
        '-' | '−' => (-1, &s[s.char_indices().nth(1)?.0..]),
        _ => return None,
    };
    let (h, m) = match rest.split_once(':') {
        Some((h, m)) => (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?),
        None => (rest.parse::<i32>().ok()?, 0),
    };
    (h <= 14 && m < 60).then_some(sign * (h * 60 + m))
}

/// Conditions of one TAF period, decoded from its groups.
#[derive(Default)]
struct Conditions {
    wind: Option<String>,
    vis: Option<f64>,
    vis_text: Option<String>,
    weather: Vec<String>,
    clouds: Vec<String>,
    ceiling: Option<f64>,
    other: Vec<String>,
    undecoded: Vec<String>,
}

fn decode_conditions(groups: &[&str], fmt: gp_base::parse::NumberFormat) -> Conditions {
    let mut c = Conditions::default();
    let mut i = 0;
    while i < groups.len() {
        let g = groups[i];
        if let Some(w) = parse_wind(g) {
            let mut t = match w.dir {
                _ if w.speed == 0.0 => "calm".to_owned(),
                None => format!(
                    "variable at {} kt",
                    display::number(w.speed, Precision::Decimals(0), fmt)
                ),
                Some(d) => format!(
                    "{:03}° true at {} kt",
                    d as i64,
                    display::number(w.speed, Precision::Decimals(0), fmt)
                ),
            };
            if let Some(gu) = w.gust {
                t.push_str(&format!(
                    ", gusting {} kt",
                    display::number(gu, Precision::Decimals(0), fmt)
                ));
            }
            c.wind = Some(t);
        } else if let Some((frac, qual)) = groups
            .get(i + 1)
            .filter(|n| is_whole(g) && n.contains('/'))
            .and_then(|n| parse_sm(n))
        {
            c.vis = Some(g.parse::<f64>().unwrap_or(0.0) + frac);
            c.vis_text = Some(format!(
                "{qual}{} {} SM",
                g,
                &groups[i + 1][..groups[i + 1].len() - 2]
            ));
            i += 1;
        } else if let Some((v, qual)) = parse_sm(g) {
            c.vis = Some(v);
            c.vis_text = Some(format!(
                "{qual}{} SM",
                &g[if qual.is_empty() { 0 } else { 1 }..g.len() - 2]
            ));
        } else if g.len() == 4 && g.bytes().all(|b| b.is_ascii_digit()) {
            let m: f64 = g.parse().unwrap_or(0.0);
            c.vis = Some(if g == "9999" { 10_000.0 } else { m } / 1609.344);
            c.vis_text = Some(if g == "9999" {
                "10 km or more".into()
            } else {
                format!("{} m", display::number(m, Precision::Decimals(0), fmt))
            });
        } else if g == "CAVOK" {
            c.vis = Some(10_000.0 / 1609.344);
            c.vis_text = Some("10 km or more (CAVOK)".into());
        } else if g == "NSW" {
            c.other.push("no significant weather".into());
        } else if let Some(w) = parse_weather(g) {
            c.weather.push(w);
        } else if let Some(cl) = parse_cloud(g) {
            let mut t = cl.cover.to_owned();
            if let Some(b) = cl.base_ft {
                t.push_str(&format!(
                    " at {} ft",
                    display::number(b, Precision::Decimals(0), fmt)
                ));
            }
            if let Some(k) = cl.kind {
                t.push_str(&format!(", {k}"));
            }
            if matches!(cl.code, "BKN" | "OVC" | "VV") && c.ceiling.is_none() {
                c.ceiling = cl.base_ft;
            }
            c.clouds.push(t);
        } else if let Some(ws) = g.strip_prefix("WS").and_then(|r| r.split_once('/')) {
            match (ws.0.parse::<f64>(), parse_wind(ws.1)) {
                (Ok(h), Some(w)) => c.other.push(format!(
                    "low-level wind shear at {} ft: {} kt from {}°",
                    display::number(h * 100.0, Precision::Decimals(0), fmt),
                    display::number(w.speed, Precision::Decimals(0), fmt),
                    w.dir
                        .map_or("variable".into(), |d| format!("{:03}", d as i64))
                )),
                _ => c.undecoded.push(g.into()),
            }
        } else {
            c.undecoded.push(g.into());
        }
        i += 1;
    }
    c
}

const PERIOD_ROW: &[Field] = &[
    text(
        "change",
        "Change",
        "base, from, temporary, becoming, or probability",
        40,
    ),
    text("from", "From", "UTC", 24),
    text("to", "To", "UTC", 24),
    text("from_local", "From (local)", "With the offset you gave", 40).optional(),
    Field::new(
        "start_hour",
        "Start",
        "Hours after the forecast starts",
        Kind::Number {
            min: 0.0,
            max: 999.0,
        },
    )
    .precision(Precision::Decimals(1)),
    text("wind", "Wind", "True-referenced", 80).optional(),
    text("visibility", "Visibility", "Prevailing", 40).optional(),
    text("weather", "Weather", "Plain words", 200).optional(),
    text("clouds", "Clouds", "Layers", 200).optional(),
    qty(
        "ceiling",
        "Ceiling",
        "Lowest broken, overcast, or vertical visibility",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0))
    .optional(),
    text(
        "flight_category",
        "Flight category",
        "VFR, MVFR, IFR, or LIFR when visibility or ceiling is forecast",
        8,
    )
    .optional(),
    text("other", "Other", "Wind shear and remarks", 200).optional(),
    text(
        "not_decoded",
        "Not decoded",
        "Groups this decoder does not read",
        200,
    )
    .optional(),
];

pub static TAF: ToolDef = ToolDef {
    id: "aviation.weather.taf-decode",
    title: "TAF decoder",
    summary: "Turns a pasted TAF into a timeline of forecast periods (FM, TEMPO, BECMG, PROB30/40) in plain language, with UTC and local times, winds (true), visibility, weather, clouds and ceilings, and flight categories.",
    aliases: &[
        "TAF decoder",
        "decode TAF",
        "terminal aerodrome forecast decoder",
        "TAF translator",
    ],
    keywords: &[
        "TAF",
        "forecast",
        "FM",
        "TEMPO",
        "BECMG",
        "PROB30",
        "PROB40",
        "ceiling",
        "visibility",
        "wind shear",
    ],
    inputs: &[
        text(
            "report",
            "TAF",
            "Paste a TAF, like TAF KDEN 181720Z 1818/1918 30012KT P6SM SCT080 FM190200 ...",
            4000,
        )
        .required()
        .core(),
        text(
            "utc_offset",
            "Local UTC offset",
            "Optional, like -06:00, for local times",
            10,
        )
        .core(),
    ],
    outputs: &[
        text("station", "Station", "ICAO identifier", 8),
        text("issued", "Issued", "Day and time, UTC", 24),
        text("valid_from", "Valid from", "UTC", 24),
        text("valid_to", "Valid to", "UTC", 24),
        Field::new(
            "valid_hours",
            "Valid for",
            "Hours",
            Kind::Number {
                min: 0.0,
                max: 999.0,
            },
        )
        .precision(Precision::Decimals(0)),
        text("amendment", "Amendment", "AMD or COR", 12).optional(),
        Field::new(
            "periods",
            "Periods",
            "The forecast timeline",
            Kind::List {
                items: PERIOD_ROW,
                min: 0,
                max: 60,
            },
        ),
        Field::new(
            "count",
            "Periods",
            "Number of periods",
            Kind::Number {
                min: 0.0,
                max: 60.0,
            },
        )
        .precision(Precision::Decimals(0)),
        text("notice", "Notice", "Safety framing", 120),
    ],
    warnings: &["SUSPECT_VALUE", "EXPERIMENTAL_TOOL"],
    model: "TAF coding as in the FAA Aviation Weather Handbook and FAA Order JO 7900.5E groups; day numbers below the start day belong to the next month",
    accuracy: "Decodes the text as written. It does not check the forecast against the station or fetch anything.",
    references: &[WEATHER_HANDBOOK, JO_7900_5],
    examples: &[Example {
        id: "primary",
        title: "A Denver TAF valid across midnight",
        input: r#"{"report":"TAF KDEN 181720Z 1818/1918 30012G22KT P6SM SCT080 BKN200 TEMPO 1820/1824 VRB25G35KT 3SM TSRA BKN060CB FM190200 32008KT P6SM FEW100 BECMG 1910/1912 18010KT PROB30 1914/1918 3SM -SHRA BKN030","utc_offset":"-06:00"}"#,
        source: "add-practitioner-essentials TAF scenario: valid 18:00Z on the 18th to 18:00Z on the 19th, each change group placed in order",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.weather.metar-decode",
            reason: "alternative",
        },
        Related {
            id: "aviation.wind.runway-components",
            reason: "next",
        },
    ],
    sentence: "{station} forecast from {valid_from} to {valid_to}, {count} {plural count \"period\" \"periods\"}. Get a current official briefing before flight.",
    limits: &[("batchRows", 1_000)],
    run: run_taf,
    ..ToolDef::BLANK
};

fn run_taf(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let raw = ctx.text("report")?.unwrap_or_default().to_ascii_uppercase();
    let raw = raw.trim_end_matches('=').to_owned();
    let mut toks: Vec<&str> = raw.split_whitespace().collect();
    let fmt = ctx.options.format;
    let offset = match ctx.text("utc_offset")? {
        Some(s) => Some(parse_offset(&s).ok_or_else(|| {
            ToolError::invalid("/utc_offset", "Use an offset like -06:00 or +05:30.")
        })?),
        None => None,
    };
    if toks.first() == Some(&"TAF") {
        toks.remove(0);
    }
    let amendment = toks
        .first()
        .filter(|t| matches!(**t, "AMD" | "COR"))
        .map(|s| s.to_string());
    if amendment.is_some() {
        toks.remove(0);
    }
    let bad = || {
        ToolError::invalid(
            "/report",
            "A TAF starts with the station, the issue time, and the valid period, like KDEN 181720Z 1818/1918.",
        )
    };
    let station = toks
        .first()
        .copied()
        .filter(|s| s.len() == 4 && s.as_bytes()[0].is_ascii_alphabetic())
        .ok_or_else(bad)?;
    let issued = toks.get(1).and_then(|s| ddhhmm(s)).ok_or_else(bad)?;
    let (vs, ve) = toks.get(2).and_then(|s| taf_range(s)).ok_or_else(bad)?;
    // Without the month, a wrap past the start day is taken as a 31-day month;
    // it affects only the hour counts, never the printed day numbers.
    let month_len = 31;
    let valid_hours = ve.hours_from(vs, month_len);
    if !(1..=48).contains(&valid_hours) {
        ctx.warnings.push(
            Warning::new(
                "SUSPECT_VALUE",
                format!("The valid period is {valid_hours} hours; TAFs cover up to 30 hours."),
            )
            .at("/report"),
        );
    }
    // Split the rest into periods at FM, TEMPO, BECMG, and PROBnn.
    let mut periods: Vec<(String, TafTime, TafTime, Vec<&str>)> =
        vec![("base".into(), vs, ve, Vec::new())];
    let mut i = 3;
    while i < toks.len() {
        let g = toks[i];
        if let Some(t) = g.strip_prefix("FM").and_then(taf_time) {
            // An FM period runs to the next FM, or the end of the forecast.
            periods.push(("from".into(), t, ve, Vec::new()));
        } else if matches!(g, "TEMPO" | "BECMG") {
            let r = toks.get(i + 1).and_then(|s| taf_range(s));
            if let Some((a, b)) = r {
                periods.push((
                    if g == "TEMPO" {
                        "temporary".into()
                    } else {
                        "becoming".into()
                    },
                    a,
                    b,
                    Vec::new(),
                ));
                i += 1;
            } else {
                periods.last_mut().expect("base").3.push(g);
            }
        } else if let Some(p) = g.strip_prefix("PROB").filter(|p| p == &"30" || p == &"40") {
            let tempo = toks.get(i + 1) == Some(&"TEMPO");
            let r = toks.get(i + 1 + tempo as usize).and_then(|s| taf_range(s));
            if let Some((a, b)) = r {
                periods.push((
                    format!("{p}% probability{}", if tempo { ", temporary" } else { "" }),
                    a,
                    b,
                    Vec::new(),
                ));
                i += 1 + tempo as usize;
            } else {
                periods.last_mut().expect("base").3.push(g);
            }
        } else if g == "RMK" {
            break;
        } else {
            periods.last_mut().expect("base").3.push(g);
        }
        i += 1;
    }
    // Close each FM period at the next FM (the base ends at the first FM too).
    let fm_starts: Vec<(usize, TafTime)> = periods
        .iter()
        .enumerate()
        .filter(|(k, p)| *k == 0 || p.0 == "from")
        .map(|(k, p)| (k, p.1))
        .collect();
    for w in fm_starts.windows(2) {
        periods[w[0].0].2 = w[1].1;
    }
    let mut rows = Vec::new();
    for (kind, from, to, groups) in &periods {
        let c = decode_conditions(groups, fmt);
        let mut row = vec![
            ("change", Json::str(kind.as_str())),
            ("from", Json::str(from.text())),
            ("to", Json::str(to.text())),
        ];
        if let Some(off) = offset {
            row.push(("from_local", Json::str(from.local(off))));
        }
        row.push((
            "start_hour",
            Json::Num(from.hours_from(vs, month_len) as f64 + from.minute as f64 / 60.0),
        ));
        if let Some(w) = c.wind {
            row.push(("wind", Json::str(w)));
        }
        if let Some(v) = c.vis_text {
            row.push(("visibility", Json::str(v)));
        }
        if !c.weather.is_empty() {
            row.push(("weather", Json::str(c.weather.join("; "))));
        }
        if !c.clouds.is_empty() {
            row.push(("clouds", Json::str(c.clouds.join("; "))));
        }
        if let Some(ce) = c.ceiling {
            row.push((
                "ceiling",
                ctx.emit("ceiling", q(ce, QT::Length, "ft"), unit(QT::Length, "ft")),
            ));
        }
        if c.vis.is_some() || c.ceiling.is_some() {
            row.push((
                "flight_category",
                Json::str(flight_category(c.ceiling, c.vis)),
            ));
        }
        if !c.other.is_empty() {
            row.push(("other", Json::str(c.other.join("; "))));
        }
        if !c.undecoded.is_empty() {
            row.push(("not_decoded", Json::str(c.undecoded.join(" "))));
        }
        rows.push(Json::obj(row));
    }
    let mut o = vec![
        ("station", Json::str(station)),
        (
            "issued",
            Json::str(format!(
                "day {} at {:02}{:02}Z",
                issued.0, issued.1, issued.2
            )),
        ),
        ("valid_from", Json::str(vs.text())),
        ("valid_to", Json::str(ve.text())),
        ("valid_hours", Json::Num(valid_hours as f64)),
    ];
    if let Some(a) = amendment {
        o.push(("amendment", Json::str(a)));
    }
    let n = rows.len();
    o.push(("periods", Json::Arr(rows)));
    o.push(("count", Json::Num(n as f64)));
    o.push(("notice", Json::str(SAFETY)));
    Ok(obj(o))
}
