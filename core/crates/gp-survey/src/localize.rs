//! Site localization (add-practitioner-essentials, gnss-field "Site
//! localization (calibration)"): the least-squares transform from local site
//! coordinates to grid coordinates through control points seen in both, as a
//! 4-parameter similarity (scale, rotation, shift) or a 6-parameter affine,
//! with residuals, their root mean square, and checks for no redundancy and
//! a suspect scale.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{atan2, hypot, sqrt};

use crate::{GHILANI, len, unit};

const PAIR: &[Field] = &[
    Field::new(
        "name",
        "Point",
        "Like CP1; optional",
        Kind::Text { max_len: 24 },
    ),
    len("local_n", "Local northing", "Like 5000").required(),
    len("local_e", "Local easting", "Like 5000").required(),
    len("grid_n", "Grid northing", "Like 1520345.12").required(),
    len("grid_e", "Grid easting", "Like 3140022.87").required(),
];

const RESIDUAL: &[Field] = &[
    Field::new(
        "name",
        "Point",
        "As given, or its row number",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "dn",
        "Northing residual",
        "Grid given − grid from the fit",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "de",
        "Easting residual",
        "Grid given − grid from the fit",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "horizontal",
        "Horizontal residual",
        "√(dN² + dE²)",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(4)),
];

const fn number(name: &'static str, title: &'static str, help: &'static str, d: u8) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e12,
            max: 1e12,
        },
    )
    .precision(Precision::Decimals(d))
}

pub static LOCALIZATION: ToolDef = ToolDef {
    id: "survey.gnss.localization",
    title: "Site localization (calibration)",
    summary: "Fits local site coordinates to grid coordinates through control points seen in both, by a similarity (scale, rotation, shift) or affine transform, with residuals, their root mean square, and a check for a feet-and-meters mix-up.",
    aliases: &[
        "site calibration",
        "site localization",
        "local to grid transformation",
        "Helmert 2D transformation",
        "4 parameter transformation",
    ],
    keywords: &[
        "localization",
        "calibration",
        "similarity",
        "affine",
        "Helmert",
        "control points",
        "residuals",
        "GNSS",
    ],
    inputs: &[
        Field::new(
            "pairs",
            "Control points",
            "Local and grid northing and easting of each, like 5000, 5000, 1520345.12, 3140022.87",
            Kind::List {
                items: PAIR,
                min: 2,
                max: 500,
            },
        )
        .required()
        .core(),
        Field::new(
            "model",
            "Transform",
            "similarity (4 parameters, default, needs 2 points) or affine (6 parameters, needs 3)",
            Kind::Choice(&["similarity", "affine"]),
        )
        .core(),
        number(
            "expected_scale",
            "Expected combined factor",
            "Grid scale × elevation factor at the site, like 0.99997",
            9,
        )
        .core(),
    ],
    outputs: &[
        number("scale", "Scale", "Grid distance / local distance", 9),
        number(
            "scale_ppm",
            "Scale offset",
            "(scale − 1) in parts per million",
            1,
        ),
        Field::new(
            "rotation",
            "Rotation",
            "Of the local axes onto the grid, counterclockwise positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6))
        .angle_range("unbounded"),
        Field::new(
            "rms",
            "Root-mean-square residual",
            "√(Σ horizontal residual² / points)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "shift_n",
            "Northing shift",
            "Grid northing of the local origin",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "shift_e",
            "Easting shift",
            "Grid easting of the local origin",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
        number(
            "a",
            "a",
            "E = a·e + b·n + shift (affine: E from local e)",
            12,
        ),
        number(
            "b",
            "b",
            "E = a·e + b·n + shift (affine: E from local n)",
            12,
        ),
        number(
            "c",
            "c",
            "N = c·e + d·n + shift (affine: N from local e)",
            12,
        ),
        number(
            "d",
            "d",
            "N = c·e + d·n + shift (affine: N from local n)",
            12,
        ),
        Field::new(
            "model_used",
            "Transform",
            "similarity or affine",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "residuals",
            "Residuals",
            "At each control point",
            Kind::List {
                items: RESIDUAL,
                min: 0,
                max: 500,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::DegenerateGeometry],
    warnings: &[
        "NO_REDUNDANCY",
        "SCALE_SUSPECT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Least squares on centroid-reduced coordinates. Similarity: E = a·e + b·n + tE, N = c·e + d·n + tN with d = a and b = −c, scale √(a² + c²), rotation atan2(c, a). Affine: E = a·e + b·n + tE, N = c·e + d·n + tN, scale √|ad − bc|. Local and grid values are compared in meters",
    accuracy: "Exact least squares; the transform is only as good as the control points and holds within their extent",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Four control points, similarity",
        input: r#"{"pairs":[{"name":"CP1","local_n":"5000 ft","local_e":"5000 ft","grid_n":"1520129.021 ft","grid_e":"3139867.253 ft"},{"name":"CP2","local_n":"6000 ft","local_e":"5200 ft","grid_n":"1521133.878 ft","grid_e":"3140041.010 ft"},{"name":"CP3","local_n":"5400 ft","local_e":"6100 ft","grid_n":"1520557.663 ft","grid_e":"3140956.371 ft"},{"name":"CP4","local_n":"4700 ft","local_e":"5800 ft","grid_n":"1519850.067 ft","grid_e":"3140674.813 ft"}],"expected_scale":0.99997}"#,
        source: "Built at scale 0.99997 and 1.5° rotation with a few millimeters of noise; least squares after Ghilani and Wolf (2021)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.reduction.combined-factor",
            reason: "parent",
        },
        Related {
            id: "survey.gnss.rtk-budget",
            reason: "alternative",
        },
    ],
    sentence: "The fit has a scale of {scale}, and its residuals average {rms} (root mean square).",
    limits: &[("batchRows", 100)],
    run: run_localization,
    ..ToolDef::BLANK
};

fn run_localization(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("pairs")?;
    let mut names = Vec::with_capacity(rows.len());
    // (local e, local n, grid e, grid n), meters.
    let mut pts: Vec<(f64, f64, f64, f64)> = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let mut get = |f: &str| -> Result<f64, ToolError> {
            Ok(ctx
                .row_quantity("pairs", i, r, f)?
                .expect("required")
                .base())
        };
        let (ln, le, gn, ge) = (
            get("local_n")?,
            get("local_e")?,
            get("grid_n")?,
            get("grid_e")?,
        );
        pts.push((le, ln, ge, gn));
        names.push(
            ctx.row_text("pairs", i, r, "name")?
                .unwrap_or_else(|| format!("{}", i + 1)),
        );
    }
    let affine = ctx.choice("model")? == Some("affine");
    let n = pts.len();
    let need = if affine { 3 } else { 2 };
    if n < need {
        return Err(ToolError::invalid(
            "/pairs",
            format!(
                "An {} fit needs at least {need} control points.",
                if affine { "affine" } else { "similarity" }
            ),
        ));
    }
    let nf = n as f64;
    let mean = pts.iter().fold((0.0, 0.0, 0.0, 0.0), |m, p| {
        (
            m.0 + p.0 / nf,
            m.1 + p.1 / nf,
            m.2 + p.2 / nf,
            m.3 + p.3 / nf,
        )
    });
    let c: Vec<(f64, f64, f64, f64)> = pts
        .iter()
        .map(|p| (p.0 - mean.0, p.1 - mean.1, p.2 - mean.2, p.3 - mean.3))
        .collect();
    let (sxx, syy, sxy) = c.iter().fold((0.0, 0.0, 0.0), |s, p| {
        (s.0 + p.0 * p.0, s.1 + p.1 * p.1, s.2 + p.0 * p.1)
    });
    let spread = sxx + syy;
    if spread <= 1e-12 * (1.0 + mean.0.abs() + mean.1.abs()).powi(2) {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The local points coincide, so they fix no scale or rotation.",
        )
        .at("/pairs"));
    }
    // E = a·e + b·n, N = c·e + d·n on reduced coordinates.
    let (a, b, cc, d) = if affine {
        let det = sxx * syy - sxy * sxy;
        if det.abs() <= 1e-12 * spread * spread {
            return Err(ToolError::new(
                ErrorCode::DegenerateGeometry,
                "The local points lie on one line, which an affine fit cannot resolve; use the similarity fit.",
            )
            .at("/pairs"));
        }
        let (sxe, sye, sxn, syn) = c.iter().fold((0.0, 0.0, 0.0, 0.0), |s, p| {
            (
                s.0 + p.0 * p.2,
                s.1 + p.1 * p.2,
                s.2 + p.0 * p.3,
                s.3 + p.1 * p.3,
            )
        });
        (
            (sxe * syy - sye * sxy) / det,
            (sye * sxx - sxe * sxy) / det,
            (sxn * syy - syn * sxy) / det,
            (syn * sxx - sxn * sxy) / det,
        )
    } else {
        let p = c.iter().map(|q| q.0 * q.2 + q.1 * q.3).sum::<f64>() / spread;
        let q = c.iter().map(|q| q.0 * q.3 - q.1 * q.2).sum::<f64>() / spread;
        (p, -q, q, p)
    };
    let scale = if affine {
        sqrt((a * d - b * cc).abs())
    } else {
        hypot(a, cc)
    };
    let rotation = atan2(cc, a).to_degrees();
    let te = mean.2 - a * mean.0 - b * mean.1;
    let tn = mean.3 - cc * mean.0 - d * mean.1;
    let m = unit(QT::Length, "m");
    let q = |v: f64| Q { value: v, unit: m };
    let mut sum2 = 0.0;
    let mut res = Vec::with_capacity(n);
    for (p, name) in pts.iter().zip(&names) {
        let (fe, fnn) = (a * p.0 + b * p.1 + te, cc * p.0 + d * p.1 + tn);
        let (dn, de) = (p.3 - fnn, p.2 - fe);
        sum2 += dn * dn + de * de;
        res.push(Json::obj([
            ("name", Json::str(name.clone())),
            ("dn", q(dn).to_json()),
            ("de", q(de).to_json()),
            ("horizontal", q(hypot(dn, de)).to_json()),
        ]));
    }
    let rms = sqrt(sum2 / nf);
    if n == need {
        ctx.warnings.push(Warning::new(
            "NO_REDUNDANCY",
            format!("{n} points exactly fix a {} fit, so the residuals are zero and say nothing about the control. Add another point to check it.", if affine { "6-parameter" } else { "4-parameter" }),
        ));
    }
    let fmt = ctx.options.format;
    let expected = ctx.number("expected_scale")?;
    let off_ppm = |e: f64| (scale / e - 1.0) * 1e6;
    let suspect = match expected {
        Some(e) if e > 0.0 => (off_ppm(e).abs() > 100.0).then(|| off_ppm(e)),
        Some(_) => {
            return Err(ToolError::invalid(
                "/expected_scale",
                "The expected combined factor must be positive.",
            ));
        }
        None => ((scale - 1.0).abs() > 0.01).then(|| off_ppm(1.0)),
    };
    if let Some(ppm) = suspect {
        let feet = (scale - 0.3048).abs() < 0.01 || (scale - 1.0 / 0.3048).abs() < 0.03;
        ctx.warnings.push(Warning::new(
            "SCALE_SUSPECT",
            format!(
                "The fitted scale, {}, is {} ppm from the {}.{}",
                display::number(scale, Precision::Decimals(6), fmt),
                display::number(ppm, Precision::Decimals(0), fmt),
                if expected.is_some() { "expected combined factor" } else { "1 a site this size should have" },
                if feet { " That is close to the ratio of a foot to a meter: check that local and grid values carry the units they are in." } else { " Check the control and the units." }
            ),
        ));
    }
    if ctx.explaining() {
        let n2 = move |v: f64, dp: u8| display::number(v, Precision::Decimals(dp), fmt);
        ctx.step(
            "Rotation",
            "atan2(c, a) from the least-squares coefficients",
            format!("a = {}, c = {}", n2(a, 9), n2(cc, 9)),
            format!("{}°", n2(rotation, 6)),
        );
        ctx.step(
            "Scale",
            if affine {
                "√|ad − bc|"
            } else {
                "√(a² + c²)"
            },
            format!("a = {}, c = {}", n2(a, 9), n2(cc, 9)),
            n2(scale, 9),
        );
    }
    Ok(Json::obj(vec![
        ("scale", Json::Num(scale)),
        ("scale_ppm", Json::Num((scale - 1.0) * 1e6)),
        (
            "rotation",
            ctx.out(
                "rotation",
                Q {
                    value: rotation,
                    unit: unit(QT::Angle, "deg"),
                },
            ),
        ),
        ("rms", ctx.out("rms", q(rms))),
        ("shift_n", ctx.out("shift_n", q(tn))),
        ("shift_e", ctx.out("shift_e", q(te))),
        ("a", Json::Num(a)),
        ("b", Json::Num(b)),
        ("c", Json::Num(cc)),
        ("d", Json::Num(d)),
        (
            "model_used",
            Json::str(if affine { "affine" } else { "similarity" }),
        ),
        ("residuals", Json::Arr(res)),
    ]))
}
