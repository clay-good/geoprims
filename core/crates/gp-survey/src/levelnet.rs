//! Leveling network adjustment (add-survey-partition-and-adjustment,
//! "Leveling network adjustment"): height differences between benchmarks,
//! weighted by standard deviation, length, or setups, adjusted by least
//! squares with at least one fixed benchmark.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use libm::sqrt;

use crate::lsq::{chi2_inv, cholesky, solve};

const GHILANI_ADJ: Reference = Reference {
    title: "Adjustment Computations: Spatial Data Analysis",
    issuer: "Ghilani, C. D., Wiley",
    year: 2017,
    edition: "6th edition",
    locator: "The chapter on adjusting differential leveling networks by least squares",
    url: "https://www.wiley.com/en-us/Adjustment+Computations%3A+Spatial+Data+Analysis%2C+6th+Edition-p-9781119385981",
};

const MAX_UNKNOWN: usize = 500;
const MAX_OBS: usize = 2_000;
/// Networks this small show their observation and normal equations.
const SHOW_EQUATIONS: usize = 12;

const fn len(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}

const FIXED: &[Field] = &[
    Field::new("name", "Benchmark", "Like BM A", Kind::Text { max_len: 24 }).required(),
    len("elevation", "Elevation", "Like 100.000 m").required(),
];

const OBS: &[Field] = &[
    Field::new(
        "from",
        "From",
        "Benchmark, like BM A",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "to",
        "To",
        "Benchmark, like BM B",
        Kind::Text { max_len: 24 },
    )
    .required(),
    len("rise", "Height difference", "To minus from, like 5.123 m").required(),
    len(
        "length",
        "Length",
        "Of the run, like 2.4 km; weight 1 / length",
    ),
    Field::new(
        "setups",
        "Setups",
        "Number of setups, like 12; weight 1 / setups",
        Kind::Number {
            min: 1.0,
            max: 10_000.0,
        },
    ),
    len(
        "sd",
        "Standard deviation",
        "Of the difference, like 4 mm; weight 1 / sd²",
    ),
];

const OUT_BM: &[Field] = &[
    Field::new("name", "Benchmark", "", Kind::Text { max_len: 24 }),
    Field::new(
        "elevation",
        "Adjusted elevation",
        "",
        Kind::Text { max_len: 32 },
    ),
    Field::new(
        "standard_error",
        "Standard error",
        "",
        Kind::Text { max_len: 32 },
    ),
];

const OUT_RES: &[Field] = &[
    Field::new("observation", "Observation", "", Kind::Text { max_len: 64 }),
    Field::new(
        "residual",
        "Residual",
        "Adjusted − observed",
        Kind::Text { max_len: 32 },
    ),
    Field::new(
        "standardized",
        "Standardized residual",
        "",
        Kind::Text { max_len: 16 },
    ),
];

pub static LEVEL_NETWORK: ToolDef = ToolDef {
    id: "survey.reduction.level-network",
    title: "Leveling network adjustment",
    summary: "Adjusted elevations for a network of leveling runs between benchmarks, by weighted least squares from one or more fixed benchmarks, with standard errors, residuals, and the reference variance.",
    aliases: &[
        "level net adjustment",
        "least squares leveling",
        "differential leveling network",
    ],
    keywords: &[
        "leveling",
        "level",
        "network",
        "benchmark",
        "least",
        "squares",
        "elevation",
        "adjustment",
    ],
    inputs: &[
        Field::new(
            "fixed",
            "Fixed benchmarks",
            "Name and elevation, at least one, like BM1 at 100.000 m",
            Kind::List {
                items: FIXED,
                min: 1,
                max: 100,
            },
        )
        .required()
        .core(),
        Field::new(
            "observations",
            "Leveling runs",
            "From, to, rise, and a length, setups, or standard deviation, like BM1 to A, +1.234 m over 2 km",
            Kind::List {
                items: OBS,
                min: 1,
                max: MAX_OBS,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "adjusted",
            "Adjusted benchmarks",
            "Elevations with standard errors",
            Kind::List {
                items: OUT_BM,
                min: 0,
                max: MAX_UNKNOWN,
            },
        ),
        Field::new(
            "residuals",
            "Residuals",
            "Every run",
            Kind::List {
                items: OUT_RES,
                min: 0,
                max: MAX_OBS,
            },
        ),
        Field::new(
            "reference_variance",
            "Reference variance",
            "Weighted residual sum of squares over the degrees of freedom",
            Kind::Number {
                min: 0.0,
                max: 1e12,
            },
        )
        .precision(Precision::Significant(4))
        .optional(),
        Field::new(
            "degrees_of_freedom",
            "Degrees of freedom",
            "Runs − unknown benchmarks",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "test",
            "Chi-square test at 95%",
            "With standard deviations given",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "equations",
            "Equations",
            "Observation and normal equations, for up to 12 unknowns",
            Kind::Text { max_len: 6_000 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &[
        "ADJUSTMENT_TEST_FAILED",
        "NO_REDUNDANCY",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Observation equations H_to − H_from = Δh + v with weight 1/σ², 1/length, or 1/setups; normal equations (AᵀPA) x = AᵀPl solved by Cholesky; σ0² = vᵀPv / r; standard errors √(σ0² Q_xx); standardized residuals v / (σ0 √Q_vv); two-tailed 95% chi-square test of vᵀPv when standard deviations are given (Ghilani, Adjustment Computations)",
    accuracy: "Exact weighted least squares for the stated weights",
    when_to_use: "Use this when leveling runs tie several benchmarks into a network with loops or more runs than needed, and you want one consistent set of elevations. Enter the fixed benchmarks and each run's height difference with its length, number of setups, or standard deviation; it returns adjusted elevations with standard errors and shows which runs fit worst.",
    limitations: "Weights by length or setups give relative weights only, so the chi-square test runs only when every run has a standard deviation. Fixed benchmarks are held exactly. Every benchmark must connect to a fixed one through the runs. It adjusts height differences as given, without rod, curvature, or orthometric corrections.",
    references: &[GHILANI_ADJ],
    examples: &[Example {
        id: "primary",
        title: "A small net: one fixed benchmark, three new ones, five runs",
        input: r#"{"fixed":[{"name":"BM A","elevation":"100.000 m"}],"observations":[{"from":"BM A","to":"BM B","rise":"5.123 m","length":"2.0 km"},{"from":"BM B","to":"BM C","rise":"2.456 m","length":"1.5 km"},{"from":"BM C","to":"BM A","rise":"-7.570 m","length":"2.5 km"},{"from":"BM B","to":"BM D","rise":"-3.211 m","length":"1.2 km"},{"from":"BM D","to":"BM C","rise":"5.674 m","length":"1.8 km"}]}"#,
        source: "Constructed network with a 9 mm loop misclosure",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("value", "degrees_of_freedom")],
    }],
    related: &[
        Related {
            id: "survey.reduction.level-adjust",
            reason: "alternative",
        },
        Related {
            id: "survey.reduction.level-run",
            reason: "parent",
        },
        Related {
            id: "survey.cogo.least-squares-2d",
            reason: "alternative",
        },
    ],
    sentence: "Extra runs to check the net: {degrees_of_freedom}. Test: {test}.",
    limits: &[("batchRows", 1)],
    run: run_level_network,
    ..ToolDef::BLANK
};

fn run_level_network(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let km = units::by_symbol(QT::Length, "km").expect("km");
    let fmt = ctx.options.format;
    let mut fixed: Vec<(String, f64)> = vec![];
    for (i, r) in ctx.rows("fixed")?.iter().enumerate() {
        let name = ctx.row_text("fixed", i, r, "name")?.expect("required");
        let h = ctx
            .row_quantity("fixed", i, r, "elevation")?
            .expect("required")
            .to(m);
        if fixed.iter().any(|(n, _)| *n == name) {
            return Err(ToolError::invalid(
                &format!("/fixed/{i}/name"),
                format!("{name} is listed twice."),
            ));
        }
        fixed.push((name, h));
    }
    // (from, to, rise, weight, sd given)
    let mut obs: Vec<(String, String, f64, f64, bool)> = vec![];
    for (i, r) in ctx.rows("observations")?.iter().enumerate() {
        let from = ctx
            .row_text("observations", i, r, "from")?
            .expect("required");
        let to = ctx.row_text("observations", i, r, "to")?.expect("required");
        if from == to {
            return Err(ToolError::invalid(
                &format!("/observations/{i}/to"),
                "A run must join two different benchmarks.",
            ));
        }
        let rise = ctx
            .row_quantity("observations", i, r, "rise")?
            .expect("required")
            .to(m);
        let sd = ctx
            .row_quantity("observations", i, r, "sd")?
            .map(|q| q.to(m));
        let length = ctx
            .row_quantity("observations", i, r, "length")?
            .map(|q| q.to(km));
        let setups = match r.get("setups").filter(|v| !v.is_null()) {
            None => None,
            Some(v) => match v
                .as_f64()
                .or_else(|| v.as_str().and_then(|t| t.trim().parse().ok()))
            {
                Some(n) if (1.0..=10_000.0).contains(&n) => Some(n),
                _ => {
                    return Err(ToolError::invalid(
                        &format!("/observations/{i}/setups"),
                        "Setups is a count from 1 to 10,000.",
                    ));
                }
            },
        };
        let (w, has_sd) = match (sd, length, setups) {
            (Some(s), _, _) if s > 0.0 => (1.0 / (s * s), true),
            (Some(_), _, _) => {
                return Err(ToolError::invalid(
                    &format!("/observations/{i}/sd"),
                    "A standard deviation must be more than zero.",
                ));
            }
            (None, Some(l), _) if l > 0.0 => (1.0 / l, false),
            (None, Some(_), _) => {
                return Err(ToolError::invalid(
                    &format!("/observations/{i}/length"),
                    "A length must be more than zero.",
                ));
            }
            (None, None, Some(n)) => (1.0 / n, false),
            (None, None, None) => (1.0, false),
        };
        obs.push((from, to, rise, w, has_sd));
    }
    // Unknown benchmarks: every name in a run that is not fixed, in order of first use.
    let mut unknown: Vec<String> = vec![];
    for (f, t, ..) in &obs {
        for n in [f, t] {
            if !fixed.iter().any(|(k, _)| k == n) && !unknown.contains(n) {
                unknown.push(n.clone());
            }
        }
    }
    let u = unknown.len();
    if u == 0 {
        return Err(ToolError::invalid(
            "/observations",
            "Every benchmark in the runs is fixed: there is nothing to adjust.",
        ));
    }
    if u > MAX_UNKNOWN {
        return Err(ToolError::invalid(
            "/observations",
            format!("At most {MAX_UNKNOWN} unknown benchmarks."),
        ));
    }
    let idx = |n: &str| unknown.iter().position(|k| k == n);
    let fixed_h = |n: &str| fixed.iter().find(|(k, _)| k == n).map(|(_, h)| *h);
    // Each row: coefficients (+1 for to, −1 for from) and l = Δh − (fixed to) + (fixed from).
    let mut nmat = vec![0.0; u * u];
    let mut t = vec![0.0; u];
    let rows: Vec<(Option<usize>, Option<usize>, f64, f64)> = obs
        .iter()
        .map(|(f, to, rise, w, _)| {
            let (fi, ti) = (idx(f), idx(to));
            let l = rise - fixed_h(to).unwrap_or(0.0) + fixed_h(f).unwrap_or(0.0);
            (fi, ti, l, *w)
        })
        .collect();
    for &(fi, ti, l, w) in &rows {
        let terms: Vec<(usize, f64)> = [(ti, 1.0), (fi, -1.0)]
            .into_iter()
            .filter_map(|(k, c)| k.map(|k| (k, c)))
            .collect();
        for &(a, ca) in &terms {
            t[a] += w * ca * l;
            for &(b, cb) in &terms {
                nmat[a * u + b] += w * ca * cb;
            }
        }
    }
    let Some(l_fac) = cholesky(&nmat, u) else {
        return Err(ToolError::new(ErrorCode::NoSolution, "Some benchmarks do not connect to a fixed benchmark through the runs: tie each part of the network to a fixed one.").at("/observations"));
    };
    let x = solve(&l_fac, u, &t);
    // Q_xx columns, for standard errors and Q_vv.
    let mut q = vec![0.0; u * u];
    for c in 0..u {
        let mut e = vec![0.0; u];
        e[c] = 1.0;
        let col = solve(&l_fac, u, &e);
        for r in 0..u {
            q[r * u + c] = col[r];
        }
    }
    let n_obs = obs.len();
    let dof = n_obs.saturating_sub(u);
    let mut vtpv = 0.0;
    let mut v = vec![0.0; n_obs];
    for (k, &(fi, ti, l, w)) in rows.iter().enumerate() {
        let ax = ti.map_or(0.0, |i| x[i]) - fi.map_or(0.0, |i| x[i]);
        v[k] = ax - l;
        vtpv += w * v[k] * v[k];
    }
    let all_sd = obs.iter().all(|o| o.4);
    let s02 = if dof > 0 { vtpv / dof as f64 } else { 1.0 };
    // With standard deviations the a-priori variance is 1; otherwise the
    // weights are relative and σ0² scales them.
    let var_unit = if all_sd && dof == 0 { 1.0 } else { s02 };
    let mmf = |x: f64| display::number(x * 1000.0, Precision::Decimals(1), fmt);
    let mf = |x: f64| display::number(x, Precision::Decimals(4), fmt);
    let adjusted: Vec<Json> = unknown
        .iter()
        .enumerate()
        .map(|(i, n)| {
            Json::obj([
                ("name", Json::str(n.clone())),
                ("elevation", Json::str(format!("{} m", mf(x[i])))),
                (
                    "standard_error",
                    Json::str(format!("±{} mm", mmf(sqrt(var_unit * q[i * u + i])))),
                ),
            ])
        })
        .collect();
    let residuals: Vec<Json> = rows
        .iter()
        .zip(&obs)
        .enumerate()
        .map(|(k, (&(fi, ti, _, w), (f, to, ..)))| {
            let aqa = {
                let mut s = 0.0;
                for (a, ca) in [(ti, 1.0), (fi, -1.0)] {
                    for (b, cb) in [(ti, 1.0), (fi, -1.0)] {
                        if let (Some(a), Some(b)) = (a, b) {
                            s += ca * cb * q[a * u + b];
                        }
                    }
                }
                s
            };
            let qvv = 1.0 / w - aqa;
            let std = if dof > 0 && qvv > 1e-15 {
                format!("{:.2}", v[k] / sqrt(var_unit * qvv))
            } else {
                "—".to_owned()
            };
            Json::obj([
                ("observation", Json::str(format!("{f} to {to}"))),
                ("residual", Json::str(format!("{} mm", mmf(v[k])))),
                ("standardized", Json::str(std)),
            ])
        })
        .collect();
    let test = if dof == 0 {
        ctx.warnings.push(Warning::new("NO_REDUNDANCY", "The runs exactly fix the benchmarks, so there is no check on them: the residuals are zero."));
        "not possible without redundancy".to_owned()
    } else if !all_sd {
        "needs a standard deviation for every run".to_owned()
    } else {
        let (lo, hi) = (chi2_inv(0.025, dof as f64), chi2_inv(0.975, dof as f64));
        let pass = (lo..=hi).contains(&vtpv);
        let n3 = |v: f64| display::number(v, Precision::Decimals(3), fmt);
        if !pass {
            ctx.warnings.push(Warning::new(
                "ADJUSTMENT_TEST_FAILED",
                format!("The weighted residual sum {} is outside {} to {}: the standard deviations {} the fit.", n3(vtpv), n3(lo), n3(hi), if vtpv > hi { "are too small for" } else { "are too large for" }),
            ));
        }
        format!(
            "{} ({} within {} to {})",
            if pass { "passed" } else { "failed" },
            n3(vtpv),
            n3(lo),
            n3(hi)
        )
    };
    let mut out = vec![
        ("adjusted", Json::Arr(adjusted)),
        ("residuals", Json::Arr(residuals)),
    ];
    if dof > 0 {
        out.push(("reference_variance", Json::Num(s02)));
    }
    out.push(("degrees_of_freedom", Json::Num(dof as f64)));
    out.push(("test", Json::str(test)));
    if u <= SHOW_EQUATIONS {
        let name = |k: Option<usize>| k.map(|k| format!("H({})", unknown[k]));
        let mut lines = vec![
            "Observation equations (unknowns only; fixed elevations moved to the right):"
                .to_owned(),
        ];
        for (k, &(fi, ti, l, w)) in rows.iter().enumerate() {
            let lhs = match (name(ti), name(fi)) {
                (Some(a), Some(b)) => format!("{a} − {b}"),
                (Some(a), None) => a,
                (None, Some(b)) => format!("−{b}"),
                (None, None) => "0".to_owned(),
            };
            lines.push(format!(
                "{lhs} = {} m + v{} (weight {})",
                mf(l),
                k + 1,
                display::number(w, Precision::Significant(4), fmt)
            ));
        }
        lines.push("Normal equations N x = t:".to_owned());
        for r in 0..u {
            let row: Vec<String> = (0..u)
                .map(|c| display::number(nmat[r * u + c], Precision::Significant(4), fmt))
                .collect();
            lines.push(format!(
                "[{}] · x = {}",
                row.join(", "),
                display::number(t[r], Precision::Significant(6), fmt)
            ));
        }
        out.push(("equations", Json::str(lines.join("\n"))));
    }
    Ok(Json::obj(out))
}
