//! Sun tools (solar-and-twilight spec): position, rise/set/twilight with
//! explicit polar states, the four US aviation "nights", and the mapping
//! window. Events are tied to the requested local date and shown in local
//! time and Zulu, each dated.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::point;
use libm::{cos, sin, tan};

use crate::sun::{self, Crossing, SUNRISE_ALTITUDE};
use crate::{ZoneSpec, civil, text};

const NOAA: Reference = Reference {
    title: "NOAA Solar Calculator: General Solar Position Calculations",
    issuer: "NOAA Global Monitoring Laboratory",
    year: 2023,
    edition: "Spreadsheet and equations after Meeus (1998)",
    locator: "Declination, equation of time, hour angle, refraction, and sunrise at −0.833°",
    url: "https://gml.noaa.gov/grad/solcalc/calcdetails.html",
};
const MEEUS: Reference = Reference {
    title: "Astronomical Algorithms",
    issuer: "Meeus, J., Willmann-Bell",
    year: 1998,
    edition: "2nd edition",
    locator: "Chapters 25 (solar coordinates) and 28 (equation of time)",
    url: "https://www.willbell.com/math/mc1.htm",
};
const CFR_1_1_NIGHT: Reference = Reference {
    title: "14 CFR 1.1, General definitions (night)",
    issuer: "Federal Aviation Administration",
    year: 2024,
    edition: "Reviewed 2026-09-18",
    locator: "\"Night\": end of evening civil twilight to beginning of morning civil twilight, as published in the Air Almanac",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-A/part-1/section-1.1",
};
pub(crate) const CFR_61_57: Reference = Reference {
    title: "14 CFR 61.57(b), Night takeoff and landing experience",
    issuer: "Federal Aviation Administration",
    year: 2024,
    edition: "Reviewed 2026-09-18",
    locator: "Three takeoffs and three full-stop landings from 1 hour after sunset to 1 hour before sunrise, within the preceding 90 days",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-D/part-61/subpart-A/section-61.57",
};
const CFR_91_209: Reference = Reference {
    title: "14 CFR 91.209, Aircraft lights",
    issuer: "Federal Aviation Administration",
    year: 2024,
    edition: "Reviewed 2026-09-18",
    locator: "Lighted position lights from sunset to sunrise (in Alaska, during the period a prominent unlighted object cannot be seen from 3 statute miles)",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-C/section-91.209",
};
const CFR_107_29: Reference = Reference {
    title: "14 CFR 107.29, Operation at night",
    issuer: "Federal Aviation Administration",
    year: 2021,
    edition: "Reviewed 2026-09-18",
    locator: "§107.29(c): civil twilight is 30 minutes before official sunrise and after official sunset, except in Alaska (the Air Almanac period)",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.29",
};

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");
const DATE: Field = text("date", "Local date", "Like 2026-06-21")
    .required()
    .core();
const OFFSET: Field = text(
    "offset",
    "Time zone or UTC offset",
    "Like America/Denver, or -06:00 for MDT",
)
.required()
.core();

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

/// The local date as a day number.
fn local_date(ctx: &Ctx) -> Result<i64, ToolError> {
    let s = ctx.text("date")?.expect("required");
    if s.trim().len() != 10 {
        return Err(ToolError::invalid(
            "/date",
            format!("\"{s}\" is not a date like 2026-06-21."),
        ));
    }
    civil::parse_stamp(&s)
        .map(|st| st.days)
        .map_err(|m| ToolError::invalid("/date", m))
}

const JD_UNIX: f64 = 2_440_587.5;

/// Julian date (UT) for a day number and minutes of day in UTC.
pub(crate) fn jd(days: i64, minutes: f64) -> f64 {
    JD_UNIX + days as f64 + minutes / 1440.0
}

/// A UT Julian date rounded to the minute, as (day number, minutes of day).
fn split(jd: f64) -> (i64, i64) {
    let total = ((jd - JD_UNIX) * 1440.0).round() as i64;
    (total.div_euclid(1440), total.rem_euclid(1440))
}

/// `2026-06-21 21:14 local (2026-06-22 0314Z)`.
fn event_text(jd: f64, zone: &ZoneSpec) -> String {
    let (ud, um) = split(jd);
    let local = ud * 1440 + um + i64::from(zone.minutes_at((ud * 1440 + um) * 60));
    let (ld, lm) = (local.div_euclid(1440), local.rem_euclid(1440));
    format!(
        "{} {:02}:{:02} local ({} {:02}{:02}Z)",
        civil::date(ld),
        lm / 60,
        lm % 60,
        civil::date(ud),
        um / 60,
        um % 60
    )
}

/// Local clock `HH:MM` for an event.
fn local_clock(jd: f64, zone: &ZoneSpec) -> String {
    let (ud, um) = split(jd);
    let m = (ud * 1440 + um + i64::from(zone.minutes_at((ud * 1440 + um) * 60))).rem_euclid(1440);
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn duration(minutes: f64) -> String {
    let m = minutes.round() as i64;
    format!("{} h {} min", m / 60, m % 60)
}

/// The transit nearest local noon of the local date.
pub(crate) fn local_noon(lon: f64, day: i64, zone: &ZoneSpec) -> f64 {
    let offset = zone.minutes_at(day * 86_400 + 43_200);
    sun::transit(lon, jd(day, 720.0 - f64::from(offset)))
}

// ---------------------------------------------------------------- position

const NREL_SPA: Reference = Reference {
    title: "Solar Position Algorithm for Solar Radiation Applications (NREL/TP-560-34302)",
    issuer: "Reda, I., and Andreas, A., National Renewable Energy Laboratory",
    year: 2008,
    edition: "Revised January 2008",
    locator: "Sections 3.1 to 3.15 and Tables A4.2, A4.3; worked example in Table A5.1",
    url: "https://www.nrel.gov/docs/fy08osti/34302.pdf",
};
const ESPENAK_MEEUS: Reference = Reference {
    title: "Polynomial Expressions for Delta T",
    issuer: "Espenak, F., and Meeus, J., NASA Goddard Space Flight Center",
    year: 2006,
    edition: "Five Millennium Canon of Solar Eclipses",
    locator: "ΔT polynomials, −1999 to +3000",
    url: "https://eclipse.gsfc.nasa.gov/SEcat5/deltatpoly.html",
};

const fn opt_q(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    unit: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit })
}

pub static POSITION: ToolDef = ToolDef {
    id: "time.sun.position",
    stability: gp_base::tool::Stability::Stable,
    title: "Sun position (azimuth and elevation)",
    summary: "The sun's azimuth, elevation, declination, and hour angle for a place and time by the NREL Solar Position Algorithm, with the NOAA result as a cross-check, shadow length, and incidence on a slope.",
    aliases: &[
        "sun position calculator",
        "solar azimuth",
        "sun angle calculator",
        "shadow length",
        "solar position algorithm",
    ],
    keywords: &[
        "sun",
        "solar",
        "azimuth",
        "elevation",
        "zenith",
        "declination",
        "shadow",
        "slope",
        "incidence",
        "SPA",
        "NREL",
    ],
    inputs: &[
        LAT,
        LON,
        text(
            "time",
            "Time",
            "With Z or an offset, like 2026-06-21T12:00-06:00",
        )
        .required()
        .core(),
        Field::new(
            "object_height",
            "Object height",
            "For shadow length, like 30 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .core(),
        Field::new(
            "slope",
            "Slope",
            "Surface tilt from level, like 20 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[0,90]"),
        Field::new(
            "aspect",
            "Aspect",
            "Direction the slope faces, from north, like 180 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[0,360)"),
        opt_q(
            "height",
            "Observer height",
            "Above the ellipsoid, for parallax (default 0)",
            QT::Length,
            "m",
        ),
        opt_q(
            "pressure",
            "Air pressure",
            "Average local pressure for refraction (default 1013.25 hPa)",
            QT::Pressure,
            "hPa",
        ),
        opt_q(
            "temperature",
            "Air temperature",
            "Average local temperature for refraction (default 12 °C)",
            QT::Temperature,
            "degC",
        ),
        opt_q(
            "delta_t",
            "ΔT (TT − UT1)",
            "Override the dated ΔT estimate, like 69.2 s",
            QT::Time,
            "s",
        ),
        opt_q(
            "dut1",
            "DUT1 (UT1 − UTC)",
            "From IERS Bulletin A, like -0.03 s (survey azimuths)",
            QT::Time,
            "s",
        ),
        Field::new(
            "precision",
            "Purpose",
            "standard (default) or survey (asks for DUT1)",
            Kind::Choice(&["standard", "survey"]),
        ),
    ],
    outputs: &[
        Field::new(
            "elevation",
            "Elevation",
            "Apparent, with refraction",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("[-90,90]"),
        Field::new(
            "azimuth",
            "Azimuth",
            "From true north, clockwise",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("[0,360)"),
        Field::new(
            "zenith",
            "Zenith angle",
            "90° minus the apparent elevation",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("[0,180]"),
        Field::new(
            "elevation_true",
            "Geometric elevation",
            "Without refraction",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("[-90,90]"),
        Field::new(
            "declination",
            "Declination",
            "Topocentric",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("[-90,90]"),
        Field::new(
            "equation_of_time",
            "Equation of time",
            "Sundial time minus clock time",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "hour_angle",
            "Hour angle",
            "Topocentric, negative before solar noon",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("[-180,180)"),
        Field::new(
            "delta_t_used",
            "ΔT used",
            "TT − UT1",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "noaa_elevation",
            "NOAA cross-check elevation",
            "Low-accuracy NOAA equations",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("[-90,90]"),
        Field::new(
            "noaa_azimuth",
            "NOAA cross-check azimuth",
            "Low-accuracy NOAA equations",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("[0,360)"),
        Field::new(
            "shadow_length",
            "Shadow length",
            "On level ground",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(3))
        .optional(),
        Field::new(
            "shadow_azimuth",
            "Shadow direction",
            "Azimuth the shadow points",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)")
        .optional(),
        Field::new(
            "incidence",
            "Incidence on the slope",
            "Angle between the sun and the slope's normal",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,180]")
        .optional(),
    ],
    errors: &[],
    warnings: &[
        "SUN_BELOW_HORIZON",
        "UT1_APPROXIMATED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "NREL Solar Position Algorithm (topocentric, with refraction from pressure and temperature); ΔT from Espenak and Meeus unless given; NOAA equations as a cross-check",
    accuracy: "±0.0003° (SPA, −2000 to 6000) with UT1; taking UT1 as UTC adds up to about 0.004° of azimuth",
    when_to_use: "Use this when you need where the sun is at a moment: the azimuth and elevation for shadow studies, solar panel aiming, camera and survey planning, glare and hotspot checks, or the incidence angle on a sloping surface. It also gives the shadow an object of a given height casts, with its direction.",
    limitations: "Refraction near the horizon depends on the pressure and temperature you give it, and a low sun is where that matters most. Taking UT1 as UTC costs up to about four thousandths of a degree of azimuth, which is only a concern for survey work, where DUT1 from IERS belongs in the input. It is the sun's geometry, not the weather: cloud, haze, and terrain are yours to account for.",
    references: &[NREL_SPA, ESPENAK_MEEUS, NOAA],
    examples: &[Example {
        id: "primary",
        title: "Denver at 12:00 MDT on the June solstice",
        input: r#"{"lat":39.7392,"lon":-104.9903,"time":"2026-06-21T12:00-06:00","object_height":"10 m"}"#,
        source: "NREL SPA",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.sun.events",
            reason: "next",
        },
        Related {
            id: "time.sun.aviation-nights",
            reason: "alternative",
        },
        Related {
            id: "time.sun.mapping-window",
            reason: "next",
        },
    ],
    sentence: "The sun is {elevation} up, at azimuth {azimuth}.{warn SUN_BELOW_HORIZON} It is below the horizon.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_position,
    ..ToolDef::BLANK
};

fn run_position(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let t = ctx.text("time")?.expect("required");
    let st = civil::parse_stamp(&t).map_err(|m| ToolError::invalid("/time", m))?;
    if st.offset.is_none() {
        return Err(ToolError::invalid(
            "/time",
            "Add Z for UTC or the UTC offset, like 2026-06-21T12:00-06:00.",
        ));
    }
    let (d, s) = crate::to_utc(st);
    let secs = |q: Option<Q>| q.map(|q| q.to(units::by_symbol(QT::Time, "s").expect("s")));
    let dut1 = secs(ctx.quantity("dut1")?);
    if let Some(v) = dut1
        && v.abs() > 0.9
    {
        return Err(ToolError::invalid(
            "/dut1",
            "DUT1 stays within ±0.9 s by definition.",
        ));
    }
    if ctx.choice("precision")? == Some("survey") && dut1.is_none() {
        ctx.warnings.push(Warning::new(
            "UT1_APPROXIMATED",
            "UT1 was taken as UTC. That can shift the azimuth by up to about 0.004°; give DUT1 from IERS Bulletin A for survey work.",
        ));
    }
    let jd_utc = jd(d, s.min(86_400.0) / 60.0);
    let jd_ut1 = jd_utc + dut1.unwrap_or(0.0) / 86_400.0;
    let (y, mo, _) = civil::civil_from_days(d);
    let dt = secs(ctx.quantity("delta_t")?)
        .unwrap_or_else(|| crate::spa::delta_t(y as f64, f64::from(mo)));
    let height = ctx
        .quantity("height")?
        .map_or(0.0, |h| h.to(units::by_symbol(QT::Length, "m").expect("m")));
    let pressure = ctx.quantity("pressure")?.map_or(1013.25, |p| {
        p.to(units::by_symbol(QT::Pressure, "hPa").expect("hPa"))
    });
    let temperature = ctx.quantity("temperature")?.map_or(12.0, |t| {
        t.to(units::by_symbol(QT::Temperature, "degC").expect("degC"))
    });
    let p = crate::spa::position(
        jd_ut1,
        dt,
        crate::spa::Observer {
            lat,
            lon,
            elevation: height,
            pressure,
            temperature,
            atmos_refract: 0.5667,
        },
    );
    let apparent = 90.0 - p.zenith;
    if apparent < 0.0 {
        ctx.warnings.push(Warning::new(
            "SUN_BELOW_HORIZON",
            "The sun is below the horizon at this time.",
        ));
    }
    let noaa = sun::position(lat, lon, jd_utc);
    let ha = (p.hour_angle + 180.0).rem_euclid(360.0) - 180.0;
    let mut out = vec![
        ("elevation", ctx.out("elevation", deg(apparent))),
        ("azimuth", ctx.out("azimuth", deg(p.azimuth))),
        ("zenith", ctx.out("zenith", deg(p.zenith))),
        (
            "elevation_true",
            ctx.out("elevation_true", deg(p.elevation_true)),
        ),
        ("declination", ctx.out("declination", deg(p.declination))),
        (
            "equation_of_time",
            ctx.out(
                "equation_of_time",
                Q {
                    value: p.eot,
                    unit: units::by_symbol(QT::Time, "min").expect("min"),
                },
            ),
        ),
        ("hour_angle", ctx.out("hour_angle", deg(ha))),
        (
            "delta_t_used",
            ctx.out(
                "delta_t_used",
                Q {
                    value: dt,
                    unit: units::by_symbol(QT::Time, "s").expect("s"),
                },
            ),
        ),
        (
            "noaa_elevation",
            ctx.out("noaa_elevation", deg(noaa.elevation + noaa.refraction)),
        ),
        ("noaa_azimuth", ctx.out("noaa_azimuth", deg(noaa.azimuth))),
    ];
    if let Some(h) = ctx.quantity("object_height")?
        && apparent > 0.0
    {
        let u = h.unit;
        out.push((
            "shadow_length",
            ctx.emit(
                "shadow_length",
                Q {
                    value: h.value / tan(apparent.to_radians()),
                    unit: u,
                },
                u,
            ),
        ));
        out.push((
            "shadow_azimuth",
            ctx.out("shadow_azimuth", deg((p.azimuth + 180.0).rem_euclid(360.0))),
        ));
    }
    let (slope, aspect) = (
        point::plain_angle(ctx, "slope")?,
        point::plain_angle(ctx, "aspect")?,
    );
    if let Some(sl) = slope {
        let asp = aspect.ok_or_else(|| {
            ToolError::invalid(
                "/aspect",
                "Give the direction the slope faces with its tilt.",
            )
        })?;
        let z = p.zenith.to_radians();
        let c = cos(z) * cos(sl.to_radians())
            + sin(z) * sin(sl.to_radians()) * cos((p.azimuth - asp).to_radians());
        out.push((
            "incidence",
            ctx.out("incidence", deg(c.clamp(-1.0, 1.0).acos().to_degrees())),
        ));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- events

const EVENT_OUT: [Field; 12] = [
    text(
        "sunrise",
        "Sunrise",
        "Upper limb at the horizon (−0.833°), or the polar state",
    ),
    text("sunset", "Sunset", "Or the polar state"),
    text("solar_noon", "Solar noon", "The sun's highest point"),
    text("day_length", "Day length", "Sunrise to sunset"),
    text(
        "civil_dawn",
        "Civil dawn",
        "Morning civil twilight begins (−6°)",
    ),
    text(
        "civil_dusk",
        "Civil dusk",
        "Evening civil twilight ends (−6°)",
    ),
    text("nautical_dawn", "Nautical dawn", "−12°"),
    text("nautical_dusk", "Nautical dusk", "−12°"),
    text("astronomical_dawn", "Astronomical dawn", "−18°"),
    text("astronomical_dusk", "Astronomical dusk", "−18°"),
    text("state", "Day type", "normal, polar-day, or polar-night"),
    Field::new(
        "day_minutes",
        "Day length (minutes)",
        "Sunrise to sunset",
        Kind::Number {
            min: 0.0,
            max: 1440.0,
        },
    )
    .precision(Precision::Decimals(0))
    .measure("time", "min"),
];

pub static EVENTS: ToolDef = ToolDef {
    id: "time.sun.events",
    stability: gp_base::tool::Stability::Stable,
    title: "Sunrise, sunset, and twilight",
    summary: "Sunrise, sunset, solar noon, day length, and civil, nautical, and astronomical twilight for a place and local date, in local time and Zulu, with polar states.",
    aliases: &[
        "sunrise sunset calculator",
        "civil twilight calculator",
        "twilight times",
    ],
    keywords: &[
        "sunrise",
        "sunset",
        "twilight",
        "civil twilight",
        "nautical twilight",
        "dawn",
        "dusk",
        "solar noon",
        "day length",
        "polar night",
    ],
    inputs: &[
        LAT,
        LON,
        DATE,
        OFFSET,
        Field::new(
            "height",
            "Observer height",
            "Above the surrounding terrain or sea, for horizon dip (optional), like 10 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
    ],
    outputs: &EVENT_OUT,
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Geometric sun from the NREL Solar Position Algorithm; each crossing found by bisection between the day's highest and lowest points, which also decide polar states; sunrise and sunset at −0.833° (minus horizon dip when a height is given); solar noon from the NOAA equation of time",
    accuracy: "Within 1 minute of USNO on 1,200 events (300 places and dates, 94% to the same minute), including twilights that graze their altitude",
    when_to_use: "Use this when you need the day's light: sunrise and sunset for a place and date, solar noon, the length of the day, and the three twilights, in local time and Zulu. It is the tool behind planning a flight, a survey, a shoot, or any outdoor work that has to finish before the light goes.",
    limitations: "The times are for a level horizon at sea level: hills, a valley, or a tall building move sunrise and sunset by minutes, and refraction near the horizon varies with the weather. Inside the polar circles the sun may not rise or set at all, which the result states rather than inventing a time for. Legal twilight comes from the Air Almanac.",
    references: &[NREL_SPA, NOAA, MEEUS],
    examples: &[Example {
        id: "primary",
        title: "Denver on the June solstice (MDT, UTC−6)",
        input: r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-06-21","offset":"-06:00"}"#,
        source: "add-practitioner-essentials scenario: evening events after 00:00Z are dated on the next UTC day",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.sun.aviation-nights",
            reason: "next",
        },
        Related {
            id: "time.sun.position",
            reason: "next",
        },
        Related {
            id: "time.sun.mapping-window",
            reason: "next",
        },
    ],
    sentence: "Sunrise is {sunrise} and sunset is {sunset}.",
    limits: &[("batchRows", 1_000)],
    run: run_events,
    ..ToolDef::BLANK
};

fn run_events(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let day = local_date(ctx)?;
    let off = crate::ZoneSpec::parse(ctx, "offset")?;
    let off = &off;
    let dip = match ctx.quantity("height")? {
        Some(h) => sun::horizon_dip(h.to(units::by_symbol(QT::Length, "m").expect("m"))),
        None => 0.0,
    };
    let noon = local_noon(lon, day, off);
    let rise = sun::crossings(lat, lon, noon, SUNRISE_ALTITUDE - dip);
    let (sunrise, sunset, state, minutes) = match rise {
        Crossing::Times(r, s) => (
            event_text(r, off),
            event_text(s, off),
            "normal",
            (s - r) * 1440.0,
        ),
        Crossing::AlwaysAbove => ("polar-day".into(), "polar-day".into(), "polar-day", 1440.0),
        Crossing::AlwaysBelow => (
            "polar-night".into(),
            "polar-night".into(),
            "polar-night",
            0.0,
        ),
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        // The solver is iterative, so the honest work is the geometry it solves:
        // where solar noon falls, what altitude counts as sunrise, and the
        // crossing it finds either side of noon.
        ctx.step(
            "Solar noon",
            "when the sun crosses this longitude's meridian",
            format!("at {}° longitude", n(lon, 6)),
            event_text(noon, off),
        );
        ctx.step(
            "Sunrise altitude",
            "−0.8333° for the sun's radius and refraction, less the dip from height",
            format!("{}° − {}° of dip", n(SUNRISE_ALTITUDE, 4), n(dip, 4)),
            format!("{}°", n(SUNRISE_ALTITUDE - dip, 4)),
        );
        ctx.step(
            "Sunrise",
            "the crossing of that altitude before solar noon",
            format!("at {}°, {}°", n(lat, 6), n(lon, 6)),
            sunrise.clone(),
        );
    }
    let mut out = vec![
        ("sunrise", Json::str(sunrise)),
        ("sunset", Json::str(sunset)),
        ("solar_noon", Json::str(event_text(noon, off))),
        ("day_length", Json::str(duration(minutes))),
    ];
    for (alt, dawn, dusk, kind) in [
        (-6.0, "civil_dawn", "civil_dusk", "civil"),
        (-12.0, "nautical_dawn", "nautical_dusk", "nautical"),
        (
            -18.0,
            "astronomical_dawn",
            "astronomical_dusk",
            "astronomical",
        ),
    ] {
        // Each side on its own: near the edge of the polar summer an evening
        // twilight can end just before midnight after a night when it never began.
        let side = |sign: f64, edge: &str| match sun::crossing(lat, lon, noon, alt, sign) {
            sun::Side::At(t) => event_text(t, off),
            // The sun never gets that low: the twilight never begins or ends.
            sun::Side::Above => format!("no-{kind}-twilight-{edge}"),
            // The sun never gets that high: no twilight of this kind at all.
            sun::Side::Below => format!("no-{kind}-twilight"),
        };
        let (a, b) = (side(-1.0, "begin"), side(1.0, "end"));
        out.push((dawn, Json::str(a)));
        out.push((dusk, Json::str(b)));
    }
    out.push(("state", Json::str(state)));
    out.push(("day_minutes", Json::Num(minutes.round())));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- aviation nights

pub static AVIATION_NIGHTS: ToolDef = ToolDef {
    id: "time.sun.aviation-nights",
    stability: gp_base::tool::Stability::Stable,
    title: "The four aviation nights",
    summary: "All four US aviation night periods for a place and date, each with its regulation: logging night, passenger-currency night, position lights, and Part 107 civil twilight.",
    aliases: &[
        "when does night start for pilots",
        "logging night time",
        "Part 107 civil twilight",
    ],
    keywords: &[
        "night",
        "night currency",
        "61.57",
        "91.209",
        "107.29",
        "civil twilight",
        "position lights",
        "logbook",
    ],
    inputs: &[
        LAT,
        LON,
        DATE,
        OFFSET,
        Field::new(
            "alaska",
            "In Alaska",
            "no (default) or yes: Alaska uses the Air Almanac twilight for Part 107",
            Kind::Choice(&["no", "yes"]),
        ),
        text(
            "landing_time",
            "Landing time (local)",
            "Optional, like 21:40 or 0510 (before noon means the next morning)",
        )
        .core(),
    ],
    outputs: &[
        text(
            "passenger_currency",
            "Passenger-currency night, §61.57(b)",
            "1 hour after sunset to 1 hour before sunrise",
        ),
        text(
            "logging_night",
            "Logging night, §1.1",
            "End of evening civil twilight to beginning of morning civil twilight",
        ),
        text(
            "position_lights",
            "Position lights, §91.209",
            "Sunset to sunrise",
        ),
        text(
            "part107_evening",
            "Part 107 evening twilight, §107.29(c)",
            "Lighting required; not a separate night",
        ),
        text(
            "part107_morning",
            "Part 107 morning twilight, §107.29(c)",
            "Lighting required; not a separate night",
        ),
        text(
            "currency_from",
            "Currency landings count from",
            "Local clock time",
        ),
        Field::new(
            "landing_logs_night",
            "Landing is loggable night",
            "yes or no",
            Kind::Text { max_len: 3 },
        )
        .optional(),
        Field::new(
            "landing_counts_currency",
            "Landing counts for passenger currency",
            "yes or no",
            Kind::Text { max_len: 3 },
        )
        .optional(),
        Field::new(
            "landing_state",
            "Landing check",
            "1 if loggable and counts, 2 if loggable only, 3 if neither",
            Kind::Number { min: 1.0, max: 3.0 },
        )
        .precision(Precision::Decimals(0))
        .optional(),
        text(
            "part107_lighting",
            "Part 107 lighting, §107.29(a)(2) and (b)",
            "What the drone must carry in civil twilight and at night",
        ),
    ],
    errors: &[],
    warnings: &[
        "CIVIL_TWILIGHT_APPROXIMATED",
        "INPUT_NORMALIZED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Sunrise and sunset (−0.833°) and civil twilight (−6°) from the NREL SPA geometric sun, for the evening of the date and the next morning",
    accuracy: "Within 1 minute of the windows built from USNO times at 100 places (491 windows); the Air Almanac, the legal source for twilight, tabulates to the minute",
    when_to_use: "Use this when a regulation turns on which night it is: logging night flight, carrying passengers on night currency, when position lights are required, and the civil-twilight window a Part 107 operation works to. All four are computed for one place and date, each shown beside the regulation that defines it, because they start and end at different times.",
    limitations: "These are the US definitions for the place and date you enter. The Air Almanac is the legal source for twilight and tabulates to the minute; this computes the same events from the sun's geometry and agrees within a minute. It does not know your aircraft's equipment, your currency record, or any waiver, and it does not decide whether a flight is legal.",
    references: &[CFR_1_1_NIGHT, CFR_61_57, CFR_91_209, CFR_107_29],
    examples: &[Example {
        id: "primary",
        title: "Denver on 2026-06-21, landing at 21:20 local",
        input: r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-06-21","offset":"-06:00","landing_time":"21:20"}"#,
        source: "add-practitioner-essentials scenario: loggable night that does not count toward §61.57(b)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.sun.events",
            reason: "parent",
        },
        Related {
            id: "time.sun.position",
            reason: "next",
        },
        Related {
            id: "time.scale.utc-offset",
            reason: "parent",
        },
    ],
    sentence: "Night landings for passenger currency count after {currency_from} local.{if landing_state == 1} This landing counts.{/if}{if landing_state == 2} This landing is loggable night but does not count for currency.{/if}{if landing_state == 3} This landing is not at night.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_nights,
    ..ToolDef::BLANK
};

/// A window from `a` to `b` as local and Zulu text.
fn window(a: f64, b: f64, off: &ZoneSpec) -> String {
    format!("{} to {}", event_text(a, off), event_text(b, off))
}

fn run_nights(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let day = local_date(ctx)?;
    let off = crate::ZoneSpec::parse(ctx, "offset")?;
    let off = &off;
    let alaska = ctx.choice("alaska")?.unwrap_or("no") == "yes";
    let today = local_noon(lon, day, off);
    let tomorrow = local_noon(lon, day + 1, off);
    let hour = 1.0 / 24.0;
    // Only this evening's side and tomorrow morning's side matter, each on its own.
    let side = |noon: f64, alt: f64, sign: f64| match sun::crossing(lat, lon, noon, alt, sign) {
        sun::Side::At(t) => Crossing::Times(t, t),
        sun::Side::Above => Crossing::AlwaysAbove,
        sun::Side::Below => Crossing::AlwaysBelow,
    };
    let set = side(today, SUNRISE_ALTITUDE, 1.0);
    let rise = side(tomorrow, SUNRISE_ALTITUDE, -1.0);
    let dusk = side(today, -6.0, 1.0);
    let dawn = side(tomorrow, -6.0, -1.0);
    ctx.warnings.push(Warning::new(
        "CIVIL_TWILIGHT_APPROXIMATED",
        "Logging night uses the sun at 6° below the horizon; the Air Almanac is the legal source and can differ by a minute.",
    ));
    let none = |why: &str| format!("none: {why}");
    let (sunset, sunrise) = match (set, rise) {
        (Crossing::Times(_, s), Crossing::Times(r, _)) => (Some(s), Some(r)),
        _ => (None, None),
    };
    let polar = |c: Crossing| match c {
        Crossing::AlwaysAbove => "the sun does not set",
        Crossing::AlwaysBelow => "the sun does not rise, so it is night all day",
        Crossing::Times(..) => "",
    };
    let (twilight_end, twilight_begin) = match (dusk, dawn) {
        (Crossing::Times(_, s), Crossing::Times(r, _)) => (Some(s), Some(r)),
        _ => (None, None),
    };
    let set_state = if let Crossing::Times(..) = set {
        rise
    } else {
        set
    };
    if ctx.explaining()
        && let (Some(s), Some(r)) = (sunset, sunrise)
    {
        // Four different rules, three different boundaries, one night. The work
        // is which boundary each rule uses, not the solver behind them.
        ctx.step(
            "Sunset and sunrise",
            "the sun's centre at −0.8333°, its radius and refraction allowed for",
            format!("at {}°, {}°", lat, lon),
            window(s, r, off),
        );
        if let (Some(te), Some(tb)) = (twilight_end, twilight_begin) {
            ctx.step(
                "Civil twilight",
                "the sun 6° below the horizon, which is when logging night begins",
                format!("−6° at {}°, {}°", lat, lon),
                window(te, tb, off),
            );
        }
        ctx.step(
            "Passenger currency",
            "one hour after sunset to one hour before sunrise",
            format!("an hour inside {}", window(s, r, off)),
            if r - s > 2.0 * hour {
                window(s + hour, r - hour, off)
            } else {
                none("the night is shorter than 2 hours")
            },
        );
    }
    let mut out = Vec::new();
    match (sunset, sunrise) {
        (Some(s), Some(r)) => {
            out.push((
                "passenger_currency",
                Json::str(if r - s > 2.0 * hour {
                    window(s + hour, r - hour, off)
                } else {
                    none("the night is shorter than 2 hours")
                }),
            ));
            out.push(("position_lights", Json::str(window(s, r, off))));
            if alaska {
                out.push((
                    "part107_evening",
                    Json::str(twilight_end.map_or_else(
                        || none("civil twilight lasts all night"),
                        |e| window(s, e, off),
                    )),
                ));
                out.push((
                    "part107_morning",
                    Json::str(twilight_begin.map_or_else(
                        || none("civil twilight lasts all night"),
                        |b| window(b, r, off),
                    )),
                ));
            } else {
                out.push(("part107_evening", Json::str(window(s, s + hour / 2.0, off))));
                out.push(("part107_morning", Json::str(window(r - hour / 2.0, r, off))));
            }
            out.push((
                "currency_from",
                Json::str(if r - s > 2.0 * hour {
                    local_clock(s + hour, off)
                } else {
                    "not tonight".into()
                }),
            ));
        }
        _ => {
            let why = polar(set_state);
            for k in [
                "passenger_currency",
                "position_lights",
                "part107_evening",
                "part107_morning",
            ] {
                out.push((k, Json::str(none(why))));
            }
            out.push(("currency_from", Json::str("not tonight")));
        }
    }
    out.push((
        "logging_night",
        Json::str(match (twilight_end, twilight_begin) {
            (Some(e), Some(b)) => window(e, b, off),
            _ => none(match dusk {
                Crossing::AlwaysAbove => "civil twilight lasts all night",
                _ => "the sun stays more than 6° below the horizon, so it is night all day",
            }),
        }),
    ));
    if let Some(t) = ctx.text("landing_time")? {
        let bad = || {
            ToolError::invalid(
                "/landing_time",
                format!("\"{t}\" is not a time like 21:40 or 0510."),
            )
        };
        let c = t.trim().replace(':', "");
        if c.len() != 4 || !c.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        let (h, m): (i64, i64) = (
            c[..2].parse().map_err(|_| bad())?,
            c[2..].parse().map_err(|_| bad())?,
        );
        if h > 23 || m > 59 {
            return Err(bad());
        }
        // Before local noon means the next morning.
        let local_min = h * 60 + m + if h < 12 { 1440 } else { 0 };
        let guess = i64::from(off.minutes_at(day * 86_400 + local_min * 60));
        let at = jd(day, (local_min - guess) as f64);
        let logs = match (twilight_end, twilight_begin) {
            (Some(e), Some(b)) => at >= e && at <= b,
            _ => matches!(dusk, Crossing::AlwaysBelow),
        };
        let counts = match (sunset, sunrise) {
            (Some(s), Some(r)) => at >= s + hour && at <= r - hour,
            _ => matches!(set_state, Crossing::AlwaysBelow),
        };
        let yn = |b: bool| Json::str(if b { "yes" } else { "no" });
        out.push(("landing_logs_night", yn(logs)));
        out.push(("landing_counts_currency", yn(counts)));
        out.push((
            "landing_state",
            Json::Num(if counts {
                1.0
            } else if logs {
                2.0
            } else {
                3.0
            }),
        ));
    }
    out.push((
        "part107_lighting",
        Json::str("Anti-collision lighting visible for at least 3 statute miles, with a flash rate enough to avoid a collision, is required during civil twilight (§107.29(b)) and at night (§107.29(a)(2)); night also needs the §107.65 training."),
    ));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- mapping window

/// A point of the day's sun path, as the diagram reads it.
const SUN_PATH_ROW: &[Field] = &[
    Field::new(
        "time",
        "Local time",
        "Clock time",
        Kind::Text { max_len: 5 },
    ),
    Field::new(
        "azimuth",
        "Azimuth",
        "Clockwise from north",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "elevation",
        "Elevation",
        "Above the horizon, negative below",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
];

pub static MAPPING_WINDOW: ToolDef = ToolDef {
    id: "time.sun.mapping-window",
    title: "Mapping light window",
    summary: "The part of a day when the sun is above an elevation threshold (default 30°) for aerial mapping, in local time and Zulu, with the day's highest sun.",
    aliases: &[
        "best time to fly a mapping mission",
        "sun angle window",
        "solar elevation window",
    ],
    keywords: &[
        "mapping",
        "photogrammetry",
        "sun angle",
        "shadows",
        "solar elevation",
        "window",
        "drone",
    ],
    inputs: &[
        LAT,
        LON,
        DATE,
        OFFSET,
        Field::new(
            "threshold",
            "Minimum sun elevation",
            "Default 30 deg (common vendor guidance)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-18,89]"),
    ],
    outputs: &[
        text("window", "Window", "When the sun is above the threshold"),
        text("window_start", "Start", "Local and Zulu").optional(),
        text("window_end", "End", "Local and Zulu").optional(),
        text("duration", "Duration", "Length of the window"),
        Field::new(
            "max_elevation",
            "Highest sun",
            "Apparent elevation at solar noon",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[-90,90]"),
        Field::new(
            "path",
            "Sun path",
            "Where the sun stands through the day, every 20 minutes",
            Kind::List {
                items: SUN_PATH_ROW,
                min: 0,
                max: 73,
            },
        ),
    ],
    errors: &[],
    stability: Stability::Stable,
    when_to_use: "Use this to plan an aerial mapping sortie: it says when the sun is above the elevation your workflow wants -- 30 degrees by default, which is common vendor guidance -- in local time and Zulu, with the day's highest sun and the whole arc it sits on.",
    limitations: "The threshold is compared against the geometric sun, where it actually is, because a shadow is cast by geometry; the elevation reported is the apparent one an observer sees, and the two differ by about 0.02 degrees at 30 degrees up. Times are good to about a minute. A high threshold in winter or at latitude gives no window at all, and a low one in polar summer gives all day; both are answers, not failures. And sun elevation is one input to image quality among several -- haze, cloud, wind and the surface itself are not modelled here.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Times the geometric sun (NREL SPA) crosses the threshold, by bisection between the day's highest and lowest points; the highest sun and the path are the same SPA sun, apparent (refracted), every 20 minutes of local clock time",
    accuracy: "About 1 minute",
    references: &[NREL_SPA, NOAA],
    examples: &[Example {
        id: "primary",
        title: "A Denver site on 2026-06-21 at 30°",
        input: r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-06-21","offset":"-06:00","threshold":"30 deg"}"#,
        source: "add-practitioner-essentials mapping-window scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.sun.position",
            reason: "next",
        },
        Related {
            id: "time.sun.hotspot",
            reason: "next",
        },
        Related {
            id: "time.sun.events",
            reason: "alternative",
        },
    ],
    sentence: "The sun is high enough {window}.",
    limits: &[("batchRows", 1_000)],
    run: run_mapping,
    ..ToolDef::BLANK
};

/// Apparent elevation and azimuth from the SPA, which is the sun the window
/// solver uses. `sun::position` is the NOAA series, good to about 0.01 deg;
/// reporting a peak from it beside times solved by the SPA made the tool
/// disagree with `time.sun.position` by up to 10 arcseconds at the same
/// instant, which is a disagreement a reader would have to explain.
fn spa_apparent(lat: f64, lon: f64, jd_utc: f64) -> (f64, f64) {
    let year = 2000.0 + (jd_utc - 2_451_545.0) / 365.25;
    let p = crate::spa::position(
        jd_utc,
        crate::spa::delta_t(year, 0.5),
        crate::spa::Observer {
            lat,
            lon,
            elevation: 0.0,
            pressure: 1013.25,
            temperature: 12.0,
            atmos_refract: 0.5667,
        },
    );
    (90.0 - p.zenith, p.azimuth.rem_euclid(360.0))
}

fn run_mapping(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let day = local_date(ctx)?;
    let off = crate::ZoneSpec::parse(ctx, "offset")?;
    let off = &off;
    let threshold = point::plain_angle(ctx, "threshold")?.unwrap_or(30.0);
    let noon = local_noon(lon, day, off);
    let top = sun::position(lat, lon, noon);
    let crossings = sun::crossings(lat, lon, noon, threshold);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Highest the sun gets",
            "the sun's elevation at solar noon, the best it will be all day",
            format!("at {}°, {}° on this date", n(lat, 6), n(lon, 6)),
            format!("{}°", n(top.elevation, 2)),
        );
        ctx.step(
            "Threshold",
            "the elevation you asked for, below which shadows grow long",
            format!(
                "{}° wanted against {}° available",
                n(threshold, 2),
                n(top.elevation, 2)
            ),
            format!("{}°", n(threshold, 2)),
        );
        ctx.step(
            "Window",
            "the time between the sun rising past that elevation and falling back",
            format!(
                "above {}° at {}°, {}°",
                n(threshold, 2),
                n(lat, 6),
                n(lon, 6)
            ),
            match crossings {
                Crossing::Times(a, b) => {
                    format!("from {} to {}", local_clock(a, off), local_clock(b, off))
                }
                Crossing::AlwaysAbove => "all day".to_owned(),
                Crossing::AlwaysBelow => "none".to_owned(),
            },
        );
    }
    let mut out = Vec::new();
    match crossings {
        Crossing::Times(a, b) => {
            out.push((
                "window",
                Json::str(format!(
                    "from {} to {}",
                    local_clock(a, off),
                    local_clock(b, off)
                )),
            ));
            out.push(("window_start", Json::str(event_text(a, off))));
            out.push(("window_end", Json::str(event_text(b, off))));
            out.push(("duration", Json::str(duration((b - a) * 1440.0))));
        }
        Crossing::AlwaysAbove => {
            out.push(("window", Json::str("all day")));
            out.push(("duration", Json::str(duration(1440.0))));
        }
        Crossing::AlwaysBelow => {
            out.push(("window", Json::str("at no time that day")));
            out.push(("duration", Json::str(duration(0.0))));
        }
    }
    out.push((
        "max_elevation",
        ctx.out("max_elevation", deg(spa_apparent(lat, lon, noon).0)),
    ));
    // The whole day's sun, every 20 minutes of local clock time, so the
    // window can be read against the path it sits on.
    let base = jd(day, -f64::from(off.minutes_at(day * 86_400 + 43_200)));
    let q = |v: f64| deg(v).to_json();
    let path: Vec<Json> = (0..73)
        .map(|k| {
            let t = base + f64::from(k) * 20.0 / 1440.0;
            let (el, az) = spa_apparent(lat, lon, t);
            let m = k * 20;
            Json::obj([
                ("time", Json::str(format!("{:02}:{:02}", m / 60, m % 60))),
                ("azimuth", q(az)),
                ("elevation", q(el)),
            ])
        })
        .collect();
    out.push(("path", Json::Arr(path)));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- hotspot

pub static HOTSPOT: ToolDef = ToolDef {
    id: "time.sun.hotspot",
    title: "Sun hotspot in a camera frame",
    summary: "Whether the hotspot, the bright patch where the camera looks straight away from the sun, falls in a camera's frame for its heading and pitch at a place and time, and how far off it is.",
    aliases: &[
        "sun hotspot calculator",
        "photogrammetry hotspot",
        "antisolar point",
        "drone hotspot check",
    ],
    keywords: &[
        "hotspot",
        "antisolar",
        "sun",
        "camera",
        "drone mapping",
        "glare",
        "BRDF",
    ],
    inputs: &[
        LAT,
        LON,
        text(
            "time",
            "Time",
            "With Z or an offset, like 2026-06-21T12:00-06:00",
        )
        .required()
        .core(),
        Field::new(
            "camera_heading",
            "Camera heading",
            "Where the camera points, from true north, like 90 deg; default 0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("unbounded"),
        Field::new(
            "camera_pitch",
            "Camera pitch",
            "From level, down negative, like -90 deg for straight down (the default)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "field_of_view",
            "Diagonal field of view",
            "Like 84 deg (the default, common for 1-inch drone cameras)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded"),
    ],
    outputs: &[
        Field::new(
            "hotspot_angle",
            "Hotspot off the look direction",
            "Angle from where the camera points to the antisolar point",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,180]"),
        Field::new(
            "in_frame",
            "Hotspot in frame",
            "yes when within half the diagonal field of view",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "sun_elevation",
            "Sun elevation",
            "Apparent, with refraction",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("[-90,90]"),
        Field::new(
            "sun_azimuth",
            "Sun azimuth",
            "From true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("[0,360)"),
        Field::new(
            "antisolar_azimuth",
            "Antisolar azimuth",
            "Opposite the sun",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("[0,360)"),
        Field::new(
            "antisolar_elevation",
            "Antisolar elevation",
            "Minus the sun's elevation",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(2))
        .angle_range("[-90,90]"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    stability: Stability::Stable,
    when_to_use: "Use this before an aerial survey to find out whether the hotspot -- the blown patch where the camera looks straight away from the sun -- will land in frame, and how far off it is. Check a planned time and camera attitude, or walk the day to find when the sun is high enough to keep it out.",
    limitations: "The frame is treated as a cone around the look direction, so the test is exact at the corners and slightly generous at the edges: a hotspot just outside the cone could still clip a corner. The hotspot is a direction, not a brightness -- how bad the patch actually looks depends on the surface, the sun angle and the camera, none of which are inputs here. And a nadir camera catches the hotspot whenever the sun is below about 48 degrees with the default field of view, which is most of a working day outside high summer; that is the geometry, not a fault in the flight plan.",
    warnings: &[
        "HOTSPOT_IN_FRAME",
        "SUN_BELOW_HORIZON",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
    ],
    model: "Sun azimuth and apparent elevation by the NREL SPA (standard atmosphere, sea level); the antisolar point is opposite the sun, at −elevation. The hotspot angle is the angle between the camera's look vector (heading, pitch) and the antisolar direction; it is in frame when within half the diagonal field of view",
    accuracy: "Sun direction to about 0.0003°; the frame test treats the field of view as a cone around the look direction, so a hotspot near a frame corner is borderline",
    references: &[NREL_SPA],
    examples: &[Example {
        id: "primary",
        title: "A nadir camera at noon near the June solstice in Denver",
        input: r#"{"lat":39.74,"lon":-104.99,"time":"2026-06-21T13:00-06:00","camera_pitch":"-90 deg"}"#,
        source: "NREL SPA sun position; with the sun near 73° up, the antisolar point is 17° off nadir, inside an 84° field of view",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.sun.mapping-window",
            reason: "alternative",
        },
        Related {
            id: "time.sun.position",
            reason: "parent",
        },
        Related {
            id: "time.sun.events",
            reason: "alternative",
        },
    ],
    sentence: "The hotspot is {hotspot_angle} from where the camera points: in frame, {in_frame}.",
    limits: &[("batchRows", 10_000)],
    run: run_hotspot,
    ..ToolDef::BLANK
};

fn run_hotspot(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let t = ctx.text("time")?.expect("required");
    let st = civil::parse_stamp(&t).map_err(|m| ToolError::invalid("/time", m))?;
    if st.offset.is_none() {
        return Err(ToolError::invalid(
            "/time",
            "Add Z for UTC or the UTC offset, like 2026-06-21T12:00-06:00.",
        ));
    }
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let heading = ctx.quantity("camera_heading")?.map_or(0.0, |q| q.to(deg));
    let pitch = ctx.quantity("camera_pitch")?.map_or(-90.0, |q| q.to(deg));
    let fov = ctx.quantity("field_of_view")?.map_or(84.0, |q| q.to(deg));
    if !(-90.0..=90.0).contains(&pitch) {
        return Err(ToolError::invalid(
            "/camera_pitch",
            "Pitch is between -90° (straight down) and 90° (straight up).",
        ));
    }
    if !(fov > 0.0 && fov < 180.0) {
        return Err(ToolError::invalid(
            "/field_of_view",
            "The field of view is between 0° and 180°.",
        ));
    }
    let (d, s) = crate::to_utc(st);
    let jd_utc = jd(d, s.min(86_400.0) / 60.0);
    let (y, mo, _) = civil::civil_from_days(d);
    let p = crate::spa::position(
        jd_utc,
        crate::spa::delta_t(y as f64, f64::from(mo)),
        crate::spa::Observer {
            lat,
            lon,
            elevation: 0.0,
            pressure: 1013.25,
            temperature: 12.0,
            atmos_refract: 0.5667,
        },
    );
    let el = 90.0 - p.zenith;
    let az = p.azimuth.rem_euclid(360.0);
    let (anti_az, anti_el) = ((az + 180.0).rem_euclid(360.0), -el);
    let v = |a: f64, e: f64| {
        let (a, e) = (a.to_radians(), e.to_radians());
        [a.sin() * e.cos(), a.cos() * e.cos(), e.sin()]
    };
    let (look, anti) = (v(heading, pitch), v(anti_az, anti_el));
    let c = (look[0] * anti[0] + look[1] * anti[1] + look[2] * anti[2]).clamp(-1.0, 1.0);
    let angle = libm::acos(c).to_degrees();
    let visible = el > 0.0;
    let inside = visible && angle <= fov / 2.0;
    let fmt = ctx.options.format;
    if !visible {
        ctx.warnings.push(Warning::new(
            "SUN_BELOW_HORIZON",
            "The sun is below the horizon, so there is no hotspot.",
        ));
    } else if inside {
        ctx.warnings.push(Warning::new(
            "HOTSPOT_IN_FRAME",
            format!(
                "The hotspot is {}° from the look direction, inside the {}° field of view: expect a bright patch where the drone's own shadow falls, which can upset image matching. Fly earlier or later, or tilt the camera.",
                display::number(angle, Precision::Decimals(1), fmt),
                display::number(fov, Precision::Decimals(0), fmt)
            ),
        ));
    }
    if ctx.explaining() {
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Antisolar point",
            "opposite the sun: azimuth + 180°, elevation × −1",
            format!("sun at {}°, {}°", n(az, 2), n(el, 2)),
            format!("{}°, {}°", n(anti_az, 2), n(anti_el, 2)),
        );
        ctx.step(
            "Angle from the look direction",
            "acos(look · antisolar)",
            format!("camera {}°, {}°", n(heading, 1), n(pitch, 1)),
            display::quantity(angle, "deg", Precision::Decimals(1), fmt),
        );
    }
    let q = |v: f64| Q {
        value: v,
        unit: deg,
    };
    Ok(Json::obj(vec![
        ("hotspot_angle", ctx.out("hotspot_angle", q(angle))),
        (
            "in_frame",
            Json::str(if !visible {
                "no, the sun is down"
            } else if inside {
                "yes"
            } else {
                "no"
            }),
        ),
        ("sun_elevation", ctx.out("sun_elevation", q(el))),
        ("sun_azimuth", ctx.out("sun_azimuth", q(az))),
        (
            "antisolar_azimuth",
            ctx.out("antisolar_azimuth", q(anti_az)),
        ),
        (
            "antisolar_elevation",
            ctx.out("antisolar_elevation", q(anti_el)),
        ),
    ]))
}
