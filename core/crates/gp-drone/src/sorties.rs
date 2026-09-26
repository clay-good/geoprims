//! Sorties and battery swaps (drone/mission-planning spec, "Sorties and
//! battery swaps"). Splits a waypoint path into the flights one battery each
//! can fly, ending every flight at the last waypoint from which the trip home
//! still lands within the reserve. The energy arithmetic is the endurance
//! tool's (`power::usable_energy`) and the trip home is the return-to-home
//! tool's (`power::return_home`); distances are geodesics (Karney 2013).

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};

use crate::mission::{CENTER_ROW, KARNEY};
use crate::power::{LEISHMAN, return_home, usable_energy};

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
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

const fn count(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Number { min: 0.0, max: 1e6 })
        .precision(Precision::Decimals(0))
}

const fn minutes(name: &'static str, title: &'static str, help: &'static str) -> Field {
    qty(name, title, help, QT::Time, "min").precision(Precision::Decimals(1))
}

const fn degrees(name: &'static str, title: &'static str) -> Field {
    qty(name, title, "Degrees", QT::Angle, "deg").precision(Precision::Decimals(7))
}

const SORTIE_ROW: &[Field] = &[
    count("sortie", "Sortie", "Its number, one battery each"),
    count(
        "first_waypoint",
        "First waypoint",
        "Where it starts on the path",
    ),
    count(
        "last_waypoint",
        "Last waypoint",
        "Where it ends and turns home",
    ),
    minutes(
        "flying_time",
        "Flying time",
        "Out from home and along the path",
    ),
    minutes("return_time", "Return time", "Home from the last waypoint"),
    qty(
        "return_energy",
        "Return energy",
        "Home from the last waypoint, when the battery energy is given",
        QT::Energy,
        "Wh",
    )
    .precision(Precision::Decimals(1))
    .optional(),
];

const SWAP_ROW: &[Field] = &[
    degrees("lat", "Latitude"),
    degrees("lon", "Longitude"),
    count(
        "waypoint",
        "Waypoint",
        "The waypoint the next battery resumes at",
    ),
    count("sortie", "Sortie", "The sortie that ends here"),
];

const PATH_ROW: &[Field] = &[
    degrees("lat", "Latitude"),
    degrees("lon", "Longitude"),
    count("part", "Sortie", "The sortie that flies this point"),
];

pub static SORTIES: ToolDef = ToolDef {
    id: "drone.mission.sorties",
    title: "Mission sorties and battery swaps",
    summary: "How many batteries a waypoint mission takes and where each flight ends, with the trip home against the wind counted before every swap, not just the total time divided by battery time.",
    aliases: &[
        "how many batteries for a drone mission",
        "battery swap points",
        "drone mission sorties",
        "split a survey into flights",
    ],
    keywords: &[
        "sorties",
        "batteries",
        "battery swap",
        "flights",
        "return to home",
        "reserve",
        "mission planning",
        "waypoints",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Home latitude",
            "Takeoff and landing point, like 40.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Home longitude",
            "Takeoff and landing point, like -105.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-180,180)"),
        Field::new(
            "waypoints",
            "Waypoints",
            "The path in flight order, like the survey grid's waypoints",
            Kind::List {
                items: CENTER_ROW,
                min: 1,
                max: 20_000,
            },
        )
        .required()
        .core(),
        qty(
            "flight_time",
            "Flight time per battery",
            "Usable flying time on a full battery, like 20 min",
            QT::Time,
            "min",
        )
        .core(),
        qty(
            "groundspeed",
            "Survey groundspeed",
            "Along the path, like 10 m/s",
            QT::Speed,
            "m/s",
        )
        .required()
        .core(),
        Field::new(
            "reserve",
            "Landing reserve (%)",
            "Share of each battery kept for landing, like 20; default 0",
            Kind::Number {
                min: 0.0,
                max: 99.0,
            },
        )
        .core(),
        qty(
            "transit_speed",
            "Transit speed",
            "Airspeed to and from home, like 15 m/s; default the survey groundspeed",
            QT::Speed,
            "m/s",
        ),
        qty(
            "headwind",
            "Headwind on the way home",
            "Like 5 m/s; applied to every trip home",
            QT::Speed,
            "m/s",
        ),
        qty(
            "energy",
            "Battery energy",
            "Instead of a flight time, with the power, like 90.4 Wh",
            QT::Energy,
            "Wh",
        ),
        Field::new(
            "usable",
            "Usable share (%)",
            "Of the battery energy, like 80; default 100",
            Kind::Number {
                min: 1.0,
                max: 100.0,
            },
        ),
        qty(
            "power",
            "Cruise power",
            "With the battery energy, like 180 W",
            QT::Power,
            "W",
        ),
    ],
    outputs: &[
        count("batteries", "Batteries", "One per sortie"),
        minutes(
            "total_time",
            "Total flying time",
            "Every sortie, out, along the path, and home",
        ),
        minutes(
            "battery_time",
            "Time per battery to the reserve",
            "Flight time less the reserve",
        ),
        Field::new(
            "sorties",
            "Sorties",
            "Each flight's waypoints and times",
            Kind::List {
                items: SORTIE_ROW,
                min: 0,
                max: 20_000,
            },
        ),
        Field::new(
            "swap_points",
            "Swap points",
            "Where one battery ends and the next resumes",
            Kind::List {
                items: SWAP_ROW,
                min: 0,
                max: 20_000,
            },
        ),
        Field::new(
            "path",
            "Flights",
            "Home, the sortie's waypoints, and home again, for each sortie",
            Kind::List {
                items: PATH_ROW,
                min: 0,
                max: 100_000,
            },
        ),
    ],
    errors: &[ErrorCode::NoSolution, ErrorCode::OutOfDomain],
    warnings: &[
        "CANNOT_RETURN_INTO_WIND",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Flight time to the reserve F = flight time × (1 − reserve), or usable energy × (1 − reserve) / power (the endurance tool). A sortie starting at waypoint s flies home → s at the transit speed, then the path at the survey groundspeed; at each waypoint j the trip home takes d_j / (V − w) (the return-to-home tool), and the sortie ends at the last j where elapsed + trip home ≤ F. The next sortie flies out to that waypoint and carries on. Distances are WGS84 geodesics",
    accuracy: "Exact for straight legs at constant speeds and power, a steady headwind on every trip home, and no time for turns, climbs, or landing. Real batteries also sag in the cold and with age.",
    when_to_use: "Use this when a mapping or inspection path is longer than one battery: it says how many batteries the job takes and where to break it, counting the trip home from each break point, which grows as the path moves away from home. Feed it the survey grid’s waypoints and your battery’s usable flight time.",
    limitations: "Every flight leg is flown at a constant speed with no time for takeoff, climb, turns, or landing, so leave those in the reserve. The wind is one headwind applied to every trip home, which is cautious; the trips out and along the path ignore it. A sortie ends only at a waypoint, so on long legs it can end well short of what the battery could reach. It does not check airspace, terrain, or line of sight.",
    references: &[LEISHMAN, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 1.2 km, four-line grid on a 7-minute battery with a 20% reserve",
        input: r#"{"lat":40.0,"lon":-105.0,"waypoints":[{"lat":40.0009,"lon":-105.0},{"lat":40.0009,"lon":-104.98592},{"lat":40.0027,"lon":-104.98592},{"lat":40.0027,"lon":-105.0},{"lat":40.0045,"lon":-105.0},{"lat":40.0045,"lon":-104.98592},{"lat":40.0063,"lon":-104.98592},{"lat":40.0063,"lon":-105.0}],"flight_time":"7 min","groundspeed":"10 m/s","reserve":20}"#,
        source: "Hand-worked in docs/derivations/drone.mission.sorties.md: three sorties, swapping at waypoints 4 and 7",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "line-geodesic",
            map: &[("path", "path")],
        },
        Layer {
            kind: "point",
            map: &[("marks", "swap_points")],
        },
    ],
    related: &[
        Related {
            id: "drone.mission.survey-grid",
            reason: "parent",
        },
        Related {
            id: "drone.power.endurance",
            reason: "parent",
        },
        Related {
            id: "drone.power.rth-budget",
            reason: "alternative",
        },
        Related {
            id: "drone.ops.vlos-check",
            reason: "next",
        },
    ],
    sentence: "The mission takes {batteries} {plural batteries \"battery\" \"batteries\"} and about {total_time} of flying.",
    limits: &[("batchRows", 100)],
    run: run_sorties,
    ..ToolDef::BLANK
};

/// One flight: first and last waypoint (0-based), flying time out and along
/// the path, and the trip home (time, energy) from the last waypoint.
struct Sortie {
    first: usize,
    last: usize,
    flying: f64,
    home_t: f64,
    home_e: f64,
}

fn run_sorties(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let home = (
        ctx.req_quantity("lat")?.to(deg),
        ctx.req_quantity("lon")?.to(deg),
    );
    let rows = ctx.rows("waypoints")?;
    let mut wp = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let la = ctx
            .row_quantity("waypoints", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lo = ctx
            .row_quantity("waypoints", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&la) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/waypoints/{i}/lat")));
        }
        wp.push((la, lo));
    }
    if !(-90.0..=90.0).contains(&home.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Latitude must be between -90° and 90°.",
        )
        .at("/lat"));
    }
    let v_c = ctx.req_quantity("groundspeed")?.base();
    if v_c <= 0.0 {
        return Err(ToolError::invalid(
            "/groundspeed",
            "The survey groundspeed must be positive.",
        ));
    }
    let v_t = ctx.quantity("transit_speed")?.map_or(v_c, |x| x.base());
    if v_t <= 0.0 {
        return Err(ToolError::invalid(
            "/transit_speed",
            "The transit speed must be positive.",
        ));
    }
    let w = ctx.quantity("headwind")?.map_or(0.0, |x| x.base());
    let reserve = ctx.number("reserve")?.map_or(0.0, |p| p / 100.0);
    // The battery as time at a constant power: from a flight time (power
    // taken as 1 W, so energy in joules is seconds), or from energy and power.
    let (avail, flyable, power, energy_given) = match (
        ctx.quantity("flight_time")?,
        ctx.quantity("energy")?,
        ctx.quantity("power")?,
    ) {
        (Some(t), None, None) => {
            let t = t.base();
            if t <= 0.0 {
                return Err(ToolError::invalid(
                    "/flight_time",
                    "The flight time must be positive.",
                ));
            }
            let (a, f) = usable_energy(t, 1.0, 0.0, reserve);
            (a, f, 1.0, false)
        }
        (None, Some(e), Some(p)) => {
            let (e, p) = (e.base(), p.base());
            if e <= 0.0 || p <= 0.0 {
                return Err(ToolError::invalid(
                    "/energy",
                    "The battery energy and power must be positive.",
                ));
            }
            let usable = ctx.number("usable")?.map_or(1.0, |u| u / 100.0);
            let (a, f) = usable_energy(e, usable, 0.0, reserve);
            (a, f, p, true)
        }
        _ => {
            return Err(ToolError::invalid(
                "/flight_time",
                "Give a flight time per battery, or the battery energy and the cruise power, not both.",
            )
            .hint("Example: 20 min, or 90.4 Wh with 180 W"));
        }
    };
    if w.abs() >= v_t {
        ctx.warnings.push(Warning::new(
            "CANNOT_RETURN_INTO_WIND",
            "The headwind is at least as fast as the transit speed, so the drone cannot make progress home against it.",
        ));
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "The headwind is at least as fast as the transit speed: the drone cannot fly home against it.",
        )
        .at("/headwind"));
    }
    let f_time = flyable / power;
    let geod = Geodesic::wgs84();
    let dist = |a: (f64, f64), b: (f64, f64)| -> f64 { geod.inverse(a.0, a.1, b.0, b.1) };
    let d_home: Vec<f64> = wp.iter().map(|&p| dist(home, p)).collect();
    let legs: Vec<f64> = wp.windows(2).map(|x| dist(x[0], x[1])).collect();
    // Seconds on the clock, with room for round-off in the last digit.
    let fits = |elapsed: f64, j: usize| {
        let (_, t, _) = return_home(d_home[j], v_t, w, power);
        elapsed + t <= f_time * (1.0 + 1e-12)
    };
    // A waypoint no battery can reach and come home from: the plan cannot be
    // flown, so the answer is an error naming the waypoint and its distance.
    let out_of_range = |j: usize, why: String| {
        ToolError::new(ErrorCode::NoSolution, why).at(&format!("/waypoints/{j}"))
    };
    let show = |x: f64, u: &str, fmt| display::quantity(x, u, Precision::Decimals(0), fmt);
    let fmt = ctx.options.format;
    let mut sorties: Vec<Sortie> = Vec::new();
    let mut s = 0;
    loop {
        let mut elapsed = d_home[s] / v_t;
        if !fits(elapsed, s) {
            return Err(out_of_range(
                s,
                format!(
                    "Waypoint {} is {} from home: too far to fly out to and back on one battery.",
                    s + 1,
                    show(d_home[s], "m", fmt)
                ),
            ));
        }
        let mut j = s;
        while j + 1 < wp.len() && fits(elapsed + legs[j] / v_c, j + 1) {
            elapsed += legs[j] / v_c;
            j += 1;
        }
        if j == s && j + 1 < wp.len() {
            return Err(out_of_range(
                j + 1,
                format!(
                    "Waypoint {} is {} from home: one battery cannot fly out to waypoint {}, on to it, and back.",
                    j + 2,
                    show(d_home[j + 1], "m", fmt),
                    j + 1
                ),
            ));
        }
        let (_, t, e) = return_home(d_home[j], v_t, w, power);
        sorties.push(Sortie {
            first: s,
            last: j,
            flying: elapsed,
            home_t: t,
            home_e: e,
        });
        if j + 1 >= wp.len() {
            break;
        }
        s = j;
    }
    let total: f64 = sorties.iter().map(|x| x.flying + x.home_t).sum();
    if ctx.explaining() {
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Time per battery",
            "flight time × (1 − reserve)",
            format!(
                "{} min × (1 − {}%)",
                n(avail / power / 60.0, 1),
                n(reserve * 100.0, 0)
            ),
            format!("{} min", n(f_time / 60.0, 1)),
        );
        for (k, x) in sorties.iter().enumerate() {
            ctx.step(
                format!("Sortie {}", k + 1),
                "ends at the last waypoint where time flown + time home ≤ time per battery",
                format!(
                    "waypoints {} to {}: {} min flown + {} min home",
                    x.first + 1,
                    x.last + 1,
                    n(x.flying / 60.0, 1),
                    n(x.home_t / 60.0, 1)
                ),
                format!("{} min", n((x.flying + x.home_t) / 60.0, 1)),
            );
        }
        // The card shows the battery count bare; the last step reads the same.
        ctx.step(
            "Batteries",
            "one per sortie",
            format!("{} sorties", sorties.len()),
            n(sorties.len() as f64, 0),
        );
    }
    let min = |sec: f64| q(sec / 60.0, QT::Time, "min").to_json();
    let dj = |v: f64| q(v, QT::Angle, "deg").to_json();
    let rows_out: Vec<Json> = sorties
        .iter()
        .enumerate()
        .map(|(k, x)| {
            let mut r = vec![
                ("sortie", Json::Num((k + 1) as f64)),
                ("first_waypoint", Json::Num((x.first + 1) as f64)),
                ("last_waypoint", Json::Num((x.last + 1) as f64)),
                ("flying_time", min(x.flying)),
                ("return_time", min(x.home_t)),
            ];
            if energy_given {
                r.push((
                    "return_energy",
                    q(x.home_e / 3600.0, QT::Energy, "Wh").to_json(),
                ));
            }
            Json::obj(r)
        })
        .collect();
    let swaps: Vec<Json> = sorties
        .iter()
        .enumerate()
        .take(sorties.len() - 1)
        .map(|(k, x)| {
            Json::obj([
                ("lat", dj(wp[x.last].0)),
                ("lon", dj(wp[x.last].1)),
                ("waypoint", Json::Num((x.last + 1) as f64)),
                ("sortie", Json::Num((k + 1) as f64)),
            ])
        })
        .collect();
    let mut path = Vec::new();
    for (k, x) in sorties.iter().enumerate() {
        let part = Json::Num((k + 1) as f64);
        let pt =
            |p: (f64, f64)| Json::obj([("lat", dj(p.0)), ("lon", dj(p.1)), ("part", part.clone())]);
        path.push(pt(home));
        path.extend(wp[x.first..=x.last].iter().map(|&p| pt(p)));
        path.push(pt(home));
    }
    Ok(Json::obj(vec![
        ("batteries", Json::Num(sorties.len() as f64)),
        ("total_time", ctx.out("total_time", q(total, QT::Time, "s"))),
        (
            "battery_time",
            ctx.out("battery_time", q(f_time, QT::Time, "s")),
        ),
        ("sorties", Json::Arr(rows_out)),
        ("swap_points", Json::Arr(swaps)),
        ("path", Json::Arr(path)),
    ]))
}
