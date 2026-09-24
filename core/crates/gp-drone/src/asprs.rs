//! The ASPRS Positional Accuracy Standards for Digital Geospatial Data,
//! Edition 2 (add-drone-suite, photogrammetry, "ASPRS Positional Accuracy
//! Standards (Edition 2)").
//!
//! Accuracy is RMSE alone (no 95% figures). Product accuracy adds the
//! checkpoint survey's own error in quadrature (section 7.11). From a list of
//! checkpoint errors the tool also computes the RMSEs themselves, tests the
//! non-vegetated vertical accuracy (NVA) and horizontal accuracy against their
//! classes, reports the vegetated vertical accuracy (VVA) as found, and applies
//! the standard's checks: at least 30 checkpoints (7.13), checkpoints twice as
//! accurate as the product (7.12), mean error under 25% of the target RMSE,
//! and any error over three times the target RMSE a blunder (7.2).

use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::sqrt;

use crate::unit;

const ASPRS: Reference = Reference {
    title: "ASPRS Positional Accuracy Standards for Digital Geospatial Data, Edition 2",
    issuer: "American Society for Photogrammetry and Remote Sensing",
    year: 2023,
    edition: "Edition 2 (Version 1.0, February 2023, as read; Version 2.0 was approved in 2024)",
    locator: "Section 7.2 (mean error and blunders), 7.3 and 7.4 (horizontal and vertical accuracy, NVA and VVA), 7.11 (product accuracy with checkpoint error, Table 7.4), 7.12 and 7.13 (checkpoint accuracy and the 30-checkpoint minimum)",
    url: "https://asprs.org/Main/Main/Standards/Positional-Accuracy-Standards.aspx",
};

/// The Edition 2 minimum number of checkpoints (section 7.13).
const MIN_CHECKPOINTS: usize = 30;

const fn cm(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "cm",
        },
    )
}

const ERROR_ROW: &[Field] = &[
    cm(
        "dx",
        "Easting error",
        "Product minus checkpoint, like -1.2 cm",
    ),
    cm(
        "dy",
        "Northing error",
        "Product minus checkpoint, like 0.8 cm",
    ),
    cm(
        "dz",
        "Height error",
        "Product minus checkpoint, like 2.1 cm",
    ),
    Field::new(
        "cover",
        "Ground cover",
        "open (bare ground, short grass, pavement; the default) or vegetated, like open",
        Kind::Choice(&["open", "vegetated"]),
    ),
];
const BLUNDER_ROW: &[Field] = &[
    Field::new(
        "checkpoint",
        "Checkpoint",
        "Its row in the list, from 1, like 7",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "component",
        "Component",
        "easting, northing, or height, like height",
        Kind::Text { max_len: 8 },
    ),
    cm("error", "Error", "The checkpoint's error, like 9.4 cm")
        .precision(Precision::Significant(3)),
    cm(
        "limit",
        "Blunder limit",
        "Three times the target RMSE, like 6 cm",
    )
    .precision(Precision::Significant(3)),
];

pub static ASPRS_ACCURACY: ToolDef = ToolDef {
    id: "drone.photogrammetry.asprs-accuracy",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "ASPRS accuracy (Edition 2)",
    summary: "Horizontal and vertical accuracy by the ASPRS Positional Accuracy Standards, Edition 2: product accuracy with the checkpoints' own error, NVA against its class, VVA as found, blunders, mean error, and the 30-checkpoint minimum.",
    aliases: &[
        "ASPRS accuracy calculator",
        "RMSE accuracy class",
        "NVA VVA calculator",
        "checkpoint accuracy report",
    ],
    keywords: &[
        "ASPRS",
        "accuracy",
        "RMSE",
        "checkpoints",
        "NVA",
        "VVA",
        "blunder",
        "mapping standard",
    ],
    inputs: &[
        cm("rmse_x", "RMSE x", "Fit to checkpoints, easting, like 1.0 cm").core(),
        cm("rmse_y", "RMSE y", "Fit to checkpoints, northing, like 1.0 cm").core(),
        cm("rmse_z", "RMSE z", "Fit to checkpoints, vertical, like 1.0 cm").core(),
        cm(
            "checkpoint_rmse",
            "Checkpoint survey RMSE",
            "The checkpoints' own accuracy per axis, as the surveyor reports it, like 2 cm",
        )
        .required()
        .core(),
        Field::new(
            "checkpoints",
            "Number of checkpoints",
            "Edition 2 requires at least 30, like 30; counted for you from a list of errors",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .core(),
        Field::new(
            "errors",
            "Checkpoint errors",
            "One checkpoint per row, product minus checkpoint, like 1.2, -0.8, 2.1 cm, open",
            Kind::List {
                items: ERROR_ROW,
                min: 1,
                max: 10_000,
            },
        ),
        cm(
            "target_horizontal",
            "Horizontal accuracy class",
            "The RMSE_H the product must meet, like 5 cm",
        ),
        cm(
            "target_vertical",
            "Vertical accuracy class",
            "The NVA RMSE_V the product must meet, like 5 cm",
        ),
    ],
    outputs: &[
        cm(
            "horizontal",
            "Horizontal accuracy (RMSE_H)",
            "Product accuracy including checkpoint error, like 3.0 cm",
        )
        .precision(Precision::Significant(3))
        .optional(),
        cm(
            "vertical",
            "Vertical accuracy (RMSE_V)",
            "Product accuracy including checkpoint error; the NVA when errors are listed, like 2.24 cm",
        )
        .precision(Precision::Significant(3))
        .optional(),
        Field::new(
            "checkpoint_status",
            "Checkpoint check",
            "Whether the checkpoints meet Edition 2, like meets the minimum of 30",
            Kind::Text { max_len: 400 },
        ),
        cm(
            "vva",
            "Vegetated vertical accuracy (VVA)",
            "RMSE_V in vegetation with checkpoint error, reported as found, like 6.1 cm",
        )
        .precision(Precision::Significant(3))
        .optional(),
        Field::new(
            "horizontal_result",
            "Horizontal class",
            "Whether RMSE_H meets the class, like meets the 5 cm class",
            Kind::Text { max_len: 80 },
        )
        .optional(),
        Field::new(
            "vertical_result",
            "Vertical class (NVA)",
            "Whether the NVA meets the class, like meets the 5 cm class",
            Kind::Text { max_len: 80 },
        )
        .optional(),
        cm("mean_x", "Mean easting error", "Average of the listed errors, like 0.3 cm")
            .precision(Precision::Significant(3))
            .optional(),
        cm(
            "mean_y",
            "Mean northing error",
            "Average of the listed errors, like -0.2 cm",
        )
        .precision(Precision::Significant(3))
        .optional(),
        cm(
            "mean_z",
            "Mean height error",
            "Average over open ground, like 0.4 cm",
        )
        .precision(Precision::Significant(3))
        .optional(),
        Field::new(
            "blunders",
            "Blunders",
            "Errors over three times the target RMSE, to investigate",
            Kind::List {
                items: BLUNDER_ROW,
                min: 0,
                max: 10_000,
            },
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::InvalidInput],
    warnings: &[
        "INSUFFICIENT_CHECKPOINTS",
        "CHECKPOINT_BLUNDER",
        "MEAN_ERROR_HIGH",
        "CHECKPOINTS_TOO_COARSE",
        "UNIT_ASSUMED",
    ],
    model: "ASPRS Edition 2: RMSE = √(Σe² / n) per component; RMSE_H = √(RMSE_x² + RMSE_y²); product accuracy = √(RMSE_fit² + RMSE_checkpoint²) per component, so horizontally √(RMSE_H1² + RMSE_H2²) with RMSE_H2 = √2 × the per-axis survey RMSE (section 7.11); NVA from open-ground checkpoints must be at or under its class, VVA from vegetated ones is reported; mean error under 25% and any error over 3 times the target RMSE per component (horizontal target ÷ √2 per axis) a blunder (7.2); checkpoints at least twice as accurate as the product (7.12) and at least 30 (7.13)",
    accuracy: "Exact to the standard's definitions",
    when_to_use: "Use this to write the accuracy statement for a mapping product by the current ASPRS standard: an orthomosaic, a drone survey, a lidar surface, or a planimetric map checked against surveyed checkpoints. Give the fit RMSEs you already have, or paste the checkpoint errors and let it compute them, and it gives the product accuracy with the checkpoints' own error folded in, whether the product meets its horizontal and NVA classes, the VVA as found, and the checks the standard asks for before a statement can be made: enough checkpoints, accurate enough, no blunders, and no bias.",
    limitations: "Accuracy here is RMSE only, as Edition 2 requires; the 95% confidence figures of older standards are not computed, and a 95% number should not be quoted as an ASPRS accuracy. The checkpoint survey RMSE is taken per axis, the same in easting, northing, and height, as the standard assumes when it combines them; if the surveyor reports a horizontal radial figure, divide it by √2 first. The standard applies its blunder limit per component but states horizontal classes as RMSE_H, so the per-axis horizontal target is taken as the class divided by √2. Blunders are listed for investigation, never removed, and vegetated height errors are not tested against a limit, since VVA has no pass or fail. It cannot tell whether checkpoints are well defined, well distributed, or on slopes under 10%, which the standard also requires.",
    references: &[ASPRS],
    examples: &[Example {
        id: "primary",
        title: "1.00 cm fit with 2.0 cm checkpoints",
        input: r#"{"rmse_z":"1.00 cm","checkpoint_rmse":"2.0 cm","checkpoints":30}"#,
        source: "ASPRS Positional Accuracy Standards, Edition 2, Table 7.4, first row: 1.00 cm fit and 2.0 cm survey checkpoints give 2.24 cm",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.altitude-for-gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.mission.survey-grid",
            reason: "alternative",
        },
    ],
    sentence: "{if vertical > 0}Vertical accuracy is {vertical} RMSE. {/if}{if horizontal > 0}Horizontal accuracy is {horizontal} RMSE. {/if}{checkpoint_status}",
    limits: &[("batchRows", 10_000)],
    run: run_asprs,
    ..ToolDef::BLANK
};

fn cmq(v_m: f64) -> Q {
    Q {
        value: v_m,
        unit: unit(QT::Length, "m"),
    }
}

/// A list cell in centimeters, from meters.
fn in_cm(v_m: f64) -> Json {
    Q {
        value: v_m * 100.0,
        unit: unit(QT::Length, "cm"),
    }
    .to_json()
}

fn rms(v: &[f64]) -> f64 {
    sqrt(v.iter().map(|e| e * e).sum::<f64>() / v.len() as f64)
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Errors read from the list, in meters: (row, dx, dy, dz, vegetated).
type Row = (usize, Option<f64>, Option<f64>, Option<f64>, bool);

fn read_errors(ctx: &mut Ctx) -> Result<Option<Vec<Row>>, ToolError> {
    if !ctx.is_set("errors") {
        return Ok(None);
    }
    let meters = unit(QT::Length, "m");
    let rows = ctx.rows("errors")?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let mut get = |k: &str| -> Result<Option<f64>, ToolError> {
            Ok(ctx.row_quantity("errors", i, r, k)?.map(|q| q.to(meters)))
        };
        let (dx, dy, dz) = (get("dx")?, get("dy")?, get("dz")?);
        if dx.is_some() != dy.is_some() {
            return Err(ToolError::invalid(
                &format!("/errors/{i}"),
                "A horizontal checkpoint needs both its easting and northing errors.",
            ));
        }
        if dx.is_none() && dz.is_none() {
            return Err(ToolError::invalid(
                &format!("/errors/{i}"),
                "Each checkpoint needs its easting and northing errors, its height error, or both.",
            ));
        }
        let vegetated = r.get("cover").and_then(|v| v.as_str()) == Some("vegetated");
        out.push((i + 1, dx, dy, dz, vegetated));
    }
    Ok(Some(out))
}

fn run_asprs(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let meters = unit(QT::Length, "m");
    let listed = read_errors(ctx)?;
    let given = |ctx: &mut Ctx, k: &str| -> Result<Option<f64>, ToolError> {
        Ok(ctx.quantity(k)?.map(|q| q.to(meters)))
    };
    let (in_x, in_y, in_z) = (
        given(ctx, "rmse_x")?,
        given(ctx, "rmse_y")?,
        given(ctx, "rmse_z")?,
    );
    let cp = ctx.req_quantity("checkpoint_rmse")?.to(meters);
    let target_h = given(ctx, "target_horizontal")?;
    let target_v = given(ctx, "target_vertical")?;
    if cp < 0.0 {
        return Err(ToolError::invalid(
            "/checkpoint_rmse",
            "The checkpoint survey RMSE cannot be negative.",
        ));
    }
    for (k, t) in [
        ("target_horizontal", target_h),
        ("target_vertical", target_v),
    ] {
        if t.is_some_and(|t| t <= 0.0) {
            return Err(ToolError::invalid(
                &format!("/{k}"),
                "An accuracy class must be more than zero.",
            ));
        }
    }

    // The fit RMSEs: given, or computed from the listed errors.
    let (mut rx, mut ry, mut rz, mut r_veg) = (in_x, in_y, in_z, None);
    let (mut n_h, mut n_v, mut n_veg) = (None, None, None);
    let mut means = (None, None, None);
    let mut blunders = Vec::new();
    if let Some(rows) = &listed {
        if in_x.is_some() || in_y.is_some() || in_z.is_some() {
            return Err(ToolError::invalid(
                "/errors",
                "Give the checkpoint errors or the fit RMSEs, not both: the RMSEs are computed from the errors.",
            ));
        }
        let xs: Vec<f64> = rows.iter().filter_map(|r| r.1).collect();
        let ys: Vec<f64> = rows.iter().filter_map(|r| r.2).collect();
        let zs: Vec<f64> = rows.iter().filter(|r| !r.4).filter_map(|r| r.3).collect();
        let vs: Vec<f64> = rows.iter().filter(|r| r.4).filter_map(|r| r.3).collect();
        if !xs.is_empty() {
            (rx, ry) = (Some(rms(&xs)), Some(rms(&ys)));
            n_h = Some(xs.len());
            means.0 = Some(mean(&xs));
            means.1 = Some(mean(&ys));
        }
        if !zs.is_empty() {
            rz = Some(rms(&zs));
            n_v = Some(zs.len());
            means.2 = Some(mean(&zs));
        }
        if !vs.is_empty() {
            r_veg = Some(rms(&vs));
            n_veg = Some(vs.len());
        }
        // Blunders: over three times the target RMSE in any component.
        let per_axis_h = target_h.map(|t| t / core::f64::consts::SQRT_2);
        for &(row, dx, dy, dz, veg) in rows {
            let checks = [
                ("easting", dx, per_axis_h),
                ("northing", dy, per_axis_h),
                ("height", if veg { None } else { dz }, target_v),
            ];
            for (name, e, t) in checks {
                if let (Some(e), Some(t)) = (e, t)
                    && e.abs() > 3.0 * t
                {
                    blunders.push(Json::obj([
                        ("checkpoint", Json::Num(row as f64)),
                        ("component", Json::str(name)),
                        ("error", in_cm(e)),
                        ("limit", in_cm(3.0 * t)),
                    ]));
                }
            }
        }
    }
    if rx.is_none() && ry.is_none() && rz.is_none() && r_veg.is_none() {
        return Err(ToolError::invalid(
            "/rmse_z",
            "Give at least one fit RMSE (x and y for horizontal, or z for vertical), or a list of checkpoint errors.",
        ));
    }
    if rx.is_some() != ry.is_some() {
        return Err(ToolError::invalid(
            "/rmse_y",
            "Horizontal accuracy needs both RMSE x and RMSE y.",
        ));
    }

    // Product accuracy: the fit and the checkpoint survey error in quadrature.
    let with_cp = |r: f64| sqrt(r * r + cp * cp);
    let mut out = Vec::new();
    let horizontal = rx.zip(ry).map(|(x, y)| libm::hypot(with_cp(x), with_cp(y)));
    let vertical = rz.map(with_cp);
    let vva = r_veg.map(with_cp);
    if let Some(h) = horizontal {
        out.push(("horizontal", ctx.out("horizontal", cmq(h))));
    }
    if let Some(v) = vertical {
        out.push(("vertical", ctx.out("vertical", cmq(v))));
    }

    let fmt_cm = |ctx: &Ctx, v: f64| {
        format!(
            "{} cm",
            display::number(v * 100.0, Precision::Significant(3), ctx.options.format)
        )
    };
    // The count: stated, or from the list, per kind of test.
    let mut notes = Vec::new();
    let mut short = false;
    let counts: Vec<(&str, usize)> = if listed.is_some() {
        [("horizontal", n_h), ("NVA", n_v), ("VVA", n_veg)]
            .into_iter()
            .filter_map(|(k, n)| n.map(|n| (k, n)))
            .collect()
    } else {
        let n = ctx.number("checkpoints")?.ok_or_else(|| {
            ToolError::invalid(
                "/checkpoints",
                "Give the number of checkpoints, or list their errors.",
            )
        })?;
        vec![("", n as usize)]
    };
    for (kind, n) in &counts {
        if *n < MIN_CHECKPOINTS {
            short = true;
            let what = if kind.is_empty() {
                String::new()
            } else {
                format!(" for the {kind} test")
            };
            notes.push(format!(
                "{} checkpoints{what} is below the Edition 2 minimum of 30.",
                display::number(*n as f64, Precision::Decimals(0), ctx.options.format)
            ));
        }
    }
    if short {
        ctx.warnings.push(
            Warning::new(
                "INSUFFICIENT_CHECKPOINTS",
                "ASPRS Edition 2 requires at least 30 checkpoints for an accuracy statement.",
            )
            .at(if listed.is_some() {
                "/errors"
            } else {
                "/checkpoints"
            }),
        );
    }
    // Checkpoints at least twice as accurate as the product (7.12).
    let tightest = [target_h.map(|t| t / core::f64::consts::SQRT_2), target_v]
        .into_iter()
        .flatten()
        .fold(f64::INFINITY, f64::min);
    if tightest.is_finite() && cp > tightest / 2.0 {
        ctx.warnings.push(
            Warning::new(
                "CHECKPOINTS_TOO_COARSE",
                "Edition 2 needs checkpoints at least twice as accurate as the product.",
            )
            .at("/checkpoint_rmse"),
        );
        notes.push(format!(
            "The checkpoints ({}) are not twice as accurate as the {} target per axis.",
            fmt_cm(ctx, cp),
            fmt_cm(ctx, tightest)
        ));
    }
    if !blunders.is_empty() {
        ctx.warnings.push(
            Warning::new(
                "CHECKPOINT_BLUNDER",
                "A checkpoint error is over three times the target RMSE: investigate it before stating accuracy.",
            )
            .at("/errors"),
        );
        notes.push(format!(
            "{} {} over three times the target RMSE: investigate before stating accuracy.",
            blunders.len(),
            if blunders.len() == 1 {
                "error is"
            } else {
                "errors are"
            }
        ));
    }
    // Mean error under 25% of the target RMSE (7.2).
    let mut biased = Vec::new();
    for (name, m, t) in [
        (
            "easting",
            means.0,
            target_h.map(|t| t / core::f64::consts::SQRT_2),
        ),
        (
            "northing",
            means.1,
            target_h.map(|t| t / core::f64::consts::SQRT_2),
        ),
        ("height", means.2, target_v),
    ] {
        if let (Some(m), Some(t)) = (m, t)
            && m.abs() >= 0.25 * t
        {
            biased.push(format!("{name} {}", fmt_cm(ctx, m)));
        }
    }
    if !biased.is_empty() {
        ctx.warnings.push(
            Warning::new(
                "MEAN_ERROR_HIGH",
                "A mean error is 25% of the target RMSE or more: look for a systematic error.",
            )
            .at("/errors"),
        );
        notes.push(format!(
            "The mean error is 25% of the target or more ({}): look for a systematic error.",
            biased.join(", ")
        ));
    }
    let status = if notes.is_empty() {
        "The checkpoint count meets the Edition 2 minimum of 30.".to_owned()
    } else {
        notes.join(" ")
    };
    out.push(("checkpoint_status", Json::str(status)));

    if let Some(v) = vva {
        out.push(("vva", ctx.out("vva", cmq(v))));
    }
    let verdict = |ctx: &Ctx, got: f64, class: f64| {
        let c = fmt_cm(ctx, class);
        if got <= class {
            format!("meets the {c} class")
        } else {
            format!("does not meet the {c} class")
        }
    };
    if let (Some(h), Some(t)) = (horizontal, target_h) {
        out.push(("horizontal_result", Json::str(verdict(ctx, h, t))));
    }
    if let (Some(v), Some(t)) = (vertical, target_v) {
        out.push(("vertical_result", Json::str(verdict(ctx, v, t))));
    }
    if let Some(m) = means.0 {
        out.push(("mean_x", ctx.out("mean_x", cmq(m))));
    }
    if let Some(m) = means.1 {
        out.push(("mean_y", ctx.out("mean_y", cmq(m))));
    }
    if let Some(m) = means.2 {
        out.push(("mean_z", ctx.out("mean_z", cmq(m))));
    }
    if listed.is_some() {
        out.push(("blunders", Json::Arr(blunders)));
    }
    Ok(Json::obj(out))
}
