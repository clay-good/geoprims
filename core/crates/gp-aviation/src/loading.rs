//! Fuel and loading (aviation/fuel-and-loading spec): fuel weight from volume
//! with nominal densities, and weight and balance with a CG envelope check for
//! the takeoff and landing states. Every aircraft number comes from the user.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::{Quantity as QT, Unit};
use gp_base::{ErrorCode, display};
use serde_json::Value;

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

// ---------------------------------------------------------------- fuel weight

/// Nominal fuel densities (lb per US gallon), FAA-H-8083-1B chapter 2.
pub const FUELS: &[(&str, &str, f64)] = &[("100ll", "100LL avgas", 6.0), ("jet-a", "Jet A", 6.7)];

pub static FUEL_WEIGHT: ToolDef = ToolDef {
    id: "aviation.loading.fuel-weight",
    title: "Fuel weight",
    summary: "Fuel weight from volume, or volume from weight, for avgas or jet fuel at a nominal density or one you enter.",
    aliases: &[
        "fuel weight calculator",
        "gallons to pounds avgas",
        "Jet A weight",
    ],
    keywords: &[
        "fuel", "100LL", "avgas", "Jet A", "lb/gal", "weight", "gallons",
    ],
    inputs: &[
        qty("volume", "Fuel volume", "Like 40 gal", QT::Volume, "galUS").core(),
        qty(
            "weight",
            "Fuel weight",
            "Like 240 lb, to find the volume",
            QT::Mass,
            "lb",
        )
        .core(),
        Field::new(
            "fuel",
            "Fuel type",
            "100ll (6.0 lb/gal nominal) or jet-a (6.7 lb/gal nominal)",
            Kind::Choice(&["100ll", "jet-a"]),
        )
        .core(),
        qty(
            "density",
            "Density",
            "Your measured density, like 5.9 lb/gal; overrides the fuel type",
            QT::Density,
            "lb/galUS",
        )
        .core(),
    ],
    outputs: &[
        qty("weight", "Fuel weight", "Volume × density", QT::Mass, "lb")
            .precision(Precision::Decimals(1)),
        qty(
            "volume",
            "Fuel volume",
            "Weight / density",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "density",
            "Density used",
            "Nominal or entered",
            QT::Density,
            "lb/galUS",
        )
        .precision(Precision::Significant(3)),
        Field::new(
            "density_basis",
            "Density basis",
            "nominal or entered",
            Kind::Text { max_len: 40 },
        ),
    ],
    warnings: &["NOMINAL_VALUE_USED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Weight = volume × density",
    accuracy: "Exact for the density used. Real fuel density varies with temperature and batch by a few percent.",
    references: &[WB_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "40 US gal of 100LL",
        input: r#"{"volume":"40 gal","fuel":"100ll"}"#,
        source: "add-aviation-suite fuel scenario: 240 lb at the nominal 6.0 lb/gal (FAA-H-8083-1B chapter 2)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "aviation.loading.weight-balance",
        reason: "next",
    }],
    sentence: "{volume} of fuel weighs {weight}.{warn NOMINAL_VALUE_USED} This uses a nominal density.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_fuel_weight,
    ..ToolDef::BLANK
};

fn run_fuel_weight(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let vol = ctx.quantity("volume")?.map(|x| x.base());
    let wt = ctx.quantity("weight")?.map(|x| x.base());
    let fuel = ctx.choice("fuel")?;
    let (rho, basis) = match (ctx.quantity("density")?, fuel) {
        (Some(d), _) => (d.base(), "entered"),
        (None, Some(f)) => {
            let (_, name, lb_gal) = FUELS.iter().find(|(id, _, _)| *id == f).expect("choice");
            let d = Q {
                value: *lb_gal,
                unit: unit(QT::Density, "lb/galUS"),
            };
            ctx.warnings.push(Warning::new(
                "NOMINAL_VALUE_USED",
                format!(
                    "Uses the nominal {name} density of {} (FAA-H-8083-1B). Actual fuel varies with temperature; use your fuel's measured density for close loading.",
                    display::quantity(*lb_gal, "lb/gal", Precision::Significant(2), ctx.options.format)
                ),
            ));
            (d.base(), "nominal")
        }
        (None, None) => {
            return Err(
                ToolError::invalid("/fuel", "Choose a fuel type or enter a density.")
                    .hint("Example: 100ll"),
            );
        }
    };
    if rho <= 0.0 {
        return Err(ToolError::invalid("/density", "Density must be positive."));
    }
    let (v, w) = match (vol, wt) {
        (Some(v), None) => (v, v * rho),
        (None, Some(w)) => (w / rho, w),
        _ => {
            return Err(ToolError::invalid(
                "/volume",
                "Give either a fuel volume or a fuel weight.",
            ));
        }
    };
    if v < 0.0 {
        return Err(ToolError::invalid("/volume", "Fuel cannot be negative."));
    }
    let base = |q: QT, x: f64| Q {
        value: x,
        unit: gp_base::units::base_unit(q),
    };
    Ok(obj(vec![
        ("weight", ctx.out("weight", base(QT::Mass, w))),
        ("volume", ctx.out("volume", base(QT::Volume, v))),
        ("density", ctx.out("density", base(QT::Density, rho))),
        ("density_basis", Json::str(basis)),
    ]))
}

// ---------------------------------------------------------------- weight and balance

const STATION: &[Field] = &[
    Field::new(
        "name",
        "Station",
        "Like Front seats",
        Kind::Text { max_len: 40 },
    )
    .required(),
    qty("weight", "Weight", "Like 340 lb", QT::Mass, "lb").required(),
    qty("arm", "Arm", "Like 90 in aft of datum", QT::Length, "in").required(),
];

const ENVELOPE_POINT: &[Field] = &[
    qty("arm", "CG", "Like 82 in", QT::Length, "in").required(),
    qty("weight", "Weight", "Like 2300 lb", QT::Mass, "lb").required(),
];

pub static WEIGHT_BALANCE: ToolDef = ToolDef {
    id: "aviation.loading.weight-balance",
    stability: gp_base::tool::Stability::Stable,
    title: "Weight and balance",
    summary: "Total weight, moment, and center of gravity from your stations, checked against your CG envelope at takeoff and after the fuel burn.",
    aliases: &[
        "weight and balance calculator",
        "W&B calculator",
        "CG calculator",
        "center of gravity calculator",
    ],
    keywords: &[
        "W&B",
        "weight and balance",
        "CG",
        "center of gravity",
        "moment",
        "arm",
        "envelope",
        "MAC",
        "loading",
    ],
    inputs: &[
        Field::new(
            "stations",
            "Stations",
            "Name, weight, and arm for each station, from your aircraft's weight and balance data",
            Kind::List {
                items: STATION,
                min: 1,
                max: 50,
            },
        )
        .required()
        .core(),
        qty(
            "fuel_burn",
            "Fuel burn",
            "Like 60 lb, for the landing state",
            QT::Mass,
            "lb",
        )
        .core(),
        qty(
            "fuel_arm",
            "Fuel arm",
            "Like 48 in; default: the arm of the station named Fuel",
            QT::Length,
            "in",
        ),
        Field::new(
            "envelope",
            "CG envelope",
            "Corner points (CG, weight) of the envelope from the POH, in order around it",
            Kind::List {
                items: ENVELOPE_POINT,
                min: 3,
                max: 100,
            },
        )
        .core(),
        qty(
            "lemac",
            "LEMAC",
            "Leading edge of the mean aerodynamic chord, like 60 in",
            QT::Length,
            "in",
        ),
        qty(
            "mac",
            "MAC length",
            "Mean aerodynamic chord, like 58 in",
            QT::Length,
            "in",
        ),
    ],
    outputs: &[
        qty(
            "total_weight",
            "Takeoff weight",
            "Sum of station weights",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "total_moment",
            "Takeoff moment",
            "Sum of weight × arm, in the moment unit",
            Kind::Number {
                min: -1e15,
                max: 1e15,
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "moment_unit",
            "Moment unit",
            "Weight unit × arm unit",
            Kind::Text { max_len: 20 },
        ),
        qty("cg", "Takeoff CG", "Moment / weight", QT::Length, "in")
            .precision(Precision::Decimals(2)),
        Field::new(
            "cg_mac",
            "Takeoff CG, % MAC",
            "(CG − LEMAC) / MAC × 100",
            Kind::Number {
                min: -1e6,
                max: 1e6,
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "takeoff_status",
            "Takeoff in envelope",
            "inside or outside",
            Kind::Text { max_len: 12 },
        )
        .optional(),
        qty(
            "takeoff_cg_shift",
            "Takeoff CG shift to the envelope",
            "How far the CG must move at this weight (+ aft)",
            QT::Length,
            "in",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty(
            "takeoff_weight_change",
            "Takeoff weight change to the envelope",
            "How much weight to add (+) or remove (−) at this CG",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "landing_weight",
            "Landing weight",
            "Takeoff weight − fuel burn",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        qty(
            "landing_cg",
            "Landing CG",
            "After the fuel burn",
            QT::Length,
            "in",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "landing_cg_mac",
            "Landing CG, % MAC",
            "(CG − LEMAC) / MAC × 100",
            Kind::Number {
                min: -1e6,
                max: 1e6,
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "landing_status",
            "Landing in envelope",
            "inside or outside",
            Kind::Text { max_len: 12 },
        )
        .optional(),
        qty(
            "landing_cg_shift",
            "Landing CG shift to the envelope",
            "How far the CG must move at this weight (+ aft)",
            QT::Length,
            "in",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty(
            "landing_weight_change",
            "Landing weight change to the envelope",
            "How much weight to add (+) or remove (−) at this CG",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["OUTSIDE_CG_ENVELOPE", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "CG = Σ(weight × arm) / Σ weight; landing state removes the fuel burn at the fuel arm; envelope membership by the even-odd rule, with points on an edge counted inside",
    accuracy: "Exact arithmetic on your data. Your aircraft's POH/AFM and current weight and balance record govern.",
    references: &[WB_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "A four-seat single with full fuel",
        input: r#"{"stations":[{"name":"Empty","weight":"1500 lb","arm":"85 in"},{"name":"Front seats","weight":"340 lb","arm":"90 in"},{"name":"Rear seats","weight":"170 lb","arm":"118 in"},{"name":"Fuel","weight":"240 lb","arm":"48 in"}],"fuel_burn":"60 lb","envelope":[{"arm":"82 in","weight":"1500 lb"},{"arm":"93 in","weight":"1500 lb"},{"arm":"93 in","weight":"2300 lb"},{"arm":"84 in","weight":"2300 lb"},{"arm":"82 in","weight":"1950 lb"}]}"#,
        source: "add-aviation-suite weight-and-balance scenario: 2,250 lb at 84.30 in (FAA-H-8083-1B chapter 2 method)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "cg")],
    }],
    related: &[Related {
        id: "aviation.loading.fuel-weight",
        reason: "alternative",
    }],
    sentence: "The takeoff weight is {total_weight} with the CG at {cg}.{if landing_cg > 0} After the burn it is {landing_weight} at {landing_cg}.{/if}{warn OUTSIDE_CG_ENVELOPE} A point is outside the envelope.{/warn}",
    limits: &[("batchRows", 1_000)],
    run: run_weight_balance,
    ..ToolDef::BLANK
};

/// Even-odd point-in-polygon on (x = arm, y = weight); points on an edge count as inside.
pub fn inside(poly: &[(f64, f64)], p: (f64, f64)) -> bool {
    let n = poly.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        // On the segment?
        let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        let scale = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1.0);
        if cross.abs() <= 1e-9 * scale * scale
            && p.0 >= a.0.min(b.0) - 1e-9
            && p.0 <= a.0.max(b.0) + 1e-9
            && p.1 >= a.1.min(b.1) - 1e-9
            && p.1 <= a.1.max(b.1) + 1e-9
        {
            return true;
        }
        if (a.1 > p.1) != (b.1 > p.1) {
            let x = a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1);
            if p.0 < x {
                c = !c;
            }
        }
    }
    c
}

/// Where the envelope boundary crosses the line through `p` along one axis:
/// `axis` 0 is the horizontal line (same weight), 1 the vertical (same CG).
/// Returns the signed move from `p` to the nearest crossing.
pub fn nearest_along(poly: &[(f64, f64)], p: (f64, f64), axis: usize) -> Option<f64> {
    let (u, w) = if axis == 0 { (0, 1) } else { (1, 0) };
    let get = |q: (f64, f64), k: usize| if k == 0 { q.0 } else { q.1 };
    let n = poly.len();
    let mut best: Option<f64> = None;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (aw, bw) = (get(a, w), get(b, w));
        let pw = get(p, w);
        if (aw - pw) * (bw - pw) > 0.0 || aw == bw {
            continue;
        }
        let t = (pw - aw) / (bw - aw);
        let x = get(a, u) + t * (get(b, u) - get(a, u));
        let d = x - get(p, u);
        if best.is_none_or(|bd| d.abs() < bd.abs()) {
            best = Some(d);
        }
    }
    best
}

fn read_rows(
    ctx: &mut Ctx,
    list: &str,
    a: &str,
    b: &str,
) -> Result<Vec<(f64, f64, String)>, ToolError> {
    let rows = ctx.rows(list)?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, r) in rows.into_iter().enumerate() {
        let x = ctx.row_quantity(list, i, &r, a)?.expect("required").base();
        let y = ctx.row_quantity(list, i, &r, b)?.expect("required").base();
        let name = r
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        out.push((x, y, name));
    }
    Ok(out)
}

fn run_weight_balance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let stations = read_rows(ctx, "stations", "weight", "arm")?;
    let (w_unit, arm_unit): (&'static Unit, &'static Unit) =
        (ctx.output_unit("total_weight"), ctx.output_unit("cg"));
    // Work in the output units so the moment reads in familiar lb·in (or kg·m).
    let wv = |x: f64| {
        Q {
            value: x,
            unit: gp_base::units::base_unit(QT::Mass),
        }
        .to(w_unit)
    };
    let av = |x: f64| {
        Q {
            value: x,
            unit: gp_base::units::base_unit(QT::Length),
        }
        .to(arm_unit)
    };
    let mut w = 0.0;
    let mut mom = 0.0;
    let mut fuel_rows = Vec::new();
    for (i, (sw, sa, r)) in stations.iter().enumerate() {
        if *sw < 0.0 {
            return Err(ToolError::invalid(
                &format!("/stations/{i}/weight"),
                "Station weights cannot be negative.",
            ));
        }
        w += wv(*sw);
        mom += wv(*sw) * av(*sa);
        if r.to_ascii_lowercase().contains("fuel") {
            fuel_rows.push((i, *sw, *sa));
        }
    }
    if w <= 0.0 {
        return Err(ToolError::invalid(
            "/stations",
            "The total weight must be positive.",
        ));
    }
    let cg = mom / w;
    let envelope: Vec<(f64, f64)> = read_rows(ctx, "envelope", "arm", "weight")?
        .into_iter()
        .map(|(a, b, _)| (av(a), wv(b)))
        .collect();
    let lemac = ctx.quantity("lemac")?.map(|x| av(x.base()));
    let mac = ctx.quantity("mac")?.map(|x| av(x.base()));
    let pct = match (lemac, mac) {
        (Some(l), Some(c)) if c > 0.0 => Some(move |x: f64| (x - l) / c * 100.0),
        (None, None) => None,
        (_, Some(c)) if c <= 0.0 => {
            return Err(ToolError::invalid("/mac", "MAC length must be positive."));
        }
        _ => {
            return Err(ToolError::invalid(
                "/mac",
                "Give both LEMAC and MAC length for % MAC.",
            ));
        }
    };
    let wq = |x: f64| Q {
        value: x,
        unit: w_unit,
    };
    let aq = |x: f64| Q {
        value: x,
        unit: arm_unit,
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let m_unit = format!("{}*{}", w_unit.symbol, arm_unit.symbol);
        ctx.step(
            "Total weight",
            "W = the sum of every station's weight",
            format!(
                "{} stations summing to {} {}",
                stations.len(),
                n(w, 1),
                w_unit.symbol
            ),
            format!("{} {}", n(w, 1), w_unit.symbol),
        );
        ctx.step(
            "Total moment",
            "M = the sum of weight × arm for every station",
            stations
                .iter()
                .map(|(sw, sa, _)| format!("{} × {}", n(wv(*sw), 1), n(av(*sa), 1)))
                .collect::<Vec<_>>()
                .join(" + "),
            format!("{} {}", n(mom, 1), m_unit),
        );
        ctx.step(
            "Centre of gravity",
            "CG = M / W",
            format!("{} {} / {} {}", n(mom, 1), m_unit, n(w, 1), w_unit.symbol),
            format!("{} {}", n(cg, 2), arm_unit.symbol),
        );
    }
    let mut o = vec![
        ("total_weight", ctx.emit("total_weight", wq(w), w_unit)),
        ("total_moment", Json::Num(mom)),
        (
            "moment_unit",
            Json::str(format!("{}*{}", w_unit.symbol, arm_unit.symbol)),
        ),
        ("cg", ctx.emit("cg", aq(cg), arm_unit)),
    ];
    if let Some(f) = pct {
        o.push(("cg_mac", Json::Num(f(cg))));
    }
    let mut outside = Vec::new();
    let check = |ctx: &mut Ctx,
                 o: &mut Vec<(&'static str, Json)>,
                 p: (f64, f64),
                 names: [&'static str; 3],
                 label: &str,
                 outside: &mut Vec<String>| {
        if envelope.is_empty() {
            return;
        }
        let ok = inside(&envelope, p);
        o.push((names[0], Json::str(if ok { "inside" } else { "outside" })));
        if !ok {
            let dx = nearest_along(&envelope, p, 0);
            let dy = nearest_along(&envelope, p, 1);
            let mut parts = Vec::new();
            if let Some(dx) = dx {
                o.push((names[1], ctx.emit(names[1], aq(dx), arm_unit)));
                parts.push(format!(
                    "move the CG {} {}",
                    display::quantity(
                        dx.abs(),
                        arm_unit.symbol,
                        Precision::Decimals(2),
                        ctx.options.format
                    ),
                    if dx > 0.0 { "aft" } else { "forward" }
                ));
            }
            if let Some(dy) = dy {
                o.push((names[2], ctx.emit(names[2], wq(dy), w_unit)));
                parts.push(format!(
                    "{} {}",
                    if dy > 0.0 { "add" } else { "remove" },
                    display::quantity(
                        dy.abs(),
                        w_unit.symbol,
                        Precision::Decimals(1),
                        ctx.options.format
                    )
                ));
            }
            outside.push(format!(
                "The {label} point is outside the CG envelope{}.",
                if parts.is_empty() {
                    String::new()
                } else {
                    format!("; to reach its edge, {}", parts.join(" or "))
                }
            ));
        }
    };
    check(
        ctx,
        &mut o,
        (cg, w),
        [
            "takeoff_status",
            "takeoff_cg_shift",
            "takeoff_weight_change",
        ],
        "takeoff",
        &mut outside,
    );
    if let Some(burn) = ctx.quantity("fuel_burn")? {
        let burn_base = burn.base();
        if burn_base < 0.0 {
            return Err(ToolError::invalid(
                "/fuel_burn",
                "Fuel burn cannot be negative.",
            ));
        }
        let arm = match ctx.quantity("fuel_arm")? {
            Some(a) => a.base(),
            None => match fuel_rows.as_slice() {
                [(_, _, a)] => *a,
                [] => {
                    return Err(ToolError::invalid(
                        "/fuel_arm",
                        "Fuel arm is required: no station is named Fuel.",
                    )
                    .hint("Enter the fuel arm, like 48 in, or name the fuel station Fuel."));
                }
                _ => {
                    return Err(ToolError::invalid(
                        "/fuel_arm",
                        "Several stations are named Fuel; enter the arm of the fuel you burn.",
                    ));
                }
            },
        };
        if let [(i, fw, _)] = fuel_rows.as_slice()
            && ctx.quantity("fuel_arm")?.is_none()
            && burn_base > *fw
        {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The fuel burn is more than the fuel on board.",
            )
            .at(&format!("/stations/{i}/weight")));
        }
        let lw = w - wv(burn_base);
        if lw <= 0.0 {
            return Err(ToolError::invalid(
                "/fuel_burn",
                "The fuel burn is more than the takeoff weight.",
            ));
        }
        let lcg = (mom - wv(burn_base) * av(arm)) / lw;
        o.push(("landing_weight", ctx.emit("landing_weight", wq(lw), w_unit)));
        o.push(("landing_cg", ctx.emit("landing_cg", aq(lcg), arm_unit)));
        if let Some(f) = pct {
            o.push(("landing_cg_mac", Json::Num(f(lcg))));
        }
        check(
            ctx,
            &mut o,
            (lcg, lw),
            [
                "landing_status",
                "landing_cg_shift",
                "landing_weight_change",
            ],
            "landing",
            &mut outside,
        );
    }
    if !outside.is_empty() {
        ctx.warnings
            .push(Warning::new("OUTSIDE_CG_ENVELOPE", outside.join(" ")).at("/envelope"));
    }
    Ok(obj(o))
}
