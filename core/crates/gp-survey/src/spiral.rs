//! Spiral (clothoid) transitions (add-survey-suite, survey/alignment-curves,
//! "Spiral (clothoid) transitions"): the elements of a spiral-curve-spiral,
//! its TS, SC, CS, and ST stations, and deflections along the spiral.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point::plain_angle;
use libm::{atan2, cos, sin, tan};

use crate::{common_unit, fmt_station, is_metric, len, len_out, station};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 24 (spiral curves: elements, stationing, and deflections)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const ROW: &[Field] = &[
    Field::new(
        "station",
        "Station",
        "Along the spiral from the TS",
        Kind::Text { max_len: 16 },
    ),
    Field::new(
        "deflection",
        "Deflection from the TS",
        "atan(y / x) at this point",
        Kind::Text { max_len: 24 },
    ),
];

/// X and Y at spiral length l with angle θ = l²/(2R·Ls), each to four series terms.
fn xy(l: f64, th: f64) -> (f64, f64) {
    let (t2, t3) = (th * th, th * th * th);
    let x = l * (1.0 - t2 / 10.0 + t2 * t2 / 216.0 - t3 * t3 / 9360.0);
    let y = l * (th / 3.0 - t3 / 42.0 + t3 * t2 / 1320.0 - t3 * t2 * t2 / 75600.0);
    (x, y)
}

pub static SPIRAL: ToolDef = ToolDef {
    id: "survey.curves.spiral",
    title: "Spiral-curve-spiral transition",
    summary: "The elements of a clothoid spiral-curve-spiral (spiral angle, X, Y, p, k, long and short tangents, total tangent), its TS, SC, CS, and ST stations, and deflections along the spiral.",
    aliases: &[
        "spiral curve",
        "clothoid",
        "spiral transition",
        "spiral curve spiral",
    ],
    keywords: &[
        "spiral",
        "clothoid",
        "transition",
        "TS",
        "SC",
        "CS",
        "ST",
        "spiral angle",
        "throw",
        "highway curve",
    ],
    inputs: &[
        len("spiral_length", "Spiral length Ls", "Like 200 ft")
            .required()
            .core(),
        len("radius", "Circular curve radius R", "Like 1000 ft")
            .required()
            .core(),
        Field::new(
            "delta",
            "Total deflection Δ",
            "Of the whole spiral-curve-spiral, like 40°00'00\"",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded")
        .required()
        .core(),
        Field::new(
            "pi_station",
            "PI station",
            "Like 50+00",
            Kind::Text { max_len: 16 },
        )
        .required()
        .core(),
        len(
            "interval",
            "Deflection interval",
            "Along the spiral, like 50 ft",
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "spiral_angle",
            "Spiral angle θs",
            "Ls / 2R",
            Kind::Text { max_len: 24 },
        ),
        len_out("x", "X", "Tangent distance to the SC"),
        len_out("y", "Y", "Offset of the SC from the tangent"),
        len_out("p", "p (throw)", "Y − R(1 − cos θs)"),
        len_out("k", "k", "X − R sin θs"),
        len_out("long_tangent", "Long tangent", "X − Y cot θs"),
        len_out("short_tangent", "Short tangent", "Y / sin θs"),
        len_out("total_tangent", "Total tangent Ts", "(R + p) tan(Δ/2) + k"),
        len_out("arc_length", "Circular arc length", "R (Δ − 2θs)"),
        Field::new(
            "ts_station",
            "TS station",
            "PI − Ts",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "sc_station",
            "SC station",
            "TS + Ls",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "cs_station",
            "CS station",
            "SC + circular arc",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "st_station",
            "ST station",
            "CS + Ls",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "rows",
            "Spiral deflections",
            "At each interval from the TS to the SC",
            Kind::List {
                items: ROW,
                min: 0,
                max: 10_000,
            },
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Clothoid: θs = Ls/(2R); X = Ls(1 − θ²/10 + θ⁴/216 − θ⁶/9360), Y = Ls(θ/3 − θ³/42 + θ⁵/1320 − θ⁷/75600), four terms each; p = Y − R(1 − cos θs), k = X − R sin θs, Ts = (R + p) tan(Δ/2) + k, arc = R(Δ − 2θs) (Ghilani & Wolf 2021, ch. 24)",
    accuracy: "The four-term series is good to far better than 0.001 of the unit for spiral angles up to about 30°",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Ls = 200 ft, R = 1,000 ft, Δ = 40°, PI at 50+00",
        input: r#"{"spiral_length":"200 ft","radius":"1000 ft","delta":"40°00'00\"","pi_station":"50+00","interval":"50 ft"}"#,
        source: "add-survey-suite spiral-curve-spiral scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.curves.circular-curve",
            reason: "alternative",
        },
        Related {
            id: "survey.curves.curve-layout",
            reason: "next",
        },
    ],
    sentence: "The spiral runs TS {ts_station} to SC {sc_station}, the arc to CS {cs_station}, and the spiral out to ST {st_station}.",
    limits: &[("batchRows", 1_000)],
    run: run_spiral,
    ..ToolDef::BLANK
};

fn run_spiral(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ls = ctx.req_quantity("spiral_length")?;
    let r = ctx.req_quantity("radius")?;
    let step = ctx.quantity("interval")?;
    let mut qs = vec![("spiral_length", ls), ("radius", r)];
    qs.extend(step.map(|q| ("interval", q)));
    let u = common_unit(&qs)?;
    let (ls, r) = (ls.to(u), r.to(u));
    if ls <= 0.0 || r <= 0.0 {
        return Err(ToolError::invalid(
            "/spiral_length",
            "The spiral length and radius must be positive.",
        ));
    }
    let delta = plain_angle(ctx, "delta")?.expect("required").to_radians();
    let ths = ls / (2.0 * r);
    if !(delta > 2.0 * ths && delta < core::f64::consts::PI) {
        return Err(ToolError::invalid(
            "/delta",
            "The total deflection must exceed twice the spiral angle (Ls / R), and stay under 180°, to leave room for the circular arc.",
        ));
    }
    if ths > 30f64.to_radians() {
        return Err(ToolError::invalid(
            "/spiral_length",
            "The spiral angle is over 30°, beyond where the series is accurate: shorten the spiral or lengthen the radius.",
        ));
    }
    let pi = station(ctx, "pi_station")?.expect("required");
    let (x, y) = xy(ls, ths);
    let p = y - r * (1.0 - cos(ths));
    let k = x - r * sin(ths);
    let lt = x - y / tan(ths);
    let st = y / sin(ths);
    let ts = (r + p) * tan(delta / 2.0) + k;
    let arc = r * (delta - 2.0 * ths);
    let s_ts = pi - ts;
    let (s_sc, s_cs) = (s_ts + ls, s_ts + ls + arc);
    let s_st = s_cs + ls;
    let metric = is_metric(u);
    let q = |v: f64| Q { value: v, unit: u };
    let fmt_angle = |rad: f64| dms::format(rad.to_degrees(), Axis::Lon, Style::Dms, 1, false);
    let mut out = vec![
        ("spiral_angle", Json::Str(fmt_angle(ths))),
        ("x", ctx.emit("x", q(x), u)),
        ("y", ctx.emit("y", q(y), u)),
        ("p", ctx.emit("p", q(p), u)),
        ("k", ctx.emit("k", q(k), u)),
        ("long_tangent", ctx.emit("long_tangent", q(lt), u)),
        ("short_tangent", ctx.emit("short_tangent", q(st), u)),
        ("total_tangent", ctx.emit("total_tangent", q(ts), u)),
        ("arc_length", ctx.emit("arc_length", q(arc), u)),
        ("ts_station", Json::Str(fmt_station(s_ts, metric))),
        ("sc_station", Json::Str(fmt_station(s_sc, metric))),
        ("cs_station", Json::Str(fmt_station(s_cs, metric))),
        ("st_station", Json::Str(fmt_station(s_st, metric))),
    ];
    if let Some(step) = step.map(|s| s.to(u)) {
        if step <= 0.0 || ls / step > 10_000.0 {
            return Err(ToolError::invalid(
                "/interval",
                "The interval must be positive and give at most 10,000 rows.",
            ));
        }
        let mut ls_at = vec![0.0];
        let mut s = ((s_ts / step).floor() + 1.0) * step - s_ts;
        while s < ls - 1e-9 * step {
            if s > 1e-9 * step {
                ls_at.push(s);
            }
            s += step;
        }
        ls_at.push(ls);
        let rows = ls_at
            .iter()
            .map(|&l| {
                let (xi, yi) = xy(l, l * l / (2.0 * r * ls));
                let d = if l == 0.0 { 0.0 } else { atan2(yi, xi) };
                Json::obj(vec![
                    ("station", Json::Str(fmt_station(s_ts + l, metric))),
                    ("deflection", Json::Str(fmt_angle(d))),
                ])
            })
            .collect();
        out.push(("rows", Json::Arr(rows)));
    }
    Ok(Json::obj(out))
}
