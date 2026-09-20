//! Sun tools (solar-and-twilight spec): position, rise/set/twilight with
//! explicit polar states, the four US aviation "nights", and the mapping
//! window. Events are tied to the requested local date and shown in local
//! time and Zulu, each dated.

use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
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
    url: "https://openlibrary.org/search?q=astronomical+algorithms+meeus",
};
const CFR_1_1_NIGHT: Reference = Reference {
    title: "14 CFR 1.1, General definitions (night)",
    issuer: "Federal Aviation Administration",
    year: 2024,
    edition: "Reviewed 2026-09-18",
    locator: "\"Night\": end of evening civil twilight to beginning of morning civil twilight, as published in the Air Almanac",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-A/part-1/section-1.1",
};
const CFR_61_57: Reference = Reference {
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
fn jd(days: i64, minutes: f64) -> f64 {
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
fn local_noon(lon: f64, day: i64, zone: &ZoneSpec) -> f64 {
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
            "Above the surrounding terrain or sea, for horizon dip (optional)",
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
        "night currency calculator",
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
    ],
    errors: &[],
    warnings: &[
        "CIVIL_TWILIGHT_APPROXIMATED",
        "INPUT_NORMALIZED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Sunrise and sunset (−0.833°) and civil twilight (−6°) from the NREL SPA geometric sun, for the evening of the date and the next morning",
    accuracy: "Within 1 minute of the windows built from USNO times at 100 places (491 windows); the Air Almanac, the legal source for twilight, tabulates to the minute",
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
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- mapping window

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
    ],
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Times the geometric sun (NREL SPA) crosses the threshold, by bisection between the day's highest and lowest points",
    accuracy: "About 1 minute",
    references: &[NOAA],
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
    related: &[Related {
        id: "time.sun.position",
        reason: "next",
    }],
    sentence: "The sun is high enough {window}.",
    limits: &[("batchRows", 1_000)],
    run: run_mapping,
    ..ToolDef::BLANK
};

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
        ctx.out("max_elevation", deg(top.elevation + top.refraction)),
    ));
    Ok(Json::obj(out))
}
