//! Climb planning and the go-on-or-turn-back points (add-flight-and-drone-
//! planning-tools, flight-planning "Climb plan" and "Equal time point and
//! point of no return"). Both lean on the wind triangle that
//! `aviation.wind.heading-groundspeed` solves, so a groundspeed here is that
//! tool's groundspeed.

use crate::refs::*;
use crate::{obj, unit, wind_triangle};
use gp_base::ErrorCode;
use gp_base::angle::wrap_azimuth;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

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

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: unit(qt, s),
    }
}

/// A wind as heading-groundspeed reads one: direction wrapped to [0, 360)
/// degrees, speed in knots; calm when neither is given.
fn read_wind(ctx: &mut Ctx) -> Result<Option<(f64, f64)>, ToolError> {
    let dir = ctx.quantity("wind_direction")?;
    let speed = ctx.quantity("wind_speed")?;
    let Some(speed) = speed else {
        if dir.is_some() {
            return Err(ToolError::invalid(
                "/wind_speed",
                "Give the wind speed with its direction, like 20 kt.",
            ));
        }
        return Ok(None);
    };
    let s = speed.to(unit(QT::Speed, "kt"));
    if s.is_nan() || s < 0.0 {
        return Err(ToolError::invalid(
            "/wind_speed",
            "The wind speed cannot be negative.",
        ));
    }
    match dir.map(|d| wrap_azimuth(d.to(unit(QT::Angle, "deg")))) {
        Some(d) => Ok(Some((d, s))),
        None if s == 0.0 => Ok(None),
        None => Err(ToolError::invalid(
            "/wind_direction",
            "Give the wind direction (where it blows from), like 090 deg.",
        )),
    }
}

/// The groundspeed on `course` at `tas` in `wind`, by the wind triangle.
fn groundspeed(course: f64, tas: f64, wind: (f64, f64), field: &str) -> Result<f64, ToolError> {
    match wind_triangle(course, tas, wind.0, wind.1) {
        Some((_, gs, _)) if gs > 0.0 => Ok(gs),
        Some(_) => Err(ToolError::new(
            ErrorCode::NoSolution,
            "The headwind is at least the airspeed, so the aircraft makes no progress.",
        )
        .at(field)),
        None => Err(ToolError::new(
            ErrorCode::NoSolution,
            "The crosswind is stronger than the true airspeed, so this course cannot be held.",
        )
        .at(field)),
    }
}

// ---------------------------------------------------------------- climb plan

const PHAK_CLIMB: Reference = Reference {
    locator: "Chapter 11 (Aircraft Performance), climb performance charts: Figure 11-25 (fuel, time, and distance to climb chart, read at the field and at cruise and subtracted) and Figure 11-26",
    ..PHAK
};

const POH_ROW: &[Field] = &[
    qty(
        "time",
        "Time",
        "From the POH table, like 6 min",
        QT::Time,
        "min",
    )
    .required(),
    qty(
        "fuel",
        "Fuel",
        "From the POH table, like 3.5 gal",
        QT::Volume,
        "galUS",
    )
    .required(),
    qty(
        "distance",
        "Distance",
        "From the POH table, like 9 NM",
        QT::Distance,
        "NM",
    )
    .required(),
];

pub static CLIMB_PLAN: ToolDef = ToolDef {
    id: "aviation.performance.climb-plan",
    title: "Climb to cruise: time, fuel, and distance",
    summary: "How long, how far, and how much fuel it takes to climb from the field to cruise altitude, and where along the first leg the top of climb falls, from a rate of climb or from the POH table.",
    aliases: &[
        "top of climb",
        "time fuel distance to climb",
        "climb planning",
        "TOC calculator",
    ],
    keywords: &[
        "climb",
        "top of climb",
        "TOC",
        "rate of climb",
        "fuel to climb",
        "time to climb",
        "distance to climb",
        "POH climb table",
    ],
    inputs: &[
        qty(
            "field_elevation",
            "Field elevation",
            "Or pressure altitude, like 1085 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "cruise_altitude",
            "Cruise altitude",
            "Like 5500 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "climb_rate",
            "Average rate of climb",
            "Like 500 fpm",
            QT::VerticalSpeed,
            "ft/min",
        )
        .core(),
        qty(
            "climb_tas",
            "Climb true airspeed",
            "Like 90 kt",
            QT::Speed,
            "kt",
        )
        .core(),
        qty(
            "climb_burn",
            "Fuel burn in the climb",
            "Like 11 gal/h",
            QT::VolumeFlow,
            "galUS/h",
        )
        .core(),
        qty(
            "course",
            "Course",
            "True, for the wind in the climb, like 031 deg",
            QT::Angle,
            "deg",
        )
        .angle_range("[0,360)"),
        qty(
            "wind_direction",
            "Wind direction",
            "True, the average in the climb, like 360 deg",
            QT::Angle,
            "deg",
        )
        .angle_range("[0,360)"),
        qty(
            "wind_speed",
            "Wind speed",
            "The average in the climb, like 10 kt",
            QT::Speed,
            "kt",
        ),
        qty(
            "taxi_fuel",
            "Start, taxi, and takeoff fuel",
            "Optional, like 1.1 gal",
            QT::Volume,
            "galUS",
        ),
        Field::new(
            "poh_table",
            "POH table readings",
            "Instead of a rate: the table’s time, fuel, and distance at the field, then at cruise, like 6 min, 3.5 gal, 9 NM",
            Kind::List {
                items: POH_ROW,
                min: 2,
                max: 2,
            },
        ),
    ],
    outputs: &[
        out(
            "top_of_climb",
            "Top of climb",
            "Ground distance from departure along the first leg",
            QT::Distance,
            "NM",
            1,
        ),
        out(
            "time",
            "Time to climb",
            "From the field to cruise",
            QT::Time,
            "min",
            0,
        ),
        out(
            "fuel",
            "Fuel to climb",
            "Burn × time, or the table’s difference",
            QT::Volume,
            "galUS",
            1,
        )
        .optional(),
        out(
            "total_fuel",
            "Fuel with taxi",
            "Start, taxi, and takeoff plus the climb",
            QT::Volume,
            "galUS",
            1,
        )
        .optional(),
        out(
            "groundspeed",
            "Groundspeed in the climb",
            "Climb TAS through the wind triangle",
            QT::Speed,
            "kt",
            0,
        )
        .optional(),
        out(
            "height",
            "Height to climb",
            "Cruise altitude − field elevation",
            QT::Length,
            "ft",
            0,
        ),
        Field::new(
            "note",
            "Note",
            "When no climb is needed",
            Kind::Text { max_len: 120 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    when_to_use: "Use this before a cross-country flight to plan the climb: from your average rate of climb, climb speed, and the wind, or from the POH’s time, fuel, and distance to climb table, it gives how long the climb takes, how much fuel it burns, and how far along the first leg you reach cruise altitude.",
    limitations: "It uses one average rate of climb and one wind for the whole climb, which a real climb does not hold: the rate falls as you climb. A POH table’s distance is for zero wind and standard temperature, and this tool does not apply the table’s temperature notes or a wind to it, so correct it as the POH instructs. It is not the aircraft’s certified climb performance.",
    model: "Rate form: time = (cruise − field) ÷ rate of climb; groundspeed from the climb TAS and wind by the wind triangle of aviation.wind.heading-groundspeed; distance = groundspeed × time; fuel = burn × time. Table form: time, fuel, and distance = the POH values at cruise − the values at the field, as the POH instructs",
    accuracy: "Exact arithmetic on your averages or table readings; as good as they are. Planning aid, not certified performance data.",
    references: &[PHAK_CLIMB],
    examples: &[Example {
        id: "primary",
        title: "From 1,085 ft to 5,500 ft at 500 fpm and 90 kt, wind 360° at 10 kt on course 031°",
        input: r#"{"field_elevation":"1085 ft","cruise_altitude":"5500 ft","climb_rate":"500 fpm","climb_tas":"90 kt","climb_burn":"11 gal/h","course":"031 deg","wind_direction":"360 deg","wind_speed":"10 kt"}"#,
        source: "A hand-worked climb on the PHAK chapter 16 trip’s course and wind: 4,415 ft at 500 fpm is 8.8 min",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "top_of_climb")],
    }],
    related: &[
        Related {
            id: "aviation.flight-plan.nav-log",
            reason: "next",
        },
        Related {
            id: "aviation.performance.climb-gradient",
            reason: "alternative",
        },
        Related {
            id: "aviation.loading.table-interpolate",
            reason: "parent",
        },
    ],
    sentence: "{if height > 0}Top of climb is {top_of_climb} from departure, {time} after takeoff{if fuel > 0}, using {fuel}{/if}.{else}No climb is needed: the field is at the cruise altitude.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_climb_plan,
    ..ToolDef::BLANK
};

fn run_climb_plan(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ft = unit(QT::Length, "ft");
    let (gal, min, nm) = (
        unit(QT::Volume, "galUS"),
        unit(QT::Time, "min"),
        unit(QT::Distance, "NM"),
    );
    let field = ctx.req_quantity("field_elevation")?.to(ft);
    let cruise = ctx.req_quantity("cruise_altitude")?.to(ft);
    let height = cruise - field;
    if height.is_nan() || height < 0.0 {
        return Err(ToolError::invalid(
            "/cruise_altitude",
            "The cruise altitude is below the field: that is a descent, not a climb.",
        )
        .hint("Plan a descent with the top-of-descent tool."));
    }
    let taxi = match ctx.quantity("taxi_fuel")? {
        Some(t) if t.to(gal) >= 0.0 => Some(t.to(gal)),
        Some(_) => {
            return Err(ToolError::invalid(
                "/taxi_fuel",
                "Taxi fuel cannot be negative.",
            ));
        }
        None => None,
    };
    let rate_inputs = [
        "climb_rate",
        "climb_tas",
        "climb_burn",
        "course",
        "wind_direction",
        "wind_speed",
    ];
    let fmt = ctx.options.format;
    let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
    // (time min, distance NM, fuel gal if known, groundspeed kt if known)
    let (time, dist, fuel, gs) = if ctx.is_set("poh_table") {
        if let Some(f) = rate_inputs.iter().find(|f| ctx.is_set(f)) {
            return Err(ToolError::invalid(
                &format!("/{f}"),
                "Give the POH table readings or a rate of climb with its speeds and wind, not both.",
            ));
        }
        let rows = ctx.rows("poh_table")?;
        let mut read = [[0.0; 3]; 2];
        for (i, r) in rows.iter().enumerate() {
            read[i] = [
                ctx.row_quantity("poh_table", i, r, "time")?
                    .expect("required")
                    .to(min),
                ctx.row_quantity("poh_table", i, r, "fuel")?
                    .expect("required")
                    .to(gal),
                ctx.row_quantity("poh_table", i, r, "distance")?
                    .expect("required")
                    .to(nm),
            ];
        }
        let [at_field, at_cruise] = read;
        if (0..3).any(|k| !(at_cruise[k] >= at_field[k] && at_field[k] >= 0.0)) {
            return Err(ToolError::invalid(
                "/poh_table/1",
                "Read the field’s row first and the cruise row second: every cruise value is at least the field’s, and none is negative.",
            ));
        }
        let d = [
            at_cruise[0] - at_field[0],
            at_cruise[1] - at_field[1],
            at_cruise[2] - at_field[2],
        ];
        ctx.step(
            "Fuel to climb",
            "fuel = table fuel at cruise − at the field",
            format!("{} − {} gal", n(at_cruise[1], 1), n(at_field[1], 1)),
            display::quantity(d[1], "galUS", Precision::Decimals(1), fmt),
        );
        ctx.step(
            "Time to climb",
            "time = table time at cruise − at the field",
            format!("{} − {} min", n(at_cruise[0], 1), n(at_field[0], 1)),
            display::quantity(d[0], "min", Precision::Decimals(0), fmt),
        );
        ctx.step(
            "Top of climb",
            "distance = table distance at cruise − at the field",
            format!("{} − {} NM", n(at_cruise[2], 1), n(at_field[2], 1)),
            display::quantity(d[2], "NM", Precision::Decimals(1), fmt),
        );
        (d[0], d[2], Some(d[1]), None)
    } else {
        let rate = ctx.quantity("climb_rate")?.ok_or_else(|| {
            ToolError::invalid(
                "/climb_rate",
                "Give the average rate of climb, like 500 fpm, or the POH table readings.",
            )
        })?;
        let rate = rate.to(unit(QT::VerticalSpeed, "ft/min"));
        if rate.is_nan() || rate <= 0.0 {
            return Err(ToolError::invalid(
                "/climb_rate",
                "The rate of climb must be above zero.",
            ));
        }
        let tas = ctx
            .quantity("climb_tas")?
            .ok_or_else(|| {
                ToolError::invalid(
                    "/climb_tas",
                    "Give the true airspeed in the climb, like 90 kt.",
                )
            })?
            .to(unit(QT::Speed, "kt"));
        if tas.is_nan() || tas <= 0.0 {
            return Err(ToolError::invalid(
                "/climb_tas",
                "The climb true airspeed must be above zero.",
            ));
        }
        let wind = read_wind(ctx)?;
        let course = ctx
            .quantity("course")?
            .map(|c| wrap_azimuth(c.to(unit(QT::Angle, "deg"))));
        let gs = match (wind, course) {
            (Some(w), Some(c)) => groundspeed(c, tas, w, "/wind_speed")?,
            (Some(_), None) => {
                return Err(ToolError::invalid(
                    "/course",
                    "Give the course, like 031 deg, so the wind can be applied.",
                ));
            }
            (None, c) => groundspeed(c.unwrap_or(0.0), tas, (0.0, 0.0), "/climb_tas")?,
        };
        let burn = match ctx.quantity("climb_burn")? {
            Some(b) => {
                let b = b.to(unit(QT::VolumeFlow, "galUS/h"));
                if b.is_nan() || b <= 0.0 {
                    return Err(ToolError::invalid(
                        "/climb_burn",
                        "The fuel burn must be above zero.",
                    ));
                }
                Some(b)
            }
            None => None,
        };
        let time = height / rate;
        let dist = gs * time / 60.0;
        let fuel = burn.map(|b| b * time / 60.0);
        ctx.step(
            "Time to climb",
            "time = (cruise − field) ÷ rate of climb",
            format!("{} ft ÷ {} fpm", n(height, 0), n(rate, 0)),
            display::quantity(time, "min", Precision::Decimals(1), fmt),
        );
        if let (Some(f), Some(b)) = (fuel, burn) {
            ctx.step(
                "Fuel to climb",
                "fuel = burn × time",
                format!("{} gal/h × {} min ÷ 60", n(b, 1), n(time, 1)),
                display::quantity(f, "galUS", Precision::Decimals(1), fmt),
            );
        }
        ctx.step(
            "Top of climb",
            "distance = groundspeed × time",
            format!("{} kt × {} min ÷ 60", n(gs, 1), n(time, 1)),
            display::quantity(dist, "NM", Precision::Decimals(1), fmt),
        );
        (time, dist, fuel, Some(gs))
    };
    // Field at cruise: nothing to climb, whatever the table's rows say.
    let flat = height == 0.0;
    let (time, dist, fuel) = if flat {
        (0.0, 0.0, fuel.map(|_| 0.0))
    } else {
        (time, dist, fuel)
    };
    let mut o = vec![
        (
            "top_of_climb",
            ctx.out("top_of_climb", q(dist, QT::Distance, "NM")),
        ),
        ("time", ctx.out("time", q(time, QT::Time, "min"))),
    ];
    if let Some(f) = fuel {
        o.push(("fuel", ctx.out("fuel", q(f, QT::Volume, "galUS"))));
    }
    if let Some(t) = taxi {
        o.push((
            "total_fuel",
            ctx.out(
                "total_fuel",
                q(t + fuel.unwrap_or(0.0), QT::Volume, "galUS"),
            ),
        ));
    }
    if let Some(g) = gs {
        o.push(("groundspeed", ctx.out("groundspeed", q(g, QT::Speed, "kt"))));
    }
    o.push(("height", ctx.out("height", q(height, QT::Length, "ft"))));
    if flat {
        o.push((
            "note",
            Json::str("No climb is needed: the field is at the cruise altitude."),
        ));
    }
    Ok(obj(o))
}

// ---------------------------------------------------------------- equal time point and point of no return

pub(crate) const CASA_AC_91_15: Reference = Reference {
    title: "Guidelines for aircraft fuel requirements, Advisory Circular AC 91-15",
    issuer: "Civil Aviation Safety Authority (Australia)",
    year: 2024,
    edition: "AC 91-15 v1.2",
    locator: "Annex B (sample fuel calculations, Beechcraft B200), pages B9 and B10 and Table 9: the critical point (equal time point) = total distance × GS back ÷ (GS back + GS forward); the AC's terms define the critical point and the point of no return",
    url: "https://www.casa.gov.au/guidelines-aircraft-fuel-requirements",
};

pub static ETP_PNR: ToolDef = ToolDef {
    id: "aviation.performance.etp-pnr",
    title: "Equal time point and point of no return",
    summary: "Where on a leg it is as quick to go on as to turn back (the equal time point), and how far and how long you can fly out and still return on your fuel (the point of no return), from the groundspeeds out and back.",
    aliases: &[
        "equal time point",
        "point of no return",
        "critical point",
        "point of safe return",
    ],
    keywords: &[
        "ETP",
        "PNR",
        "PSR",
        "critical point",
        "equal time point",
        "point of no return",
        "over water",
        "safe endurance",
        "turn back",
    ],
    inputs: &[
        qty(
            "distance",
            "Leg distance",
            "Departure to destination, like 906 NM",
            QT::Distance,
            "NM",
        )
        .required()
        .core(),
        qty("tas", "True airspeed", "Like 270 kt", QT::Speed, "kt").core(),
        qty(
            "course",
            "Course",
            "True, departure to destination, like 090 deg",
            QT::Angle,
            "deg",
        )
        .core()
        .angle_range("[0,360)"),
        qty(
            "wind_direction",
            "Wind direction",
            "True, like 090 deg",
            QT::Angle,
            "deg",
        )
        .core()
        .angle_range("[0,360)"),
        qty("wind_speed", "Wind speed", "Like 20 kt", QT::Speed, "kt").core(),
        qty(
            "groundspeed_out",
            "Groundspeed going on",
            "Instead of TAS and wind, like 250 kt",
            QT::Speed,
            "kt",
        ),
        qty(
            "groundspeed_back",
            "Groundspeed turning back",
            "Instead of TAS and wind, like 290 kt",
            QT::Speed,
            "kt",
        ),
        qty(
            "usable_fuel",
            "Usable fuel",
            "At departure, for the point of no return, like 53 gal",
            QT::Volume,
            "galUS",
        ),
        qty(
            "reserve_fuel",
            "Reserve fuel",
            "To keep at the end, like 6 gal",
            QT::Volume,
            "galUS",
        ),
        qty(
            "fuel_burn",
            "Fuel burn",
            "In cruise, like 9 gal/h",
            QT::VolumeFlow,
            "galUS/h",
        ),
        qty(
            "safe_endurance",
            "Safe endurance",
            "Instead of fuel: flying time before the reserve, like 4.5 h",
            QT::Time,
            "h",
        ),
    ],
    outputs: &[
        out(
            "etp_distance",
            "Equal time point",
            "Distance from departure",
            QT::Distance,
            "NM",
            1,
        ),
        out(
            "etp_to_destination",
            "Equal time point to destination",
            "Leg distance − the ETP distance",
            QT::Distance,
            "NM",
            1,
        ),
        out(
            "etp_time",
            "Time to the equal time point",
            "From departure at the groundspeed going on",
            QT::Time,
            "min",
            0,
        ),
        out(
            "groundspeed_out",
            "Groundspeed going on",
            "Toward the destination",
            QT::Speed,
            "kt",
            0,
        ),
        out(
            "groundspeed_back",
            "Groundspeed turning back",
            "Toward departure",
            QT::Speed,
            "kt",
            0,
        ),
        out(
            "safe_endurance",
            "Safe endurance",
            "(Usable fuel − reserve) ÷ burn",
            QT::Time,
            "min",
            0,
        )
        .optional(),
        out(
            "pnr_time",
            "Time to the point of no return",
            "Safe endurance × GS back ÷ (GS out + GS back)",
            QT::Time,
            "min",
            0,
        )
        .optional(),
        out(
            "pnr_distance",
            "Point of no return",
            "Distance from departure",
            QT::Distance,
            "NM",
            1,
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    when_to_use: "Use this when planning a long leg over water or remote country: the equal time point tells you where, if something goes wrong, going on and turning back take the same time; the point of no return tells you the last point from which you can still get back to departure on your fuel, keeping the reserve.",
    limitations: "One steady wind for the whole leg, applied both ways, and the same true airspeed going on and turning back. A real diversion may fly at another altitude, speed, or burn, such as after an engine failure or a loss of pressurization; work those cases with their own groundspeeds. The point of no return here is back to departure, not to an en route alternate.",
    model: "ETP distance = D × GSback ÷ (GSout + GSback); PNR time = E × GSback ÷ (GSout + GSback), PNR distance = PNR time × GSout, with safe endurance E = (usable fuel − reserve) ÷ burn. Groundspeeds from the TAS and wind by the wind triangle of aviation.wind.heading-groundspeed, on the course out and its reciprocal back",
    accuracy: "Exact arithmetic for a steady wind and constant speeds. Planning aid, not certified for navigation.",
    references: &[CASA_AC_91_15],
    examples: &[Example {
        id: "primary",
        title: "906 NM at 270 kt into a 20 kt headwind, with 4 hours of safe endurance",
        input: r#"{"distance":"906 NM","tas":"270 kt","course":"090 deg","wind_direction":"090 deg","wind_speed":"20 kt","safe_endurance":"4 h"}"#,
        source: "CASA AC 91-15 Annex B, Table 9 (Darwin to Cairns, 906 NM, TAS 270 kt, 20 kt headwind on and tailwind back): critical point 487 NM from Darwin; the 4 h endurance is added to show the point of no return, 4 h × 290 ÷ 540 = 129 min",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("etp", "etp_distance")],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "parent",
        },
        Related {
            id: "aviation.loading.fuel-plan",
            reason: "next",
        },
        Related {
            id: "aviation.flight-plan.nav-log",
            reason: "alternative",
        },
    ],
    sentence: "The equal time point is {etp_distance} from departure, {etp_time} out.{if pnr_time > 0} The point of no return is {pnr_distance} out, {pnr_time} after departure.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_etp_pnr,
    ..ToolDef::BLANK
};

fn run_etp_pnr(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let kt = unit(QT::Speed, "kt");
    let d = ctx.req_quantity("distance")?.to(unit(QT::Distance, "NM"));
    if d.is_nan() || d <= 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The leg distance must be above zero, like 906 NM.",
        ));
    }
    let given = (
        ctx.quantity("groundspeed_out")?,
        ctx.quantity("groundspeed_back")?,
    );
    let (go, gb) = match given {
        (Some(o), Some(b)) => {
            if let Some(f) = ["tas", "course", "wind_direction", "wind_speed"]
                .iter()
                .find(|f| ctx.is_set(f))
            {
                return Err(ToolError::invalid(
                    &format!("/{f}"),
                    "Give the groundspeeds, or the TAS and wind to work them out, not both.",
                ));
            }
            let (o, b) = (o.to(kt), b.to(kt));
            if o.is_nan() || b.is_nan() || o <= 0.0 || b <= 0.0 {
                return Err(ToolError::invalid(
                    "/groundspeed_out",
                    "Both groundspeeds must be above zero.",
                ));
            }
            (o, b)
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(ToolError::invalid(
                if given.0.is_some() {
                    "/groundspeed_back"
                } else {
                    "/groundspeed_out"
                },
                "Give both groundspeeds, going on and turning back.",
            ));
        }
        (None, None) => {
            let tas = ctx
                .quantity("tas")?
                .ok_or_else(|| {
                    ToolError::invalid(
                        "/tas",
                        "Give the true airspeed, like 270 kt, or both groundspeeds.",
                    )
                })?
                .to(kt);
            if tas.is_nan() || tas <= 0.0 {
                return Err(ToolError::invalid(
                    "/tas",
                    "True airspeed must be greater than zero.",
                ));
            }
            let wind = read_wind(ctx)?;
            let course = ctx
                .quantity("course")?
                .map(|c| wrap_azimuth(c.to(unit(QT::Angle, "deg"))));
            match (wind, course) {
                (Some(w), Some(c)) => (
                    groundspeed(c, tas, w, "/wind_speed")?,
                    groundspeed(wrap_azimuth(c + 180.0), tas, w, "/wind_speed")?,
                ),
                (Some(_), None) => {
                    return Err(ToolError::invalid(
                        "/course",
                        "Give the course, like 090 deg, so the wind can be applied both ways.",
                    ));
                }
                (None, _) => (tas, tas),
            }
        }
    };
    // The share of the leg (and of the endurance) spent going out: GSback ÷
    // (GSout + GSback). With no wind it is exactly one half.
    let share = gb / (go + gb);
    let etp = d * share;
    let etp_min = etp / go * 60.0;
    let (gal, gph) = (unit(QT::Volume, "galUS"), unit(QT::VolumeFlow, "galUS/h"));
    let endurance_h = match (
        ctx.quantity("safe_endurance")?,
        ctx.quantity("usable_fuel")?,
    ) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/safe_endurance",
                "Give the safe endurance or the fuel to work it out, not both.",
            ));
        }
        (Some(e), None) => Some(e.to(unit(QT::Time, "h"))),
        (None, Some(u)) => {
            let burn = ctx.quantity("fuel_burn")?.ok_or_else(|| {
                ToolError::invalid(
                    "/fuel_burn",
                    "Give the fuel burn with the usable fuel, like 9 gal/h.",
                )
            })?;
            let reserve = ctx.quantity("reserve_fuel")?;
            let (u, r, b) = (u.to(gal), reserve.map(|r| r.to(gal)), burn.to(gph));
            if r.is_some_and(|r| r.is_nan() || r < 0.0) {
                return Err(ToolError::invalid(
                    "/reserve_fuel",
                    "The reserve cannot be negative.",
                ));
            }
            if b.is_nan() || b <= 0.0 {
                return Err(ToolError::invalid(
                    "/fuel_burn",
                    "The fuel burn must be above zero.",
                ));
            }
            Some((u - r.unwrap_or(0.0)) / b)
        }
        (None, None) => None,
    };
    if let Some(e) = endurance_h
        && (e.is_nan() || e <= 0.0)
    {
        return Err(ToolError::invalid(
            "/reserve_fuel",
            "The reserve takes all the fuel, so there is no safe endurance to fly out on.",
        ));
    }
    let fmt = ctx.options.format;
    let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
    let mut o = vec![
        (
            "etp_distance",
            ctx.out("etp_distance", q(etp, QT::Distance, "NM")),
        ),
        (
            "etp_to_destination",
            ctx.out("etp_to_destination", q(d - etp, QT::Distance, "NM")),
        ),
        ("etp_time", ctx.out("etp_time", q(etp_min, QT::Time, "min"))),
        (
            "groundspeed_out",
            ctx.out("groundspeed_out", q(go, QT::Speed, "kt")),
        ),
        (
            "groundspeed_back",
            ctx.out("groundspeed_back", q(gb, QT::Speed, "kt")),
        ),
    ];
    if let Some(e) = endurance_h {
        let t = e * share;
        o.push((
            "safe_endurance",
            ctx.out("safe_endurance", q(e * 60.0, QT::Time, "min")),
        ));
        o.push((
            "pnr_time",
            ctx.out("pnr_time", q(t * 60.0, QT::Time, "min")),
        ));
        o.push((
            "pnr_distance",
            ctx.out("pnr_distance", q(t * go, QT::Distance, "NM")),
        ));
        ctx.step(
            "Point of no return",
            "PNR time = E × GSback ÷ (GSout + GSback)",
            format!(
                "{} h × {} ÷ ({} + {})",
                n(e, 2),
                n(gb, 1),
                n(go, 1),
                n(gb, 1)
            ),
            display::quantity(t * 60.0, "min", Precision::Decimals(0), fmt),
        );
    }
    ctx.step(
        "Equal time point",
        "ETP = D × GSback ÷ (GSout + GSback)",
        format!(
            "{} NM × {} ÷ ({} + {})",
            n(d, 1),
            n(gb, 1),
            n(go, 1),
            n(gb, 1)
        ),
        display::quantity(etp, "NM", Precision::Decimals(1), fmt),
    );
    Ok(obj(o))
}
