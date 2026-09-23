//! Unequal-tangent vertical curves (add-survey-suite, survey/alignment-
//! curves, "Vertical curves": "It SHALL support unequal-tangent curves"): two
//! parabolas meeting under the PVI with a common grade, the high or low point,
//! and the elevation at any station.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};

use crate::{common_unit, fmt_station, is_metric, len, len_out, station};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 25 (vertical curves: unequal-tangent curves)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

pub static UNEQUAL: ToolDef = ToolDef {
    id: "survey.curves.unequal-vertical-curve",
    version: "1.0.1",
    title: "Unequal-tangent vertical curve",
    summary: "A vertical curve with different tangent lengths each side of the PVI: PVC, PVT, and the point under the PVI, the high or low point, and the elevation at any station.",
    aliases: &[
        "unequal tangent vertical curve",
        "asymmetric vertical curve",
        "unsymmetrical vertical curve",
    ],
    keywords: &[
        "vertical curve",
        "unequal",
        "asymmetric",
        "PVI",
        "PVC",
        "PVT",
        "high point",
        "low point",
        "grade",
    ],
    inputs: &[
        Field::new(
            "g1",
            "Entering grade g1",
            "Percent, like 2",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "g2",
            "Leaving grade g2",
            "Percent, like -3",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .required()
        .core(),
        len(
            "length1",
            "Length before the PVI",
            "PVC to PVI, like 200 ft",
        )
        .required()
        .core(),
        len("length2", "Length after the PVI", "PVI to PVT, like 400 ft")
            .required()
            .core(),
        Field::new(
            "pvi_station",
            "PVI station",
            "Like 10+00; without one, stations count from the PVI at 0+00",
            Kind::Text { max_len: 16 },
        ),
        len("pvi_elevation", "PVI elevation", "Like 100.00 ft")
            .required()
            .core(),
        Field::new(
            "at_station",
            "Elevation at station",
            "Optional, like 9+50",
            Kind::Text { max_len: 16 },
        ),
    ],
    outputs: &[
        Field::new(
            "pvc_station",
            "PVC station",
            "PVI − L1",
            Kind::Text { max_len: 16 },
        ),
        len_out("pvc_elevation", "PVC elevation", "On the entering tangent"),
        Field::new(
            "pvt_station",
            "PVT station",
            "PVI + L2",
            Kind::Text { max_len: 16 },
        ),
        len_out("pvt_elevation", "PVT elevation", "On the leaving tangent"),
        len_out(
            "cvc_elevation",
            "Curve elevation under the PVI",
            "PVI + L1·L2·(g2 − g1) / (200 (L1 + L2))",
        ),
        Field::new(
            "cvc_grade",
            "Grade under the PVI",
            "(L1 g1 + L2 g2) / (L1 + L2), percent",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "turning_station",
            "High or low point station",
            "Where the grade is zero, if on the curve",
            Kind::Text { max_len: 16 },
        )
        .optional(),
        len_out(
            "turning_elevation",
            "High or low point elevation",
            "Where the grade is zero",
        )
        .optional(),
        Field::new(
            "turning",
            "High or low point",
            "high, low, or none on the curve",
            Kind::Text { max_len: 40 },
        ),
        len_out(
            "elevation",
            "Elevation at the station asked",
            "On the curve or its tangents",
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Two parabolas, PVC to the point under the PVI and on to the PVT, sharing the grade g_c = (L1 g1 + L2 g2)/(L1 + L2) where they meet; the curve passes L1·L2·(g2 − g1)/(200(L1 + L2)) from the PVI (Ghilani & Wolf 2021, ch. 25)",
    accuracy: "Exact for the parabolas",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "g1 = +2%, g2 = −3%, 200 ft before and 400 ft after a PVI at 10+00",
        input: r#"{"g1":2,"g2":-3,"length1":"200 ft","length2":"400 ft","pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
        source: "Two-parabola unequal-tangent curve",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.curves.vertical-curve",
            reason: "alternative",
        },
        Related {
            id: "survey.curves.sight-distance",
            reason: "next",
        },
    ],
    sentence: "The curve runs PVC {pvc_station} to PVT {pvt_station}. At the PVI station it is at {cvc_elevation}, and the PVI is at {pvi_elevation}; {turning}.",
    limits: &[("batchRows", 10_000)],
    run: run_unequal,
    ..ToolDef::BLANK
};

fn run_unequal(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (l1q, l2q, eq) = (
        ctx.req_quantity("length1")?,
        ctx.req_quantity("length2")?,
        ctx.req_quantity("pvi_elevation")?,
    );
    let u = common_unit(&[("length1", l1q), ("length2", l2q), ("pvi_elevation", eq)])?;
    let (l1, l2, e) = (l1q.to(u), l2q.to(u), eq.to(u));
    if l1 <= 0.0 || l2 <= 0.0 {
        return Err(ToolError::invalid(
            "/length1",
            "Both tangent lengths must be positive.",
        ));
    }
    let g1 = ctx.number("g1")?.expect("required") / 100.0;
    let g2 = ctx.number("g2")?.expect("required") / 100.0;
    let pvi = station(ctx, "pvi_station")?.unwrap_or(0.0);
    let gc = (l1 * g1 + l2 * g2) / (l1 + l2);
    let (r1, r2) = ((gc - g1) / l1, (g2 - gc) / l2);
    let (s_pvc, s_pvt) = (pvi - l1, pvi + l2);
    let (e_pvc, e_pvt) = (e - g1 * l1, e + g2 * l2);
    let e_cvc = e + l1 * l2 * (g2 - g1) / (2.0 * (l1 + l2));
    // Elevation anywhere: the entering tangent, either parabola, or the leaving tangent.
    let elev = |s: f64| -> f64 {
        if s <= s_pvc {
            e_pvc + g1 * (s - s_pvc)
        } else if s <= pvi {
            let x = s - s_pvc;
            e_pvc + g1 * x + r1 * x * x / 2.0
        } else if s <= s_pvt {
            let x = s - pvi;
            e_cvc + gc * x + r2 * x * x / 2.0
        } else {
            e_pvt + g2 * (s - s_pvt)
        }
    };
    // The grade is zero where each parabola's slope crosses zero.
    let turn = [(r1, g1, s_pvc, l1), (r2, gc, pvi, l2)]
        .iter()
        .filter(|(r, _, _, _)| *r != 0.0)
        .map(|&(r, g, s0, len)| (s0 - g / r, (-g / r), len))
        .find(|&(_, x, len)| (0.0..=len).contains(&x))
        .map(|(s, _, _)| s);
    let metric = is_metric(u);
    let q = |v: f64| Q { value: v, unit: u };
    let mut out = vec![
        ("pvc_station", Json::Str(fmt_station(s_pvc, metric))),
        ("pvc_elevation", ctx.emit("pvc_elevation", q(e_pvc), u)),
        ("pvt_station", Json::Str(fmt_station(s_pvt, metric))),
        ("pvt_elevation", ctx.emit("pvt_elevation", q(e_pvt), u)),
        ("cvc_elevation", ctx.emit("cvc_elevation", q(e_cvc), u)),
        ("cvc_grade", Json::Num(gc * 100.0)),
    ];
    match turn {
        Some(s) => {
            out.push(("turning_station", Json::Str(fmt_station(s, metric))));
            out.push((
                "turning_elevation",
                ctx.emit("turning_elevation", q(elev(s)), u),
            ));
            out.push((
                "turning",
                Json::Str(if g2 < g1 {
                    "the high point is on the curve".into()
                } else {
                    "the low point is on the curve".into()
                }),
            ));
        }
        None => out.push((
            "turning",
            Json::Str("no high or low point lies on the curve".into()),
        )),
    }
    if let Some(s) = station(ctx, "at_station")? {
        out.push(("elevation", ctx.emit("elevation", q(elev(s)), u)));
    }
    Ok(Json::obj(out))
}
