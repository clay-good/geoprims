//! Differential leveling (add-survey-suite, survey/instrument-reductions,
//! "Differential leveling"): a level book reduced to heights of instrument and
//! elevations, the arithmetic check, and for a loop or a run between
//! benchmarks, the misclosure, its allowable, and its adjustment.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::sqrt;
use serde_json::{Map, Value};

use crate::{common_unit, len, len_out, unit};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapter 5 (leveling field procedures and computations: level notes, arithmetic check, misclosure, and adjustment)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003237",
};

const SHOT: &[Field] = &[
    Field::new(
        "station",
        "Station",
        "Benchmark or turning point, like BM 1 or TP 1",
        Kind::Text { max_len: 24 },
    )
    .required(),
    len("backsight", "Backsight", "Plus sight, like 4.52 ft"),
    len("foresight", "Foresight", "Minus sight, like 3.97 ft"),
    len(
        "intermediate",
        "Intermediate sight",
        "A side shot that does not carry the run, like 2.35 ft",
    ),
    len(
        "distance",
        "Distance",
        "From the previous station, for the adjustment, like 300 ft",
    ),
];

const ROW: &[Field] = &[
    Field::new(
        "station",
        "Station",
        "As entered",
        Kind::Text { max_len: 24 },
    ),
    len_out("hi", "Height of instrument", "Elevation plus the backsight").optional(),
    len_out("elevation", "Elevation", "As observed"),
    len_out(
        "adjusted",
        "Adjusted elevation",
        "With the misclosure distributed",
    )
    .optional(),
];

pub static LEVEL_RUN: ToolDef = ToolDef {
    id: "survey.reduction.level-run",
    title: "Level run reduction and closure",
    summary: "Heights of instrument and elevations from a level book, the arithmetic check, and for a loop or a run between benchmarks, the misclosure, its allowable, and the adjusted elevations.",
    aliases: &["level loop", "differential leveling", "level notes", "benchmark run"],
    keywords: &["leveling", "level run", "backsight", "foresight", "turning point", "benchmark", "misclosure", "HI", "arithmetic check", "loop"],
    inputs: &[
        len("start_elevation", "Starting elevation", "Of the first benchmark, like 100.00 ft").required().core(),
        Field::new("shots", "Level book", "One row per station, in order, like BM 1 with a 4.52 ft backsight, then TP 1 with a 3.97 ft foresight and a 6.13 ft backsight", Kind::List { items: SHOT, min: 2, max: 2000 })
            .required()
            .core(),
        len("end_elevation", "Closing benchmark elevation", "Known elevation of the last station, like 247.975 m; a loop back to the start closes on the starting elevation").core(),
        len("allowable_constant", "Allowable constant C", "Allowable closure is C × √K, K in kilometers, like 12 mm"),
    ],
    outputs: &[
        Field::new("stations", "Stations", "Each station reduced", Kind::List { items: ROW, min: 1, max: 2000 }),
        len_out("sum_backsights", "Sum of backsights", "ΣBS"),
        len_out("sum_foresights", "Sum of foresights", "ΣFS"),
        Field::new("check", "Arithmetic check", "ΣBS − ΣFS against the last elevation less the first", Kind::Text { max_len: 120 }),
        len_out("misclosure", "Misclosure", "Observed closing elevation minus the known one").optional(),
        len_out("allowable", "Allowable closure", "C × √K").optional(),
        Field::new("closure", "Closure", "Within or beyond the allowable", Kind::Text { max_len: 60 }).optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "HI = elevation + BS; elevation = HI − FS (or − IS for a side shot); check ΣBS − ΣFS = last − first elevation; misclosure = observed − known closing elevation, distributed in proportion to cumulative distance (or to setups when no distances are given); allowable = C·√K (Ghilani & Wolf 2018, ch. 5)",
    accuracy: "Exact arithmetic; the allowable closure is the standard you choose, entered as C",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A loop from BM 1 through two turning points and back",
        input: r#"{"start_elevation":"100.00 ft","shots":[{"station":"BM 1","backsight":"4.52 ft"},{"station":"TP 1","foresight":"3.97 ft","backsight":"6.13 ft","distance":"300 ft"},{"station":"TP 2","foresight":"5.26 ft","backsight":"2.84 ft","distance":"280 ft"},{"station":"BM 1","foresight":"4.28 ft","distance":"310 ft"}],"allowable_constant":"0.05 ft"}"#,
        source: "A level loop in the textbook's note form",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "survey.reduction.slope", reason: "alternative" },
        Related { id: "survey.reduction.curvature-refraction", reason: "next" },
    ],
    sentence: "ΣBS − ΣFS is {sum_backsights} less {sum_foresights}, and it checks against the elevations.{if abs(misclosure) >= 0} The run misses closing by {misclosure}.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_level,
    ..ToolDef::BLANK
};

struct Shot {
    station: String,
    bs: Option<f64>,
    fs: Option<f64>,
    is: Option<f64>,
    dist: Option<f64>,
}

fn run_level(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let start = ctx.req_quantity("start_elevation")?;
    let end = ctx.quantity("end_elevation")?;
    let allow_c = ctx.quantity("allowable_constant")?;
    let rows: Vec<Map<String, Value>> = ctx.rows("shots")?;
    let mut qs: Vec<(&str, Q)> = vec![("start_elevation", start)];
    if let Some(e) = end {
        qs.push(("end_elevation", e));
    }
    let mut raw = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let mut get =
            |name: &'static str, qs: &mut Vec<(&str, Q)>| -> Result<Option<Q>, ToolError> {
                let q = ctx.row_quantity("shots", i, row, name)?;
                if let Some(q) = q {
                    qs.push((name, q));
                }
                Ok(q)
            };
        let bs = get("backsight", &mut qs)?;
        let fs = get("foresight", &mut qs)?;
        let is = get("intermediate", &mut qs)?;
        let d = get("distance", &mut qs)?;
        let station = row
            .get("station")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        raw.push((station, bs, fs, is, d));
    }
    let u = common_unit(&qs)?;
    let shots: Vec<Shot> = raw
        .into_iter()
        .map(|(station, bs, fs, is, d)| Shot {
            station,
            bs: bs.map(|q| q.to(u)),
            fs: fs.map(|q| q.to(u)),
            is: is.map(|q| q.to(u)),
            dist: d.map(|q| q.to(u)),
        })
        .collect();
    let at = |i: usize, f: &str| format!("/shots/{i}/{f}");
    if shots[0].bs.is_none() {
        return Err(ToolError::invalid(
            &at(0, "backsight"),
            "The run starts with a backsight on the first benchmark.",
        ));
    }
    if shots[0].fs.is_some() {
        return Err(ToolError::invalid(
            &at(0, "foresight"),
            "The first station has only a backsight; a foresight here has nothing to be read from.",
        ));
    }
    // Reduce: each backsight sets a height of instrument; each foresight or
    // intermediate sight reads an elevation off it.
    let mut elev = start.to(u);
    let mut hi: Option<f64> = None;
    let (mut sbs, mut sfs) = (0.0, 0.0);
    let mut reduced: Vec<(Option<f64>, f64)> = Vec::new(); // (HI at this station, elevation)
    let mut carried: Vec<usize> = Vec::new(); // stations the run passes through (not side shots)
    for (i, s) in shots.iter().enumerate() {
        let mut here = elev;
        if let Some(is) = s.is {
            if s.bs.is_some() || s.fs.is_some() {
                return Err(ToolError::invalid(
                    &at(i, "intermediate"),
                    "A side shot has only an intermediate sight.",
                ));
            }
            let h = hi.ok_or_else(|| {
                ToolError::invalid(
                    &at(i, "intermediate"),
                    "A side shot needs a backsight before it.",
                )
            })?;
            reduced.push((None, h - is));
            continue;
        }
        if let Some(fs) = s.fs {
            let h = hi.ok_or_else(|| {
                ToolError::invalid(
                    &at(i, "foresight"),
                    "A foresight needs a backsight before it.",
                )
            })?;
            here = h - fs;
            sfs += fs;
        } else if i > 0 && s.bs.is_none() {
            return Err(ToolError::invalid(
                &at(i, "foresight"),
                "Each station after the first needs a foresight (and a backsight to carry on).",
            ));
        }
        elev = here;
        let new_hi = s.bs.map(|bs| {
            sbs += bs;
            here + bs
        });
        // With no backsight the run stops here; anything later needs a new one.
        hi = new_hi;
        reduced.push((new_hi, here));
        carried.push(i);
    }
    let last = *carried.last().expect("at least one station");
    if shots[last].fs.is_none() {
        return Err(ToolError::invalid(
            &at(last, "foresight"),
            "The run ends with a foresight on its last station.",
        ));
    }
    let first_e = start.to(u);
    let last_e = reduced[last].1;
    // The arithmetic check must balance; if it does not, the book is not what it seems.
    let diff = sbs - sfs;
    let scale = 1.0 + sbs.abs() + sfs.abs();
    if (diff - (last_e - first_e)).abs() > 1e-9 * scale {
        return Err(ToolError::invalid(
            "/shots",
            format!(
                "The arithmetic check does not balance: ΣBS − ΣFS = {diff} but the last elevation less the first is {}.",
                last_e - first_e
            ),
        ));
    }
    let q = |x: f64| Q { value: x, unit: u };
    let fmt = |x: f64| format!("{:.3}", x);
    let mut out = vec![
        ("sum_backsights", ctx.emit("sum_backsights", q(sbs), u)),
        ("sum_foresights", ctx.emit("sum_foresights", q(sfs), u)),
        (
            "check",
            Json::Str(format!(
                "ΣBS − ΣFS = {} − {} = {} {}, and the last elevation less the first = {} {}: balanced",
                fmt(sbs),
                fmt(sfs),
                fmt(diff),
                u.symbol,
                fmt(last_e - first_e),
                u.symbol
            )),
        ),
    ];
    // Closure: on a known closing benchmark, or back on the start for a loop.
    let known = end.map(|e| e.to(u)).or_else(|| {
        (shots[last].station.eq_ignore_ascii_case(&shots[0].station) && last > 0).then_some(first_e)
    });
    let mut adjust: Option<Vec<f64>> = None;
    if let Some(known) = known {
        let mis = last_e - known;
        out.push(("misclosure", ctx.emit("misclosure", q(mis), u)));
        // Cumulative distance to each carried station, or the count of setups.
        let have_dist = carried.iter().skip(1).all(|&i| shots[i].dist.is_some());
        let mut cum = vec![0.0; shots.len()];
        let mut run = 0.0;
        for &i in carried.iter().skip(1) {
            run += if have_dist {
                shots[i].dist.unwrap_or(0.0)
            } else {
                1.0
            };
            cum[i] = run;
        }
        let total = run.max(f64::MIN_POSITIVE);
        let mut adj = vec![f64::NAN; shots.len()];
        for &i in &carried {
            adj[i] = reduced[i].1 - mis * cum[i] / total;
        }
        adjust = Some(adj);
        if let Some(c) = allow_c {
            if !have_dist {
                return Err(ToolError::invalid(
                    "/shots",
                    "The allowable closure needs the distance of every leg (C × √K).",
                ));
            }
            let km = q(run).to(unit(QT::Length, "km"));
            let allowable = c.to(u) * sqrt(km);
            out.push(("allowable", ctx.emit("allowable", q(allowable), u)));
            out.push((
                "closure",
                Json::Str(if mis.abs() <= allowable {
                    "within the allowable".into()
                } else {
                    "beyond the allowable".into()
                }),
            ));
        }
    }
    let mut table = Vec::new();
    for (i, s) in shots.iter().enumerate() {
        let (h, e) = reduced[i];
        let mut row = vec![
            ("station", Json::Str(s.station.clone())),
            ("elevation", ctx.emit("elevation", q(e), u)),
        ];
        if let Some(h) = h {
            row.insert(1, ("hi", ctx.emit("hi", q(h), u)));
        }
        if let Some(a) = adjust.as_ref().map(|a| a[i]).filter(|a| a.is_finite()) {
            row.push(("adjusted", ctx.emit("adjusted", q(a), u)));
        }
        table.push(Json::obj(row));
    }
    out.insert(0, ("stations", Json::Arr(table)));
    Ok(Json::obj(out))
}
