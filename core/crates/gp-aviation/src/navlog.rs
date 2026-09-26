//! The navigation log (add-flight-and-drone-planning-tools, flight-planning
//! "Navigation log"): every leg of a route at once, from waypoints or from
//! course and distance rows, with the wind triangle, variation, deviation,
//! time, and fuel per leg and in total. Nothing here is a new formula: the
//! course and distance are the Karney geodesic, the heading and groundspeed
//! are `aviation.wind.heading-groundspeed`'s own function, the variation is
//! WMM2025 when a date is given, and the deviation is read from the card the
//! way `aviation.wind.heading-chain` reads it.

use crate::heading::{CARD_ROW, card_deviation, read_card};
use crate::refs::*;
use crate::{obj, unit, wind_triangle};
use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::wrap_azimuth;
use gp_base::display;
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::magnetic::{self as mag, Model};
use serde_json::{Map, Value};

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const fn out(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
    d: u8,
) -> Field {
    qty(name, title, help, q, u).precision(Precision::Decimals(d))
}

pub(crate) const PHAK_FLIGHT_LOG: Reference = Reference {
    locator: "Chapter 16 (Navigation), Flight Planning: Charting the Course and Figure 16-26, the pilot's planning sheet and visual flight log (TC ± WCA = TH ± V = MH ± D = CH)",
    ..PHAK
};

/// The per-leg wind and variation columns both row forms share.
const WIND_COLS: [Field; 3] = [
    qty(
        "wind_direction",
        "Wind direction",
        "For this leg, true, like 360 deg",
        QT::Angle,
        "deg",
    ),
    qty(
        "wind_speed",
        "Wind speed",
        "For this leg, like 10 kt",
        QT::Speed,
        "kt",
    ),
    qty(
        "variation",
        "Variation",
        "For this leg, east positive, like 7 for 7° E",
        QT::Angle,
        "deg",
    ),
];

const WAYPOINT: &[Field] = &[
    Field::new(
        "name",
        "Name",
        "Optional, like KCHK",
        Kind::Text { max_len: 24 },
    ),
    qty("lat", "Latitude", "Like 35.097", QT::Angle, "deg").required(),
    qty("lon", "Longitude", "Like -97.967", QT::Angle, "deg").required(),
    WIND_COLS[0],
    WIND_COLS[1],
    WIND_COLS[2],
];

const LEG_IN: &[Field] = &[
    Field::new(
        "name",
        "To",
        "The checkpoint at the leg's end, like Checkpoint 1",
        Kind::Text { max_len: 24 },
    ),
    qty(
        "course",
        "True course",
        "Measured on the chart, like 031 deg",
        QT::Angle,
        "deg",
    )
    .required(),
    qty("distance", "Distance", "Like 11 NM", QT::Distance, "NM").required(),
    WIND_COLS[0],
    WIND_COLS[1],
    WIND_COLS[2],
];

const LEG_OUT: &[Field] = &[
    Field::new(
        "leg",
        "Leg",
        "In route order",
        Kind::Number {
            min: 1.0,
            max: 99.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new("from", "From", "Waypoint", Kind::Text { max_len: 24 }),
    Field::new("to", "To", "Waypoint", Kind::Text { max_len: 24 }),
    out(
        "true_course",
        "True course",
        "Geodesic course leaving the waypoint, or as given",
        QT::Angle,
        "deg",
        0,
    )
    .angle_range("[0,360)"),
    out("distance", "Distance", "This leg", QT::Distance, "NM", 1),
    out(
        "wind_correction_angle",
        "Wind correction angle",
        "Negative is a correction to the left",
        QT::Angle,
        "deg",
        0,
    )
    .angle_range("unbounded")
    .optional(),
    out(
        "true_heading",
        "True heading",
        "True course plus the wind correction",
        QT::Angle,
        "deg",
        0,
    )
    .angle_range("[0,360)")
    .optional(),
    out(
        "variation",
        "Variation",
        "East positive, as given or from WMM2025",
        QT::Angle,
        "deg",
        1,
    )
    .angle_range("unbounded")
    .optional(),
    out(
        "magnetic_heading",
        "Magnetic heading",
        "True heading minus east variation",
        QT::Angle,
        "deg",
        0,
    )
    .angle_range("[0,360)")
    .optional(),
    out(
        "compass_heading",
        "Compass heading",
        "Magnetic heading minus the card's deviation",
        QT::Angle,
        "deg",
        0,
    )
    .angle_range("[0,360)")
    .optional(),
    out(
        "groundspeed",
        "Groundspeed",
        "From the wind triangle",
        QT::Speed,
        "kt",
        0,
    )
    .optional(),
    out(
        "time",
        "Time en route",
        "Distance ÷ groundspeed",
        QT::Time,
        "min",
        0,
    )
    .optional(),
    out("fuel", "Fuel", "Time × fuel burn", QT::Volume, "galUS", 1).optional(),
    out(
        "cumulative_distance",
        "Distance so far",
        "From the first waypoint",
        QT::Distance,
        "NM",
        1,
    ),
    out(
        "cumulative_time",
        "Time so far",
        "From the first waypoint",
        QT::Time,
        "min",
        0,
    )
    .optional(),
    out(
        "cumulative_fuel",
        "Fuel so far",
        "From the first waypoint",
        QT::Volume,
        "galUS",
        1,
    )
    .optional(),
];

const PATH_ROW: &[Field] = &[
    out("lat", "Latitude", "Degrees", QT::Angle, "deg", 7),
    out("lon", "Longitude", "Degrees", QT::Angle, "deg", 7),
    Field::new("name", "Name", "Waypoint", Kind::Text { max_len: 24 }),
];

pub static NAV_LOG: ToolDef = ToolDef {
    id: "aviation.flight-plan.nav-log",
    title: "Navigation log",
    summary: "A VFR navigation log for a whole route: each leg’s true course, distance, wind correction, true, magnetic, and compass heading, groundspeed, time, and fuel, with the totals.",
    aliases: &[
        "navigation log",
        "VFR flight log",
        "pilot navigation log",
        "nav log with wind and fuel",
    ],
    keywords: &[
        "nav log",
        "flight log",
        "legs",
        "checkpoints",
        "wind correction",
        "magnetic heading",
        "compass heading",
        "groundspeed",
        "time en route",
        "fuel by leg",
        "cross-country",
    ],
    inputs: &[
        Field::new(
            "waypoints",
            "Waypoints",
            "In order: name (optional), latitude, longitude, and the leg’s wind if it differs, like KCHK, 35.097, -97.967",
            Kind::List {
                items: WAYPOINT,
                min: 2,
                max: 100,
            },
        )
        .core(),
        qty("tas", "True airspeed", "Like 115 kt", QT::Speed, "kt")
            .required()
            .core(),
        qty(
            "wind_direction",
            "Wind direction",
            "True, for every leg without its own, like 360 deg",
            QT::Angle,
            "deg",
        )
        .core()
        .angle_range("[0,360)"),
        qty(
            "wind_speed",
            "Wind speed",
            "For every leg without its own, like 10 kt",
            QT::Speed,
            "kt",
        )
        .core(),
        qty(
            "fuel_burn",
            "Fuel burn",
            "In cruise, like 8 gal/h",
            QT::VolumeFlow,
            "galUS/h",
        )
        .core(),
        Field::new(
            "legs",
            "Legs by course and distance",
            "Instead of waypoints, from the chart: true course and distance per leg, like 031 deg, 11 NM",
            Kind::List {
                items: LEG_IN,
                min: 1,
                max: 99,
            },
        ),
        qty(
            "variation",
            "Magnetic variation",
            "For every leg without its own, east positive, like 7 for 7° E",
            QT::Angle,
            "deg",
        ),
        Field::new(
            "date",
            "Date for WMM variation",
            "With waypoints, when no variation is given, like 2026-09-25",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "deviation_card",
            "Deviation card",
            "Magnetic heading and deviation (east positive), one per line, like 030, -2",
            Kind::List {
                items: CARD_ROW,
                min: 0,
                max: 36,
            },
        ),
    ],
    outputs: &[
        out(
            "total_time",
            "Total time",
            "Every leg’s time; left out when a leg cannot be flown",
            QT::Time,
            "min",
            0,
        )
        .optional(),
        out(
            "total_distance",
            "Total distance",
            "All legs",
            QT::Distance,
            "NM",
            1,
        ),
        out(
            "total_fuel",
            "Total fuel",
            "Every leg’s fuel, without taxi, climb, or reserve",
            QT::Volume,
            "galUS",
            1,
        )
        .optional(),
        Field::new(
            "legs_count",
            "Legs",
            "Number of legs",
            Kind::Number {
                min: 1.0,
                max: 99.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "legs",
            "Legs",
            "One row per leg",
            Kind::List {
                items: LEG_OUT,
                min: 1,
                max: 99,
            },
        ),
        Field::new(
            "path",
            "Route",
            "The waypoints in order, for drawing",
            Kind::List {
                items: PATH_ROW,
                min: 2,
                max: 100,
            },
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "WIND_EXCEEDS_TAS",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    when_to_use: "Use this to fill in a navigation log for a cross-country flight: give the waypoints (or the course and distance of each leg from the chart), your true airspeed, the winds aloft, and the fuel burn, and it works every leg the way the planning sheet does, from true course to compass heading, with the time and fuel for each leg and the totals.",
    limitations: "Each leg is flown at cruise true airspeed in a steady wind, so the climb, the descent, and any wind change inside a leg are not in it: add the climb from the climb plan and plan the fuel reserve with the fuel planner. The winds are true, as forecast. The course from waypoints is the geodesic’s course leaving each waypoint, which drifts along a long leg. Planning aid, not for primary navigation.",
    model: "Course and distance: Karney geodesic inverse on WGS 84 (or the chart values given). Heading and groundspeed: the wind triangle of aviation.wind.heading-groundspeed, WCA = asin(W/TAS · sin(WD − TC)), GS = TAS·cos(WCA) − W·cos(WD − TC). MH = TH − variation (east positive), from the value given or WMM2025 at the leg’s start; CH = MH − deviation, read from the card by linear interpolation. Time = distance ÷ GS; fuel = time × burn",
    accuracy: "Exact arithmetic for steady winds; the geodesic to nanometers; WMM2025 variation about ±0.5° typical. Planning aid, not certified for navigation.",
    references: &[PHAK_FLIGHT_LOG, gp_geo::magnetic::WMM_REPORT],
    examples: &[Example {
        id: "primary",
        title: "Chickasha to Wiley Post to Guthrie at 115 kt, wind 360° at 10 kt",
        input: r#"{"waypoints":[{"name":"KCHK","lat":35.0967,"lon":-97.9678},{"name":"KPWA","lat":35.5342,"lon":-97.6471},{"name":"KGOK","lat":35.8498,"lon":-97.4156}],"tas":"115 kt","wind_direction":"360 deg","wind_speed":"10 kt","variation":"7 deg","fuel_burn":"8 gal/h"}"#,
        source: "The PHAK chapter 16 trip's numbers (TAS 115 kt, wind 360° at 10 kt, 7° E variation, 8 GPH) over approximate positions of Chickasha, Wiley Post (its checkpoint 3), and Guthrie",
    }],
    assets: &["wmm2025"],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("path", "path"), ("distance", "total_distance")],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "parent",
        },
        Related {
            id: "aviation.performance.climb-plan",
            reason: "next",
        },
        Related {
            id: "aviation.loading.fuel-plan",
            reason: "next",
        },
        Related {
            id: "navigation.route.legs",
            reason: "alternative",
        },
    ],
    sentence: "The route is {total_distance} in {legs_count} {plural legs_count \"leg\" \"legs\"}{if total_time > 0}, {total_time} in all{/if}{if total_fuel > 0}, burning {total_fuel}{/if}.{warn WIND_EXCEEDS_TAS} One leg cannot be flown in this wind, so there are no totals.{/warn}",
    limits: &[("batchRows", 1_000)],
    run: run_nav_log,
    ..ToolDef::BLANK
};

/// A wind read from a row or the top level, as heading-groundspeed reads one:
/// the direction wrapped to [0, 360) in degrees, the speed in knots.
fn wind_of(dir: Option<Q>, speed: Option<Q>, at: &str) -> Result<Option<(f64, f64)>, ToolError> {
    let Some(speed) = speed else {
        if dir.is_some() {
            return Err(ToolError::invalid(
                &format!("{at}wind_speed"),
                "Give the wind speed with its direction, like 10 kt.",
            ));
        }
        return Ok(None);
    };
    let s = speed.to(unit(QT::Speed, "kt"));
    if s < 0.0 {
        return Err(ToolError::invalid(
            &format!("{at}wind_speed"),
            "The wind speed cannot be negative.",
        ));
    }
    let d = dir.map(|d| wrap_azimuth(d.to(unit(QT::Angle, "deg"))));
    match d {
        Some(d) => Ok(Some((d, s))),
        None if s == 0.0 => Ok(Some((0.0, 0.0))),
        None => Err(ToolError::invalid(
            &format!("{at}wind_direction"),
            "Give the wind direction (where it blows from), like 360 deg.",
        )),
    }
}

/// One leg before the triangle: names, true course, distance in NM, the
/// start position (for WMM), and the leg's own wind and variation.
/// A wind as (direction from, degrees true; speed, knots).
type Wind = (f64, f64);
/// A named waypoint: (name, latitude, longitude).
type Point = (String, f64, f64);

struct Leg {
    from: String,
    to: String,
    course: f64,
    nm: f64,
    start: Option<(f64, f64)>,
    wind: Option<(f64, f64)>,
    variation: Option<f64>,
}

fn row_wind_var(
    ctx: &mut Ctx,
    list: &str,
    i: usize,
    r: &Map<String, Value>,
) -> Result<(Option<Wind>, Option<f64>), ToolError> {
    let dir = ctx.row_quantity(list, i, r, "wind_direction")?;
    let speed = ctx.row_quantity(list, i, r, "wind_speed")?;
    let wind = wind_of(dir, speed, &format!("/{list}/{i}/"))?;
    let var = ctx
        .row_quantity(list, i, r, "variation")?
        .map(|v| v.to(unit(QT::Angle, "deg")));
    Ok((wind, var))
}

fn check_variation(v: f64, at: &str) -> Result<f64, ToolError> {
    if v.abs() > 180.0 {
        return Err(ToolError::invalid(
            at,
            "A variation is within ±180°, east positive.",
        ));
    }
    Ok(v)
}

fn read_legs(ctx: &mut Ctx) -> Result<(Vec<Leg>, Option<Vec<Point>>), ToolError> {
    let dg = unit(QT::Angle, "deg");
    let nm = unit(QT::Distance, "NM");
    match (ctx.is_set("waypoints"), ctx.is_set("legs")) {
        (true, true) => Err(ToolError::invalid(
            "/legs",
            "Give waypoints or legs by course and distance, not both.",
        )),
        (false, false) => Err(ToolError::invalid(
            "/waypoints",
            "Give the route: waypoints with latitude and longitude, or legs by true course and distance.",
        )
        .hint("Like KCHK, 35.097, -97.967 then KGOK, 35.850, -97.416")),
        (true, false) => {
            let rows = ctx.rows("waypoints")?;
            let mut pts = Vec::with_capacity(rows.len());
            let mut own = Vec::with_capacity(rows.len());
            for (i, r) in rows.iter().enumerate() {
                let lat = ctx
                    .row_quantity("waypoints", i, r, "lat")?
                    .expect("required")
                    .to(dg);
                let lon = ctx
                    .row_quantity("waypoints", i, r, "lon")?
                    .expect("required")
                    .to(dg);
                if !(-90.0..=90.0).contains(&lat) {
                    return Err(ToolError::new(
                        ErrorCode::OutOfDomain,
                        "Latitude must be between -90° and 90°.",
                    )
                    .at(&format!("/waypoints/{i}/lat")));
                }
                let name = ctx
                    .row_text("waypoints", i, r, "name")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("WP{}", i + 1));
                let (wind, var) = row_wind_var(ctx, "waypoints", i, r)?;
                if i + 1 == rows.len() && (wind.is_some() || var.is_some()) {
                    return Err(ToolError::invalid(
                        &format!("/waypoints/{i}"),
                        "A waypoint’s wind and variation are for the leg that leaves it; the last waypoint starts no leg.",
                    ));
                }
                pts.push((name, lat, lon));
                own.push((wind, var));
            }
            let g = Geodesic::wgs84();
            let mut legs = Vec::with_capacity(pts.len() - 1);
            for (k, w) in pts.windows(2).enumerate() {
                let ((n1, la1, lo1), (n2, la2, lo2)) = (&w[0], &w[1]);
                let (s, a1, _, _): (f64, f64, f64, f64) = g.inverse(*la1, *lo1, *la2, *lo2);
                if s < 1e-3 {
                    return Err(ToolError::invalid(
                        &format!("/waypoints/{}", k + 1),
                        format!(
                            "Waypoints {} and {} are the same place, so that leg has no course.",
                            k + 1,
                            k + 2
                        ),
                    ));
                }
                legs.push(Leg {
                    from: n1.clone(),
                    to: n2.clone(),
                    course: wrap_azimuth(a1),
                    nm: Q {
                        value: s,
                        unit: unit(QT::Distance, "m"),
                    }
                    .to(nm),
                    start: Some((*la1, *lo1)),
                    wind: own[k].0,
                    variation: own[k].1,
                });
            }
            Ok((legs, Some(pts)))
        }
        (false, true) => {
            let rows = ctx.rows("legs")?;
            let mut legs = Vec::with_capacity(rows.len());
            for (i, r) in rows.iter().enumerate() {
                let course = wrap_azimuth(
                    ctx.row_quantity("legs", i, r, "course")?
                        .expect("required")
                        .to(dg),
                );
                let d = ctx
                    .row_quantity("legs", i, r, "distance")?
                    .expect("required")
                    .to(nm);
                if d.is_nan() || d <= 0.0 {
                    return Err(ToolError::invalid(
                        &format!("/legs/{i}/distance"),
                        "Each leg needs a distance above zero, like 11 NM.",
                    ));
                }
                let to = ctx
                    .row_text("legs", i, r, "name")?
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("WP{}", i + 2));
                let (wind, var) = row_wind_var(ctx, "legs", i, r)?;
                let from = legs
                    .last()
                    .map_or_else(|| "WP1".to_owned(), |l: &Leg| l.to.clone());
                legs.push(Leg {
                    from,
                    to,
                    course,
                    nm: d,
                    start: None,
                    wind,
                    variation: var,
                });
            }
            Ok((legs, None))
        }
    }
}

fn run_nav_log(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let kt = unit(QT::Speed, "kt");
    let (legs, pts) = read_legs(ctx)?;
    let tas = ctx.req_quantity("tas")?.to(kt);
    if tas.is_nan() || tas <= 0.0 {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be greater than zero.",
        ));
    }
    let dir = ctx.quantity("wind_direction")?;
    let speed = ctx.quantity("wind_speed")?;
    let all_wind = wind_of(dir, speed, "/")?;
    let all_var = match ctx.quantity("variation")? {
        Some(v) => Some(check_variation(v.to(dg), "/variation")?),
        None => None,
    };
    let burn = match ctx.quantity("fuel_burn")? {
        Some(b) => {
            let b = b.to(unit(QT::VolumeFlow, "galUS/h"));
            if b.is_nan() || b <= 0.0 {
                return Err(ToolError::invalid(
                    "/fuel_burn",
                    "The fuel burn must be above zero, like 8 gal/h.",
                ));
            }
            Some(b)
        }
        None => None,
    };
    // WMM2025 at each leg's start, only for legs with no variation of their own.
    let wmm = match ctx.text("date")? {
        None => None,
        Some(raw) => {
            if pts.is_none() {
                return Err(ToolError::invalid(
                    "/date",
                    "A date gives the variation from each waypoint’s position; with legs by course and distance, give the variation instead.",
                ));
            }
            let t = mag::parse_date(&raw).map_err(|m| ToolError::invalid("/date", m))?;
            let (lo, hi) = Model::Wmm2025.window();
            if !(lo..=hi).contains(&t) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    format!(
                        "WMM2025 is valid from 2025.0 to 2030.0; {} is outside it.",
                        raw.trim()
                    ),
                )
                .at("/date"));
            }
            Some(mag::coeffs_at(Model::Wmm2025, t))
        }
    };
    let card = read_card(ctx)?;
    let has_card = ctx.is_set("deviation_card");
    let q = |v: f64, qt: QT, s: &str| {
        Q {
            value: v,
            unit: unit(qt, s),
        }
        .to_json()
    };
    let (mut cum_nm, mut cum_min, mut cum_gal) = (0.0, 0.0, 0.0);
    let mut all_flown = true;
    let mut used_wmm = false;
    let mut rows = Vec::with_capacity(legs.len());
    let mut first: Option<(f64, f64, f64, f64, f64)> = None;
    let mut times = Vec::with_capacity(legs.len());
    for (i, leg) in legs.iter().enumerate() {
        let (wd, ws) = leg.wind.or(all_wind).unwrap_or((0.0, 0.0));
        let var = match leg.variation.or(all_var) {
            Some(v) => Some(check_variation(v, &format!("/legs/{i}/variation"))?),
            None => match (&wmm, leg.start) {
                (Some(c), Some((la, lo))) => {
                    used_wmm = true;
                    let (b, sv) = mag::field(c, la, lo, 0.0);
                    Some(mag::elements(b, sv).d)
                }
                _ => None,
            },
        };
        cum_nm += leg.nm;
        let mut row = vec![
            ("leg", Json::Num((i + 1) as f64)),
            ("from", Json::str(&leg.from)),
            ("to", Json::str(&leg.to)),
            ("true_course", q(leg.course, QT::Angle, "deg")),
            ("distance", q(leg.nm, QT::Distance, "NM")),
        ];
        let solved = wind_triangle(leg.course, tas, wd, ws).filter(|&(_, gs, _)| gs > 0.0);
        match solved {
            Some((th, gs, wca)) => {
                let minutes = leg.nm / gs * 60.0;
                times.push(minutes);
                row.push(("wind_correction_angle", q(wca, QT::Angle, "deg")));
                row.push(("true_heading", q(th, QT::Angle, "deg")));
                if let Some(v) = var {
                    let mh = wrap_azimuth(th - v);
                    row.push(("variation", q(v, QT::Angle, "deg")));
                    row.push(("magnetic_heading", q(mh, QT::Angle, "deg")));
                    if has_card {
                        let ch = wrap_azimuth(mh - card_deviation(&card, mh));
                        row.push(("compass_heading", q(ch, QT::Angle, "deg")));
                    }
                }
                row.push(("groundspeed", q(gs, QT::Speed, "kt")));
                row.push(("time", q(minutes, QT::Time, "min")));
                if let Some(b) = burn {
                    row.push(("fuel", q(minutes / 60.0 * b, QT::Volume, "galUS")));
                }
                if first.is_none() {
                    first = Some((leg.course, wca, th, gs, minutes));
                }
                if all_flown {
                    cum_min += minutes;
                    if let Some(b) = burn {
                        cum_gal += minutes / 60.0 * b;
                    }
                }
            }
            None => {
                all_flown = false;
                let fmt = ctx.options.format;
                ctx.warnings.push(
                    Warning::new(
                        "WIND_EXCEEDS_TAS",
                        format!(
                            "Leg {} ({} to {}): the wind ({} kt) is too strong for {} kt true airspeed on this course, so it has no heading, time, or fuel, and the route has no totals.",
                            i + 1,
                            leg.from,
                            leg.to,
                            display::number(ws, Precision::Decimals(0), fmt),
                            display::number(tas, Precision::Decimals(0), fmt)
                        ),
                    )
                    .at(&format!("/legs/{i}")),
                );
            }
        }
        row.push(("cumulative_distance", q(cum_nm, QT::Distance, "NM")));
        if all_flown {
            row.push(("cumulative_time", q(cum_min, QT::Time, "min")));
            if burn.is_some() {
                row.push(("cumulative_fuel", q(cum_gal, QT::Volume, "galUS")));
            }
        }
        rows.push(Json::obj(row));
    }
    if used_wmm {
        ctx.assets.push(AssetRef {
            id: Model::Wmm2025.id().into(),
            version: Model::Wmm2025.version().into(),
        });
    }
    let n = rows.len();
    let mut out = Vec::new();
    if all_flown {
        out.push((
            "total_time",
            ctx.out(
                "total_time",
                Q {
                    value: cum_min,
                    unit: unit(QT::Time, "min"),
                },
            ),
        ));
        if burn.is_some() {
            out.push((
                "total_fuel",
                ctx.out(
                    "total_fuel",
                    Q {
                        value: cum_gal,
                        unit: unit(QT::Volume, "galUS"),
                    },
                ),
            ));
        }
    }
    out.push((
        "total_distance",
        ctx.out(
            "total_distance",
            Q {
                value: cum_nm,
                unit: unit(QT::Distance, "NM"),
            },
        ),
    ));
    out.push(("legs_count", Json::Num(n as f64)));
    out.push(("legs", Json::Arr(rows)));
    if let Some(pts) = &pts {
        out.push((
            "path",
            Json::Arr(
                pts.iter()
                    .map(|(name, la, lo)| {
                        Json::obj([
                            ("lat", q(*la, QT::Angle, "deg")),
                            ("lon", q(*lo, QT::Angle, "deg")),
                            ("name", Json::str(name)),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    if ctx.explaining()
        && all_flown
        && let Some((tc, wca, th, gs, minutes)) = first
    {
        let fmt = ctx.options.format;
        let nf = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Leg 1 heading",
            "TH = TC + WCA, WCA = asin(W/TAS · sin(WD − TC))",
            format!(
                "{}° {} {}°",
                nf(tc, 0),
                if wca < 0.0 { "−" } else { "+" },
                nf(wca.abs(), 1)
            ),
            format!("{}°", nf(th, 0)),
        );
        ctx.step(
            "Leg 1 time",
            "time = distance ÷ groundspeed",
            format!("{} NM ÷ {} kt × 60", nf(legs[0].nm, 1), nf(gs, 1)),
            display::quantity(minutes, "min", Precision::Decimals(0), fmt),
        );
        ctx.step(
            "Total time",
            "Σ leg times",
            times
                .iter()
                .map(|&t| nf(t, 1))
                .collect::<Vec<_>>()
                .join(" + "),
            display::quantity(cum_min, "min", Precision::Decimals(0), fmt),
        );
    }
    Ok(obj(out))
}
