//! Least-squares adjustment of a small horizontal network (add-survey-suite,
//! cogo-and-traverse "Least-squares adjustment (experimental)"): distances,
//! angles, and direction sets with standard deviations, adjusted by weighted
//! Gauss-Newton iteration (Ghilani and Wolf 2018, chapters 11, 16, and 19),
//! with standard errors, 95% error ellipses, the reference variance, and a
//! two-tailed chi-square test of it.

use std::collections::HashMap;

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{atan2, cos, exp, log, sin, sqrt};

use crate::{GHILANI, common_unit, direction, len, unit};

const MAX_UNKNOWN: usize = 200;
const MAX_OBS: usize = 2_000;
const ARCSEC: f64 = core::f64::consts::PI / 648_000.0;

const POINT: &[Field] = &[
    Field::new("name", "Point", "Like BM1", Kind::Text { max_len: 24 }).required(),
    len("northing", "Northing", "Like 5000.00").required(),
    len("easting", "Easting", "Like 5000.00").required(),
];
const DIST: &[Field] = &[
    Field::new(
        "from",
        "From",
        "Point name, like P1",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "to",
        "To",
        "Point name, like P1",
        Kind::Text { max_len: 24 },
    )
    .required(),
    len("distance", "Horizontal distance", "Like 400.12").required(),
    len("sd", "Standard deviation", "Like 0.01").required(),
];
const ANGLE: &[Field] = &[
    Field::new(
        "backsight",
        "Backsight",
        "Point name, like P1",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "station",
        "Station",
        "Where the angle was turned, like A",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "foresight",
        "Foresight",
        "Point name, like P1",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "angle",
        "Angle",
        "Clockwise from backsight to foresight, like 91-15-20",
        Kind::Text { max_len: 32 },
    )
    .required(),
    Field::new(
        "sd",
        "Standard deviation",
        "Like 5 arcsec",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
    .required(),
];
const DIRECTION: &[Field] = &[
    Field::new(
        "station",
        "Station",
        "Like A; each station's directions share one orientation",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "target",
        "Target",
        "Point name, like P1",
        Kind::Text { max_len: 24 },
    )
    .required(),
    Field::new(
        "direction",
        "Direction",
        "Circle reading, like 45-10-30",
        Kind::Text { max_len: 32 },
    )
    .required(),
    Field::new(
        "sd",
        "Standard deviation",
        "Like 3 arcsec",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
    .required(),
];

const OUT_POINT: &[Field] = &[
    Field::new("name", "Point", "Unknown point", Kind::Text { max_len: 24 }),
    len("northing", "Northing", "Adjusted").precision(Precision::Decimals(4)),
    len("easting", "Easting", "Adjusted").precision(Precision::Decimals(4)),
    len("sd_northing", "Northing standard error", "One sigma").precision(Precision::Decimals(4)),
    len("sd_easting", "Easting standard error", "One sigma").precision(Precision::Decimals(4)),
    len(
        "semi_major",
        "95% ellipse semi-major axis",
        "95% confidence",
    )
    .precision(Precision::Decimals(4)),
    len(
        "semi_minor",
        "95% ellipse semi-minor axis",
        "95% confidence",
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "orientation",
        "Ellipse orientation",
        "Of the semi-major axis, clockwise from north",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2))
    .angle_range("[0,180)"),
];
const OUT_RESIDUAL: &[Field] = &[
    Field::new(
        "observation",
        "Observation",
        "What was measured",
        Kind::Text { max_len: 80 },
    ),
    Field::new(
        "residual",
        "Residual",
        "Adjusted − observed",
        Kind::Text { max_len: 32 },
    ),
    Field::new(
        "standardized",
        "Standardized residual",
        "Residual / its own standard error",
        Kind::Number {
            min: -1e9,
            max: 1e9,
        },
    )
    .precision(Precision::Decimals(2)),
];

pub static LEAST_SQUARES: ToolDef = ToolDef {
    id: "survey.cogo.least-squares-2d",
    title: "Least-squares network adjustment (2D)",
    summary: "Adjusts a small horizontal network of distances, angles, and direction sets by weighted least squares, with adjusted coordinates, standard errors, 95% error ellipses, the reference variance, and a chi-square test.",
    aliases: &[
        "least squares adjustment",
        "network adjustment",
        "survey least squares",
        "error ellipse calculator",
    ],
    keywords: &[
        "least squares",
        "adjustment",
        "network",
        "error ellipse",
        "reference variance",
        "chi-square",
        "residuals",
    ],
    inputs: &[
        Field::new(
            "control",
            "Control points",
            "Held fixed: name, northing, easting, like BM1, 5000, 5000",
            Kind::List { items: POINT, min: 1, max: 500 },
        )
        .required()
        .core(),
        Field::new(
            "unknowns",
            "Points to adjust",
            "With approximate coordinates, like P1, 5400, 5100",
            Kind::List { items: POINT, min: 1, max: MAX_UNKNOWN },
        )
        .required()
        .core(),
        Field::new(
            "distances",
            "Distances",
            "From, to, horizontal distance, and standard deviation, like BM1, P1, 400.12, 0.01",
            Kind::List { items: DIST, min: 0, max: MAX_OBS },
        )
        .core(),
        Field::new(
            "angles",
            "Angles",
            "Backsight, station, foresight, angle, and standard deviation, like BM2, BM1, P1, 91-15-20, 5",
            Kind::List { items: ANGLE, min: 0, max: MAX_OBS },
        )
        .core(),
        Field::new(
            "directions",
            "Direction sets",
            "Station, target, circle reading, and standard deviation, like A, P1, 45-10-30, 3",
            Kind::List { items: DIRECTION, min: 0, max: MAX_OBS },
        ),
    ],
    outputs: &[
        Field::new(
            "reference_variance",
            "Reference variance",
            "Σ weighted residual² / degrees of freedom; about 1 when the standard deviations fit",
            Kind::Number { min: 0.0, max: 1e18 },
        )
        .precision(Precision::Decimals(4))
        .optional(),
        Field::new(
            "degrees_of_freedom",
            "Degrees of freedom",
            "Observations − unknowns",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "test",
            "Chi-square test (95%)",
            "passed, failed, or not possible without redundancy",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "chi_square",
            "Chi-square",
            "Σ weighted residual², against its 95% bounds",
            Kind::Number { min: 0.0, max: 1e18 },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "bounds",
            "95% bounds",
            "The chi-square values between which the test passes",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "iterations",
            "Iterations",
            "Until corrections fell below a micrometer",
            Kind::Number { min: 0.0, max: 100.0 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "adjusted",
            "Adjusted points",
            "Coordinates, standard errors, and 95% error ellipses",
            Kind::List { items: OUT_POINT, min: 0, max: MAX_UNKNOWN },
        ),
        Field::new(
            "residuals",
            "Residuals",
            "Every observation",
            Kind::List { items: OUT_RESIDUAL, min: 0, max: 3 * MAX_OBS },
        ),
        Field::new(
            "largest",
            "Largest standardized residuals",
            "The three observations that fit worst",
            Kind::Text { max_len: 300 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::DegenerateGeometry,
        ErrorCode::UnitMismatch,
        ErrorCode::NoSolution,
    ],
    warnings: &[
        "ADJUSTMENT_TEST_FAILED",
        "NO_REDUNDANCY",
        "LEGACY_UNIT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Weighted Gauss-Newton least squares (weights 1/σ²) on linearized distance, angle, and direction equations, each direction set with its own orientation unknown. Reference variance vᵀWv / r; two-tailed χ² test at 95%; 95% error ellipses scaled by √(2 F(0.05; 2, r)), or by 2.4477 without redundancy",
    accuracy: "Converges to the least-squares solution for well-conditioned networks with approximate coordinates near the truth. Experimental: check it against your adjustment software before relying on it",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "One new point from two control points",
        input: r#"{"control":[{"name":"A","northing":"1000 m","easting":"1000 m"},{"name":"B","northing":"1000 m","easting":"1400 m"}],"unknowns":[{"name":"P","northing":"1300 m","easting":"1200 m"}],"distances":[{"from":"A","to":"P","distance":"360.567 m","sd":"0.005 m"},{"from":"B","to":"P","distance":"360.551 m","sd":"0.005 m"}],"angles":[{"backsight":"B","station":"A","foresight":"P","angle":"303-41-26","sd":5},{"backsight":"P","station":"B","foresight":"A","angle":"303-41-20","sd":5}]}"#,
        source: "Weighted least squares after Ghilani and Wolf (2018), chapter 16",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "alternative",
        },
        Related {
            id: "survey.land.alta-rpp",
            reason: "next",
        },
    ],
    sentence: "The reference variance is {reference_variance}; the chi-square test {test}.",
    limits: &[("batchRows", 10)],
    run: run_lsq,
    ..ToolDef::BLANK
};

// ---------------------------------------------------------------- chi-square

/// ln Γ(x) by Lanczos (g = 7, 9 terms).
fn ln_gamma(x: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let t = x + 7.5;
    let s = C[0] + (1..9).map(|i| C[i] / (x + i as f64)).sum::<f64>();
    0.5 * log(2.0 * core::f64::consts::PI) + (x + 0.5) * log(t) - t + log(s)
}

/// The regularized lower incomplete gamma P(a, x).
fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let lead = exp(-x + a * log(x) - ln_gamma(a));
    if x < a + 1.0 {
        let (mut sum, mut term, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..1000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        sum * lead
    } else {
        // Lentz's continued fraction for Q(a, x).
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..1000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        1.0 - lead * h
    }
}

/// The chi-square value with `k` degrees of freedom below which lies `p`.
pub fn chi2_inv(p: f64, k: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, k + 20.0 * sqrt(k) + 50.0);
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if gamma_p(k / 2.0, mid / 2.0) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

// ---------------------------------------------------------------- linear algebra

/// Cholesky factor (lower) of the symmetric `n`×`n` matrix `a`, or None if it
/// is not positive definite.
fn cholesky(a: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0; n * n];
    let scale = (0..n)
        .map(|i| a[i * n + i].abs())
        .fold(0.0, f64::max)
        .max(1e-300);
    for i in 0..n {
        for j in 0..=i {
            let s = a[i * n + j] - (0..j).map(|k| l[i * n + k] * l[j * n + k]).sum::<f64>();
            if i == j {
                if s <= 1e-12 * scale {
                    return None;
                }
                l[i * n + i] = sqrt(s);
            } else {
                l[i * n + j] = s / l[j * n + j];
            }
        }
    }
    Some(l)
}

fn solve(l: &[f64], n: usize, b: &[f64]) -> Vec<f64> {
    let mut y = vec![0.0; n];
    for i in 0..n {
        y[i] = (b[i] - (0..i).map(|k| l[i * n + k] * y[k]).sum::<f64>()) / l[i * n + i];
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        x[i] = (y[i] - (i + 1..n).map(|k| l[k * n + i] * x[k]).sum::<f64>()) / l[i * n + i];
    }
    x
}

// ---------------------------------------------------------------- the adjustment

#[derive(Clone, Copy)]
enum Where {
    Fixed(f64, f64),
    Free(usize),
}

enum Obs {
    Dist {
        i: usize,
        j: usize,
        value: f64,
        sd: f64,
    },
    Angle {
        b: usize,
        s: usize,
        f: usize,
        value: f64,
        sd: f64,
    },
    Dir {
        s: usize,
        t: usize,
        set: usize,
        value: f64,
        sd: f64,
    },
}

fn wrap(a: f64) -> f64 {
    let t = core::f64::consts::TAU;
    let r = (a + core::f64::consts::PI).rem_euclid(t) - core::f64::consts::PI;
    if r <= -core::f64::consts::PI {
        r + t
    } else {
        r
    }
}

fn run_lsq(ctx: &mut Ctx) -> Result<Json, ToolError> {
    // Points: name → where, and the unknowns' names in order.
    let mut pts: HashMap<String, Where> = HashMap::new();
    let mut free_names: Vec<String> = Vec::new();
    let mut start: Vec<f64> = Vec::new(); // E, N per free point
    let mut lengths: Vec<(String, Q)> = Vec::new();
    for list in ["control", "unknowns"] {
        let rows = ctx.rows(list)?;
        for (i, r) in rows.iter().enumerate() {
            let name = ctx.row_text(list, i, r, "name")?.expect("required");
            let n = ctx.row_quantity(list, i, r, "northing")?.expect("required");
            let e = ctx.row_quantity(list, i, r, "easting")?.expect("required");
            lengths.push((format!("/{list}/{i}/northing"), n));
            lengths.push((format!("/{list}/{i}/easting"), e));
            if pts.contains_key(&name) {
                return Err(ToolError::invalid(
                    &format!("/{list}/{i}/name"),
                    format!("Point {name} is listed twice."),
                ));
            }
            let w = if list == "control" {
                Where::Fixed(e.base(), n.base())
            } else {
                free_names.push(name.clone());
                start.push(e.base());
                start.push(n.base());
                Where::Free(free_names.len() - 1)
            };
            pts.insert(name, w);
        }
    }
    let find = |name: &str, at: String| -> Result<usize, ToolError> {
        Err(ToolError::invalid(
            &at,
            format!("{name} is not a control point or a point to adjust."),
        ))
    };
    // Points referenced by observations get an index into `slots`.
    let mut slot_of: HashMap<String, usize> = HashMap::new();
    let mut slots: Vec<Where> = Vec::new();
    let mut slot = |name: &str, at: String, slots: &mut Vec<Where>| -> Result<usize, ToolError> {
        if let Some(&k) = slot_of.get(name) {
            return Ok(k);
        }
        match pts.get(name) {
            Some(&w) => {
                slots.push(w);
                slot_of.insert(name.to_owned(), slots.len() - 1);
                Ok(slots.len() - 1)
            }
            None => find(name, at),
        }
    };
    let mut obs: Vec<Obs> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let rows = ctx.rows("distances")?;
    for (i, r) in rows.iter().enumerate() {
        let from = ctx.row_text("distances", i, r, "from")?.expect("required");
        let to = ctx.row_text("distances", i, r, "to")?.expect("required");
        let d = ctx
            .row_quantity("distances", i, r, "distance")?
            .expect("required");
        let sd = ctx
            .row_quantity("distances", i, r, "sd")?
            .expect("required");
        lengths.push((format!("/distances/{i}/distance"), d));
        if sd.value <= 0.0 || d.value <= 0.0 {
            return Err(ToolError::invalid(
                &format!("/distances/{i}"),
                "Distances and their standard deviations must be positive.",
            ));
        }
        let (a, b) = (
            slot(&from, format!("/distances/{i}/from"), &mut slots)?,
            slot(&to, format!("/distances/{i}/to"), &mut slots)?,
        );
        obs.push(Obs::Dist {
            i: a,
            j: b,
            value: d.base(),
            sd: sd.base(),
        });
        labels.push(format!("distance {from}–{to}"));
    }
    let rows = ctx.rows("angles")?;
    for (i, r) in rows.iter().enumerate() {
        let bs = ctx
            .row_text("angles", i, r, "backsight")?
            .expect("required");
        let st = ctx.row_text("angles", i, r, "station")?.expect("required");
        let fs = ctx
            .row_text("angles", i, r, "foresight")?
            .expect("required");
        let text = ctx.row_text("angles", i, r, "angle")?.expect("required");
        let value = direction::parse(&text)
            .map_err(|m| ToolError::invalid(&format!("/angles/{i}/angle"), m))?;
        let sd = ctx
            .row_quantity("angles", i, r, "sd")?
            .expect("required")
            .to(unit(QT::Angle, "arcsec"));
        if sd.is_nan() || sd <= 0.0 {
            return Err(ToolError::invalid(
                &format!("/angles/{i}/sd"),
                "The standard deviation must be a positive number of arc seconds.",
            ));
        }
        let (b, s, f) = (
            slot(&bs, format!("/angles/{i}/backsight"), &mut slots)?,
            slot(&st, format!("/angles/{i}/station"), &mut slots)?,
            slot(&fs, format!("/angles/{i}/foresight"), &mut slots)?,
        );
        obs.push(Obs::Angle {
            b,
            s,
            f,
            value: value.to_radians(),
            sd: sd * ARCSEC,
        });
        labels.push(format!("angle {bs}–{st}–{fs}"));
    }
    let rows = ctx.rows("directions")?;
    let mut sets: Vec<String> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let st = ctx
            .row_text("directions", i, r, "station")?
            .expect("required");
        let tg = ctx
            .row_text("directions", i, r, "target")?
            .expect("required");
        let text = ctx
            .row_text("directions", i, r, "direction")?
            .expect("required");
        let value = direction::parse(&text)
            .map_err(|m| ToolError::invalid(&format!("/directions/{i}/direction"), m))?;
        let sd = ctx
            .row_quantity("directions", i, r, "sd")?
            .expect("required")
            .to(unit(QT::Angle, "arcsec"));
        if sd.is_nan() || sd <= 0.0 {
            return Err(ToolError::invalid(
                &format!("/directions/{i}/sd"),
                "The standard deviation must be a positive number of arc seconds.",
            ));
        }
        let set = match sets.iter().position(|s| *s == st) {
            Some(k) => k,
            None => {
                sets.push(st.clone());
                sets.len() - 1
            }
        };
        let (s, t) = (
            slot(&st, format!("/directions/{i}/station"), &mut slots)?,
            slot(&tg, format!("/directions/{i}/target"), &mut slots)?,
        );
        obs.push(Obs::Dir {
            s,
            t,
            set,
            value: value.to_radians(),
            sd: sd * ARCSEC,
        });
        labels.push(format!("direction {st}→{tg}"));
    }
    let u_pts = free_names.len();
    let u = 2 * u_pts + sets.len();
    let m = obs.len();
    if m == 0 {
        return Err(ToolError::invalid(
            "/distances",
            "Give some distances, angles, or directions.",
        ));
    }
    let lu = common_unit(
        &lengths
            .iter()
            .map(|(p, q)| (p.as_str(), *q))
            .collect::<Vec<_>>(),
    )?;
    if m < u {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            format!("{m} observations cannot fix {u} unknowns (2 per point to adjust, and 1 per direction set)."),
        )
        .at("/unknowns"));
    }
    // Parameters: E, N of each free point, then one orientation per set.
    let mut x = start.clone();
    x.extend(sets.iter().map(|_| 0.0));
    // Each direction set's orientation starts from its first reading.
    for o in &obs {
        if let Obs::Dir {
            s, t, set, value, ..
        } = *o
            && x[2 * u_pts + set] == 0.0
        {
            let (ps, pt) = (pos(&slots, &x, s), pos(&slots, &x, t));
            x[2 * u_pts + set] = wrap(atan2(pt.0 - ps.0, pt.1 - ps.1) - value);
        }
    }
    let mut iterations = 0;
    let mut lfac: Vec<f64> = Vec::new();
    for it in 1..=25 {
        iterations = it;
        let (a_rows, l) = design(&obs, &slots, &x, u_pts);
        let mut nmat = vec![0.0; u * u];
        let mut t = vec![0.0; u];
        for (k, row) in a_rows.iter().enumerate() {
            let w = 1.0 / sd_of(&obs[k]).powi(2);
            for &(p, ap) in row {
                t[p] += ap * w * l[k];
                for &(q, aq) in row {
                    nmat[p * u + q] += ap * w * aq;
                }
            }
        }
        let Some(fac) = cholesky(&nmat, u) else {
            return Err(ToolError::new(
                ErrorCode::DegenerateGeometry,
                "The observations do not fix every point to adjust: each needs enough distances, angles, or directions to pin it down.",
            )
            .at("/unknowns"));
        };
        let dx = solve(&fac, u, &t);
        for (xi, d) in x.iter_mut().zip(&dx) {
            *xi += d;
        }
        lfac = fac;
        let biggest = dx[..2 * u_pts].iter().fold(0.0f64, |a, v| a.max(v.abs()));
        if biggest < 1e-6 && dx[2 * u_pts..].iter().all(|v| v.abs() < 1e-11) {
            break;
        }
        if it == 25 {
            return Err(ToolError::new(
                ErrorCode::NoSolution,
                "The adjustment did not converge in 25 iterations; check the approximate coordinates and the observations.",
            )
            .at("/unknowns"));
        }
    }
    // Residuals at the solution: computed − observed.
    let (a_rows, l) = design(&obs, &slots, &x, u_pts);
    let v: Vec<f64> = l.iter().map(|x| -x).collect();
    let vtwv: f64 = v
        .iter()
        .zip(&obs)
        .map(|(v, o)| v * v / sd_of(o).powi(2))
        .sum();
    let r = m - u;
    let s02 = if r > 0 { vtwv / r as f64 } else { f64::NAN };
    // Qxx = N⁻¹, column by column.
    let q = |i: usize, j: usize, cols: &HashMap<usize, Vec<f64>>| cols[&j][i];
    let mut cols: HashMap<usize, Vec<f64>> = HashMap::new();
    for j in 0..u {
        let mut e = vec![0.0; u];
        e[j] = 1.0;
        cols.insert(j, solve(&lfac, u, &e));
    }
    let fmt = ctx.options.format;
    let sigma0 = if r > 0 { sqrt(s02) } else { 1.0 };
    let ellipse_k = if r > 0 {
        sqrt(2.0 * (r as f64 / 2.0) * (libm::pow(0.05, -2.0 / r as f64) - 1.0))
    } else {
        sqrt(chi2_inv(0.95, 2.0))
    };
    let emit = |ctx: &mut Ctx, name: &str, v: f64| {
        ctx.emit(
            name,
            Q {
                value: v,
                unit: unit(QT::Length, "m"),
            },
            lu,
        )
    };
    let mut adjusted = Vec::new();
    for (k, name) in free_names.iter().enumerate() {
        let (ix, iy) = (2 * k, 2 * k + 1);
        let (qxx, qyy, qxy) = (q(ix, ix, &cols), q(iy, iy, &cols), q(ix, iy, &cols));
        let t = 0.5 * atan2(2.0 * qxy, qyy - qxx);
        let (ct, st) = (cos(t), sin(t));
        let su2 = qyy * ct * ct + 2.0 * qxy * ct * st + qxx * st * st;
        let sv2 = qxx * ct * ct - 2.0 * qxy * ct * st + qyy * st * st;
        let (major, minor, mut orient) = if su2 >= sv2 {
            (sqrt(su2.max(0.0)), sqrt(sv2.max(0.0)), t.to_degrees())
        } else {
            (
                sqrt(sv2.max(0.0)),
                sqrt(su2.max(0.0)),
                t.to_degrees() + 90.0,
            )
        };
        orient = orient.rem_euclid(180.0);
        let row = Json::obj([
            ("name", Json::str(name.clone())),
            ("northing", emit(ctx, "northing", x[iy])),
            ("easting", emit(ctx, "easting", x[ix])),
            ("sd_northing", emit(ctx, "sd_northing", sigma0 * sqrt(qyy))),
            ("sd_easting", emit(ctx, "sd_easting", sigma0 * sqrt(qxx))),
            (
                "semi_major",
                emit(ctx, "semi_major", ellipse_k * sigma0 * major),
            ),
            (
                "semi_minor",
                emit(ctx, "semi_minor", ellipse_k * sigma0 * minor),
            ),
            (
                "orientation",
                Q {
                    value: orient,
                    unit: unit(QT::Angle, "deg"),
                }
                .to_json(),
            ),
        ]);
        adjusted.push(row);
    }
    // Standardized residuals from Qvv = W⁻¹ − A Qxx Aᵀ.
    let mut std_res: Vec<(f64, usize)> = Vec::new();
    let mut residuals = Vec::new();
    for (k, row) in a_rows.iter().enumerate() {
        let mut aqa = 0.0;
        for &(p, ap) in row {
            for &(pp, aq) in row {
                aqa += ap * q(p, pp, &cols) * aq;
            }
        }
        let qvv = sd_of(&obs[k]).powi(2) - aqa;
        let z = if qvv > 1e-15 * sd_of(&obs[k]).powi(2) {
            v[k] / (sigma0 * sqrt(qvv))
        } else {
            0.0
        };
        if qvv > 1e-15 * sd_of(&obs[k]).powi(2) {
            std_res.push((z, k));
        }
        let text = match obs[k] {
            Obs::Dist { .. } => display::quantity(
                Q {
                    value: v[k],
                    unit: unit(QT::Length, "m"),
                }
                .to(lu),
                lu.symbol,
                Precision::Decimals(4),
                fmt,
            ),
            _ => format!(
                "{}″",
                display::number(v[k] / ARCSEC, Precision::Decimals(1), fmt)
            ),
        };
        residuals.push(Json::obj([
            ("observation", Json::str(labels[k].clone())),
            ("residual", Json::str(text)),
            ("standardized", Json::Num(z)),
        ]));
    }
    std_res.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
    let largest = std_res
        .iter()
        .take(3)
        .map(|&(z, k)| {
            format!(
                "{} ({})",
                labels[k],
                display::number(z, Precision::Decimals(2), fmt)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let (test, bounds) = if r == 0 {
        ctx.warnings.push(Warning::new(
            "NO_REDUNDANCY",
            "The observations exactly fix the unknowns, so there is no check on them: the residuals are zero and the ellipses use the stated standard deviations.",
        ));
        (
            "not possible without redundancy".to_owned(),
            "none".to_owned(),
        )
    } else {
        let (lo, hi) = (chi2_inv(0.025, r as f64), chi2_inv(0.975, r as f64));
        let n = |v: f64| display::number(v, Precision::Decimals(3), fmt);
        let pass = (lo..=hi).contains(&vtwv);
        if !pass {
            ctx.warnings.push(Warning::new(
                "ADJUSTMENT_TEST_FAILED",
                format!(
                    "The reference variance {} fails the 95% chi-square test ({} is outside {} to {}): the standard deviations {} the fit. Largest standardized residuals: {largest}.",
                    display::number(s02, Precision::Decimals(3), fmt),
                    n(vtwv),
                    n(lo),
                    n(hi),
                    if vtwv > hi { "are too small for" } else { "are too large for" }
                ),
            ));
        }
        (
            if pass { "passed" } else { "failed" }.to_owned(),
            format!("{} to {}", n(lo), n(hi)),
        )
    };
    if ctx.explaining() {
        ctx.step(
            "Degrees of freedom",
            "observations − unknowns",
            format!("{m} − {u}"),
            format!("{r}"),
        );
        ctx.step(
            "Reference variance",
            "Σ v²/σ² ÷ degrees of freedom",
            format!(
                "{} ÷ {r}",
                display::number(vtwv, Precision::Decimals(4), fmt)
            ),
            if r > 0 {
                display::number(s02, Precision::Decimals(4), fmt)
            } else {
                "none".to_owned()
            },
        );
    }
    let mut out = vec![];
    if r > 0 {
        out.push(("reference_variance", Json::Num(s02)));
    }
    out.extend([
        ("degrees_of_freedom", Json::Num(r as f64)),
        ("test", Json::str(test)),
        ("chi_square", Json::Num(vtwv)),
        ("bounds", Json::str(bounds)),
        ("iterations", Json::Num(iterations as f64)),
        ("adjusted", Json::Arr(adjusted)),
        ("residuals", Json::Arr(residuals)),
        ("largest", Json::str(largest)),
    ]);
    Ok(Json::obj(out))
}

fn sd_of(o: &Obs) -> f64 {
    match *o {
        Obs::Dist { sd, .. } | Obs::Angle { sd, .. } | Obs::Dir { sd, .. } => sd,
    }
}

/// (E, N) of slot `k` at parameters `x`.
fn pos(slots: &[Where], x: &[f64], k: usize) -> (f64, f64) {
    match slots[k] {
        Where::Fixed(e, n) => (e, n),
        Where::Free(i) => (x[2 * i], x[2 * i + 1]),
    }
}

/// Sparse design rows (parameter, partial) and misclosures observed − computed.
fn design(
    obs: &[Obs],
    slots: &[Where],
    x: &[f64],
    u_pts: usize,
) -> (Vec<Vec<(usize, f64)>>, Vec<f64>) {
    let free = |k: usize| match slots[k] {
        Where::Free(i) => Some(i),
        Where::Fixed(..) => None,
    };
    // Azimuth from a to b and its partials (dAz/dEa, dAz/dNa); the b partials are their negatives.
    let az = |a: usize, b: usize| {
        let (pa, pb) = (pos(slots, x, a), pos(slots, x, b));
        let (de, dn) = (pb.0 - pa.0, pb.1 - pa.1);
        let d2 = de * de + dn * dn;
        (atan2(de, dn), -dn / d2, de / d2)
    };
    let mut rows = Vec::with_capacity(obs.len());
    let mut l = Vec::with_capacity(obs.len());
    for o in obs {
        let mut row: Vec<(usize, f64)> = Vec::new();
        let put = |k: usize, de: f64, dn: f64, row: &mut Vec<(usize, f64)>| {
            if let Some(i) = free(k) {
                row.push((2 * i, de));
                row.push((2 * i + 1, dn));
            }
        };
        match *o {
            Obs::Dist { i, j, value, .. } => {
                let (pa, pb) = (pos(slots, x, i), pos(slots, x, j));
                let (de, dn) = (pb.0 - pa.0, pb.1 - pa.1);
                let d = sqrt(de * de + dn * dn);
                put(i, -de / d, -dn / d, &mut row);
                put(j, de / d, dn / d, &mut row);
                l.push(value - d);
            }
            Obs::Angle { b, s, f, value, .. } => {
                let (azf, fe, fnn) = az(s, f);
                let (azb, be, bn) = az(s, b);
                put(s, fe - be, fnn - bn, &mut row);
                put(f, -fe, -fnn, &mut row);
                put(b, be, bn, &mut row);
                l.push(wrap(value - (azf - azb)));
            }
            Obs::Dir {
                s, t, set, value, ..
            } => {
                let (a, ae, an) = az(s, t);
                put(s, ae, an, &mut row);
                put(t, -ae, -an, &mut row);
                row.push((2 * u_pts + set, -1.0));
                l.push(wrap(value - (a - x[2 * u_pts + set])));
            }
        }
        // Merge repeated parameters (a point seen twice in one observation).
        row.sort_by_key(|p| p.0);
        let mut merged: Vec<(usize, f64)> = Vec::with_capacity(row.len());
        for (p, v) in row {
            match merged.last_mut() {
                Some(last) if last.0 == p => last.1 += v,
                _ => merged.push((p, v)),
            }
        }
        rows.push(merged);
    }
    (rows, l)
}
