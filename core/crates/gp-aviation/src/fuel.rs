//! Fuel planning (aviation/fuel-and-loading, "Fuel planning" and "Reserve
//! requirements as dated reference data"): trip fuel by leg at each leg's
//! burn, taxi and climb allowances, fuel to an alternate, and a reserve from
//! a dated 14 CFR preset (by aircraft category) or your own, checked against
//! usable fuel.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::regulation::rule;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_base::{ErrorCode, display};

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const CFR_91: Reference = Reference {
    title: "14 CFR Part 91, General Operating and Flight Rules",
    issuer: "Federal Aviation Administration",
    year: 2000,
    edition: "eCFR, current",
    locator: "§ 91.151 (fuel requirements for flight in VFR conditions) and § 91.167 (fuel requirements for flight in IFR conditions)",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91",
};

const LEG: &[Field] = &[
    qty("time", "Time", "Like 1.5 h", QT::Time, "h").required(),
    qty(
        "burn",
        "Fuel burn",
        "Like 9.5 gal/h",
        QT::VolumeFlow,
        "galUS/h",
    )
    .required(),
];

const LEG_OUT: &[Field] = &[
    Field::new(
        "leg",
        "Leg",
        "In the order given",
        Kind::Number {
            min: 1.0,
            max: 20.0,
        },
    )
    .precision(Precision::Decimals(0)),
    qty("fuel", "Fuel", "Time × burn", QT::Volume, "galUS").precision(Precision::Decimals(1)),
];

const RESERVES: &[&str] = &["vfr-day", "vfr-night", "ifr", "custom", "none"];

pub static FUEL_PLAN: ToolDef = ToolDef {
    id: "aviation.loading.fuel-plan",
    version: "1.1.0",
    title: "Fuel planning",
    summary: "Fuel for a trip by leg, with taxi and climb allowances, fuel to an alternate, and a reserve from the 14 CFR 91.151 or 91.167 minimum, checked against your usable fuel.",
    aliases: &[
        "fuel planner",
        "fuel required",
        "fuel reserve",
        "VFR fuel reserve",
        "IFR fuel requirements",
    ],
    keywords: &[
        "fuel",
        "reserve",
        "91.151",
        "91.167",
        "endurance",
        "burn",
        "gph",
        "alternate",
        "legs",
    ],
    inputs: &[
        Field::new(
            "legs",
            "Legs",
            "Time and fuel burn for each leg, like 1.5 h at 9.5 gal/h",
            Kind::List {
                items: LEG,
                min: 1,
                max: 20,
            },
        )
        .required()
        .core(),
        Field::new(
            "reserve",
            "Reserve",
            "vfr-day, vfr-night, ifr (the 14 CFR minimums), custom, or none",
            Kind::Choice(RESERVES),
        )
        .required()
        .core(),
        qty(
            "usable_fuel",
            "Usable fuel on board",
            "To check the plan, like 53 gal",
            QT::Volume,
            "galUS",
        )
        .core(),
        qty(
            "taxi",
            "Start, taxi, and run-up",
            "Like 1.4 gal",
            QT::Volume,
            "galUS",
        )
        .core(),
        Field::new(
            "category",
            "Aircraft category",
            "airplane (the default) or rotorcraft; sets the reserve minimums",
            Kind::Choice(&["airplane", "rotorcraft"]),
        ),
        qty(
            "climb",
            "Extra fuel to climb",
            "From the POH climb table, like 1.5 gal",
            QT::Volume,
            "galUS",
        ),
        qty(
            "alternate_time",
            "Time to the alternate",
            "For IFR with an alternate, like 30 min",
            QT::Time,
            "min",
        ),
        qty(
            "cruise_burn",
            "Cruise burn for the reserve",
            "At normal cruising speed, like 9 gal/h; defaults to the last leg's burn",
            QT::VolumeFlow,
            "galUS/h",
        ),
        qty(
            "custom_reserve",
            "Your reserve time",
            "For a custom reserve, like 60 min",
            QT::Time,
            "min",
        ),
    ],
    outputs: &[
        qty(
            "total",
            "Fuel required",
            "Taxi + climb + trip + alternate + reserve",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "trip",
            "Trip fuel",
            "Every leg's time × burn",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "reserve_fuel",
            "Reserve fuel",
            "Reserve time × cruise burn",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "reserve_time",
            "Reserve time",
            "From the rule or your entry",
            QT::Time,
            "min",
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "reserve_rule",
            "Reserve basis",
            "The regulation and the date these rules were checked",
            Kind::Text { max_len: 200 },
        ),
        qty(
            "alternate_fuel",
            "Fuel to the alternate",
            "Time to the alternate × cruise burn",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "margin",
            "Fuel to spare",
            "Usable fuel − fuel required; negative is a shortfall",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "endurance",
            "Endurance",
            "Usable fuel less taxi, at the cruise burn",
            QT::Time,
            "h",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "fuel_status",
            "Fuel check",
            "Within, near, or beyond your usable fuel",
            Kind::Text { max_len: 80 },
        )
        .status("threshold", "user")
        .optional(),
        Field::new(
            "legs",
            "Fuel by leg",
            "Each leg's fuel",
            Kind::List {
                items: LEG_OUT,
                min: 1,
                max: 20,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Trip = Σ time × burn over the legs; alternate and reserve = time × cruise burn; total = taxi + climb + trip + alternate + reserve. Reserve minimums: airplane VFR 30 min by day and 45 min at night (14 CFR 91.151(a)), rotorcraft VFR 20 min (91.151(b)), IFR 45 min, or 30 min for helicopters, after the alternate (91.167(a)(3))",
    accuracy: "Arithmetic on your times and burns. The reserves are the regulatory minimums as of the review date shown; operators and good judgment often carry more. Summary, not legal advice.",
    references: &[CFR_91, PHAK],
    examples: &[Example {
        id: "primary",
        title: "Two legs at night with a VFR reserve",
        input: r#"{"legs":[{"time":"1.5 h","burn":"9.5 gal/h"},{"time":"0.75 h","burn":"9 gal/h"}],"reserve":"vfr-night","taxi":"1.4 gal","usable_fuel":"53 gal"}"#,
        source: "add-aviation-suite VFR night scenario: a 45-minute reserve at the entered cruise burn, cited to 14 CFR 91.151(a)(2)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.loading.fuel-weight",
            reason: "next",
        },
        Related {
            id: "aviation.loading.weight-balance",
            reason: "next",
        },
    ],
    sentence: "You need {total} of fuel, including {reserve_fuel} of reserve.{if margin < 0} That is more than you carry.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_fuel_plan,
    ..ToolDef::BLANK
};

fn run_fuel_plan(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (gal, gph, hr, min) = (
        unit(QT::Volume, "galUS"),
        unit(QT::VolumeFlow, "galUS/h"),
        unit(QT::Time, "h"),
        unit(QT::Time, "min"),
    );
    let rows = ctx.rows("legs")?;
    let mut legs: Vec<f64> = Vec::with_capacity(rows.len());
    let mut last_burn = 0.0;
    for (i, r) in rows.iter().enumerate() {
        let t = ctx
            .row_quantity("legs", i, r, "time")?
            .expect("required")
            .to(hr);
        let b = ctx
            .row_quantity("legs", i, r, "burn")?
            .expect("required")
            .to(gph);
        if !(t >= 0.0 && b > 0.0) {
            return Err(ToolError::invalid(
                &format!("/legs/{i}"),
                "Each leg needs a time of zero or more and a fuel burn above zero, like 1.5 h at 9.5 gal/h.",
            ));
        }
        legs.push(t * b);
        last_burn = b;
    }
    let nonneg = |v: Option<Q>, u, field: &str| -> Result<f64, ToolError> {
        match v.map(|q| q.to(u)) {
            Some(x) if x < 0.0 => Err(ToolError::invalid(
                field,
                "Fuel amounts and times cannot be negative.",
            )),
            x => Ok(x.unwrap_or(0.0)),
        }
    };
    let taxi = nonneg(ctx.quantity("taxi")?, gal, "/taxi")?;
    let climb = nonneg(ctx.quantity("climb")?, gal, "/climb")?;
    let alt_time = nonneg(ctx.quantity("alternate_time")?, hr, "/alternate_time")?;
    let cruise = match ctx.quantity("cruise_burn")? {
        Some(q) if q.to(gph) > 0.0 => q.to(gph),
        Some(_) => {
            return Err(ToolError::invalid(
                "/cruise_burn",
                "The cruise burn must be above zero.",
            ));
        }
        None => last_burn,
    };
    let rotorcraft = ctx.choice("category")? == Some("rotorcraft");
    let preset = ctx.choice("reserve")?.expect("required");
    let rule_id = match (preset, rotorcraft) {
        ("vfr-day" | "vfr-night", true) => Some("faa-91-151-rotorcraft"),
        ("vfr-day", false) => Some("faa-91-151-day"),
        ("vfr-night", false) => Some("faa-91-151-night"),
        ("ifr", false) => Some("faa-91-167-ifr"),
        ("ifr", true) => Some("faa-91-167-ifr-helicopter"),
        _ => None,
    };
    let (reserve_min, basis) = match (rule_id, preset) {
        (Some(id), _) => {
            let r = rule(id);
            (
                r.value,
                format!(
                    "{} minimum, rules as of {}; operators may require more. Not legal advice: {}",
                    r.citation, r.reviewed, r.url
                ),
            )
        }
        (None, "custom") => {
            let t = ctx.quantity("custom_reserve")?.ok_or_else(|| {
                ToolError::invalid(
                    "/custom_reserve",
                    "Give your reserve time for a custom reserve, like 60 min.",
                )
            })?;
            let m = nonneg(Some(t), min, "/custom_reserve")?;
            (m, "Your own reserve".to_owned())
        }
        _ => (0.0, "No reserve: the regulations require one".to_owned()),
    };
    if ctx.is_set("custom_reserve") && preset != "custom" {
        return Err(ToolError::invalid(
            "/custom_reserve",
            "A reserve time applies only with the custom reserve; choose custom or clear it.",
        ));
    }
    let trip: f64 = legs.iter().sum();
    let alt_fuel = alt_time * cruise;
    let reserve_fuel = reserve_min / 60.0 * cruise;
    let total = taxi + climb + trip + alt_fuel + reserve_fuel;
    let g = |v: f64| Q {
        value: v,
        unit: gal,
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Trip fuel",
            "trip = Σ time × burn",
            legs.iter()
                .map(|&f| n(f, 1))
                .collect::<Vec<_>>()
                .join(" + "),
            display::quantity(trip, "galUS", Precision::Decimals(1), fmt),
        );
        ctx.step(
            "Reserve",
            "reserve = reserve time × cruise burn",
            format!("{} min ÷ 60 × {} gal/h", n(reserve_min, 0), n(cruise, 1)),
            display::quantity(reserve_fuel, "galUS", Precision::Decimals(1), fmt),
        );
        ctx.step(
            "Fuel required",
            "taxi + climb + trip + alternate + reserve",
            format!(
                "{} + {} + {} + {} + {}",
                n(taxi, 1),
                n(climb, 1),
                n(trip, 1),
                n(alt_fuel, 1),
                n(reserve_fuel, 1)
            ),
            display::quantity(total, "galUS", Precision::Decimals(1), fmt),
        );
    }
    let mut out = vec![
        ("total", ctx.out("total", g(total))),
        ("trip", ctx.out("trip", g(trip))),
        ("reserve_fuel", ctx.out("reserve_fuel", g(reserve_fuel))),
        (
            "reserve_time",
            ctx.out(
                "reserve_time",
                Q {
                    value: reserve_min,
                    unit: min,
                },
            ),
        ),
        ("reserve_rule", Json::str(basis)),
    ];
    if alt_time > 0.0 {
        out.push(("alternate_fuel", ctx.out("alternate_fuel", g(alt_fuel))));
    }
    if let Some(u) = ctx.quantity("usable_fuel")? {
        let usable = u.to(gal);
        if usable <= 0.0 {
            return Err(ToolError::invalid(
                "/usable_fuel",
                "Usable fuel must be above zero.",
            ));
        }
        out.push(("margin", ctx.out("margin", g(usable - total))));
        out.push((
            "endurance",
            ctx.out(
                "endurance",
                Q {
                    value: ((usable - taxi).max(0.0)) / cruise,
                    unit: hr,
                },
            ),
        ));
        let label = format!(
            "{} usable fuel",
            display::quantity(
                u.value,
                u.unit.symbol,
                Precision::Significant(4),
                ctx.options.format
            )
        );
        out.push((
            "fuel_status",
            Json::str(gp_base::status::threshold(
                total,
                usable,
                gp_base::status::NEAR_MARGIN,
                &label,
            )),
        ));
    }
    out.push((
        "legs",
        Json::Arr(
            legs.iter()
                .enumerate()
                .map(|(i, &f)| {
                    Json::obj([("leg", Json::Num((i + 1) as f64)), ("fuel", g(f).to_json())])
                })
                .collect(),
        ),
    ));
    Ok(obj(out))
}
