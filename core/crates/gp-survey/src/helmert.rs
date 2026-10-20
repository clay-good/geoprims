//! 3D conformal fit (add-survey-partition-and-adjustment, "3D conformal fit
//! from common points"): the seven Bursa-Wolf parameters that best carry one
//! set of 3D coordinates onto another, by least squares, in the linearized
//! rotation model of IOGP Guidance Note 7-2 that `geodesy.datum.helmert`
//! applies, so the fitted parameters can be sent to it unchanged.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT, Unit};
use libm::sqrt;

const IOGP: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "§4.2.3 Helmert 7-parameter transformations (EPSG methods 1032 and 1033)",
    url: "https://www.iogp.org/wp-content/uploads/2019/09/373-07-02.pdf",
};

const MAX_POINTS: usize = 500;
/// Seconds of arc per radian.
const ARCSEC: f64 = 206_264.806_247_096_36;

fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

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

const POINT: &[Field] = &[
    Field::new("name", "Point", "Like P1", Kind::Text { max_len: 24 }),
    len("x", "Source X", "Like 4027894.006 m").required(),
    len("y", "Source Y", "Like 307045.600 m").required(),
    len("z", "Source Z", "Like 4919474.910 m").required(),
    len("x_target", "Target X", "Like 4027893.682 m").required(),
    len("y_target", "Target Y", "Like 307045.252 m").required(),
    len("z_target", "Target Z", "Like 4919475.164 m").required(),
];

const OUT_RESIDUAL: &[Field] = &[
    Field::new("name", "Point", "", Kind::Text { max_len: 24 }),
    Field::new(
        "residual",
        "Residual in X, Y, Z",
        "Fitted − target, in mm",
        Kind::Text { max_len: 60 },
    ),
];

const fn qout(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
    d: u8,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u }).precision(Precision::Decimals(d))
}

pub static HELMERT_3D_FIT: ToolDef = ToolDef {
    id: "survey.cogo.helmert-3d-fit",
    title: "3D conformal (Helmert) fit from common points",
    summary: "The seven Helmert parameters (three translations, three rotations, and a scale) that carry one set of 3D coordinates onto another with the least squared misfit, with standard errors and per-point residuals.",
    aliases: &[
        "7 parameter fit",
        "Bursa-Wolf estimation",
        "3D conformal transformation fit",
    ],
    keywords: &[
        "Helmert",
        "Bursa",
        "conformal",
        "transformation",
        "fit",
        "least",
        "squares",
        "datum",
    ],
    inputs: &[
        Field::new(
            "points",
            "Common points",
            "Each with source and target X, Y, Z, at least 3",
            Kind::List {
                items: POINT,
                min: 3,
                max: MAX_POINTS,
            },
        )
        .required()
        .core(),
        Field::new(
            "convention",
            "Rotation convention",
            "position-vector (EPSG 1033, the default) or coordinate-frame (EPSG 1032)",
            Kind::Choice(&["position-vector", "coordinate-frame"]),
        )
        .core(),
    ],
    outputs: &[
        qout("tx", "Translation X", "", QT::Length, "m", 4),
        qout("ty", "Translation Y", "", QT::Length, "m", 4),
        qout("tz", "Translation Z", "", QT::Length, "m", 4),
        qout("rx", "Rotation X", "", QT::Angle, "arcsec", 5),
        qout("ry", "Rotation Y", "", QT::Angle, "arcsec", 5),
        qout("rz", "Rotation Z", "", QT::Angle, "arcsec", 5),
        Field::new(
            "scale",
            "Scale change",
            "Parts per million",
            Kind::Quantity {
                q: QT::Dimensionless,
                unit: "ppm",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "convention",
            "Rotation convention",
            "As fitted",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "standard_errors",
            "Standard errors",
            "Of the seven parameters",
            Kind::Text { max_len: 200 },
        )
        .optional(),
        qout("rms", "RMS residual", "Over all axes", QT::Length, "mm", 2),
        Field::new(
            "reference_variance",
            "Reference variance",
            "Residual sum of squares over the degrees of freedom, in m²",
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
            "3 × points − 7",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "residuals",
            "Residuals",
            "Per point",
            Kind::List {
                items: OUT_RESIDUAL,
                min: 0,
                max: MAX_POINTS,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::DegenerateGeometry,
        ErrorCode::NoSolution,
    ],
    warnings: &[
        "NO_REDUNDANCY",
        "SCALE_SUSPECT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Least squares on V_T = T + (1 + dS) · (I + Ω) · V_S (IOGP GN 7-2 §4.2.3), Ω the small-angle rotation matrix (signs reversed for the coordinate-frame convention); Gauss-Newton on centroid-reduced coordinates from zero, to 1e-15; σ0² = vᵀv / (3n − 7); standard errors from σ0² N⁻¹",
    accuracy: "Exact least squares for the linearized rotation model the Helmert tool uses; valid for the small rotations of datum transformations (well under a degree)",
    when_to_use: "Use this to find the seven-parameter transformation between two 3D coordinate systems from points known in both, such as control coordinates in an old and a new datum, or a scan or local survey registered to ECEF. It reports the parameters with standard errors and every point's residual, so a bad common point shows itself. Send the parameters to the Helmert tool to transform other points.",
    limitations: "It uses the small-angle rotation model of geodetic datum transformations, so it is for rotations of seconds to minutes of arc, not for registering systems turned by degrees. All points weigh equally. With 3 points there are only 2 degrees of freedom, too few to judge the fit; use more points spread over the area. Source and target must be the same kind of coordinates (both ECEF, or both one local 3D system) and in the same unit.",
    references: &[IOGP],
    examples: &[Example {
        id: "primary",
        title: "Five common points 2 km apart, a sub-meter shift",
        input: r#"{"points":[{"name":"P1","x":"4027189.3311 m","y":"305648.9967 m","z":"4920078.6379 m","x_target":"4027190.3555 m","y_target":"305651.3552 m","z_target":"4920084.8697 m"},{"name":"P2","x":"4026183.7451 m","y":"307189.1280 m","z":"4918937.6557 m","x_target":"4026184.7673 m","y_target":"307191.4873 m","z_target":"4918943.8870 m"},{"name":"P3","x":"4026125.9957 m","y":"307075.3429 m","z":"4917624.8826 m","x_target":"4026127.0184 m","y_target":"307077.7028 m","z_target":"4917631.1128 m"},{"name":"P4","x":"4027628.5827 m","y":"305325.0217 m","z":"4917837.7521 m","x_target":"4027629.6088 m","y_target":"305327.3818 m","z_target":"4917843.9820 m"},{"name":"P5","x":"4027592.0768 m","y":"308353.0085 m","z":"4917970.1078 m","x_target":"4027593.0990 m","y_target":"308355.3710 m","z_target":"4917976.3396 m"}]}"#,
        source: "Synthetic points from known parameters (tools/vectors/gen_helmert_fit.py), rounded to 0.1 mm",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.datum.helmert",
            reason: "next",
        },
        Related {
            id: "survey.gnss.localization",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.least-squares-2d",
            reason: "alternative",
        },
    ],
    sentence: "The fit's residuals are {rms} RMS, with a scale change of {scale}.{warn SCALE_SUSPECT} The scale is far from one: check the units and the points.{/warn}",
    limits: &[("batchRows", 1)],
    run: run_helmert_3d_fit,
    ..ToolDef::BLANK
};

type V3 = [f64; 3];

/// The rotation part (I + Ω) V for rotations r (radians), in the convention.
fn rotate(r: V3, v: V3, cf: bool) -> V3 {
    let r = if cf { [-r[0], -r[1], -r[2]] } else { r };
    [
        v[0] - r[2] * v[1] + r[1] * v[2],
        r[2] * v[0] + v[1] - r[0] * v[2],
        -r[1] * v[0] + r[0] * v[1] + v[2],
    ]
}

/// Solves the 7 × 7 symmetric positive-definite system by Cholesky, returning
/// the solution and the inverse; None when it is singular.
fn solve(n: &[[f64; 7]; 7], b: &[f64; 7]) -> Option<([f64; 7], [[f64; 7]; 7])> {
    let mut l = [[0.0; 7]; 7];
    for i in 0..7 {
        for j in 0..=i {
            let s: f64 = n[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            if i == j {
                if s <= n[i][i].abs() * 1e-14 || !s.is_finite() {
                    return None;
                }
                l[i][i] = sqrt(s);
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    let fwd = |b: &[f64; 7]| {
        let mut y = [0.0; 7];
        for i in 0..7 {
            y[i] = (b[i] - (0..i).map(|k| l[i][k] * y[k]).sum::<f64>()) / l[i][i];
        }
        let mut x = [0.0; 7];
        for i in (0..7).rev() {
            x[i] = (y[i] - (i + 1..7).map(|k| l[k][i] * x[k]).sum::<f64>()) / l[i][i];
        }
        x
    };
    let x = fwd(b);
    let mut inv = [[0.0; 7]; 7];
    for c in 0..7 {
        let mut e = [0.0; 7];
        e[c] = 1.0;
        let col = fwd(&e);
        for r in 0..7 {
            inv[r][c] = col[r];
        }
    }
    Some((x, inv))
}

fn run_helmert_3d_fit(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let rows = ctx.rows("points")?;
    let mut names = vec![];
    let (mut src, mut dst): (Vec<V3>, Vec<V3>) = (vec![], vec![]);
    for (i, r) in rows.iter().enumerate() {
        let mut get = |k: &str| -> Result<f64, ToolError> {
            Ok(ctx
                .row_quantity("points", i, r, k)?
                .expect("required")
                .to(m))
        };
        let s = [get("x")?, get("y")?, get("z")?];
        let t = [get("x_target")?, get("y_target")?, get("z_target")?];
        if s.iter().chain(&t).any(|v| !v.is_finite()) {
            return Err(ToolError::invalid(
                &format!("/points/{i}"),
                "Coordinates must be numbers.",
            ));
        }
        names.push(
            ctx.row_text("points", i, r, "name")?
                .unwrap_or_else(|| format!("{}", i + 1)),
        );
        src.push(s);
        dst.push(t);
    }
    let n = src.len();
    if n < 3 {
        return Err(ToolError::invalid(
            "/points",
            "A 7-parameter fit needs at least 3 common points.",
        ));
    }
    let cf = ctx.choice("convention")? == Some("coordinate-frame");
    let mean = |p: &[V3]| {
        let mut c = [0.0; 3];
        for v in p {
            for k in 0..3 {
                c[k] += v[k] / n as f64;
            }
        }
        c
    };
    let (cs, ct) = (mean(&src), mean(&dst));
    let sc: Vec<V3> = src
        .iter()
        .map(|v| [v[0] - cs[0], v[1] - cs[1], v[2] - cs[2]])
        .collect();
    let tc: Vec<V3> = dst
        .iter()
        .map(|v| [v[0] - ct[0], v[1] - ct[1], v[2] - ct[2]])
        .collect();
    let spread = sc
        .iter()
        .map(|v| sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]))
        .fold(0.0, f64::max);
    if spread < 1e-6 {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The source points coincide; spread them over the area.",
        )
        .at("/points"));
    }
    // Parameters on centered coordinates: t'' (3), s, r (3). The rotation
    // derivatives below are those of the position-vector Ω; the coordinate-
    // frame convention flips their sign.
    let mut p = [0.0f64; 7];
    let sign = if cf { -1.0 } else { 1.0 };
    let mut inv: [[f64; 7]; 7];
    let mut iterations = 0;
    loop {
        iterations += 1;
        let (mut nm, mut b) = ([[0.0; 7]; 7], [0.0; 7]);
        let r = [p[4], p[5], p[6]];
        for (v, t) in sc.iter().zip(&tc) {
            let rv = rotate(r, *v, cf);
            let f: V3 = [0, 1, 2].map(|k| p[k] + (1.0 + p[3]) * rv[k] - t[k]);
            let k1 = (1.0 + p[3]) * sign;
            let jac: [[f64; 7]; 3] = [
                [1.0, 0.0, 0.0, rv[0], 0.0, k1 * v[2], -k1 * v[1]],
                [0.0, 1.0, 0.0, rv[1], -k1 * v[2], 0.0, k1 * v[0]],
                [0.0, 0.0, 1.0, rv[2], k1 * v[1], -k1 * v[0], 0.0],
            ];
            for a in 0..3 {
                for i in 0..7 {
                    b[i] -= jac[a][i] * f[a];
                    for j in 0..7 {
                        nm[i][j] += jac[a][i] * jac[a][j];
                    }
                }
            }
        }
        let Some((dx, ninv)) = solve(&nm, &b) else {
            return Err(ToolError::new(ErrorCode::DegenerateGeometry, "The points do not fix all seven parameters: they lie on a line. Add points off it.").at("/points"));
        };
        inv = ninv;
        for i in 0..7 {
            p[i] += dx[i];
        }
        let step = dx[..3]
            .iter()
            .map(|d| d.abs() / spread)
            .chain(dx[3..].iter().map(|d| d.abs()))
            .fold(0.0, f64::max);
        if step < 1e-15 || iterations >= 20 {
            if step > 1e-9 {
                return Err(ToolError::new(ErrorCode::NoSolution, "The fit did not settle: the rotations are too large for the small-angle model.").at("/points"));
            }
            break;
        }
    }
    let r = [p[4], p[5], p[6]];
    // Back to uncentered translations: T = c_t + t'' − (1 + s)(I + Ω) c_s.
    let rc = rotate(r, cs, cf);
    let t = [0, 1, 2].map(|k| ct[k] + p[k] - (1.0 + p[3]) * rc[k]);
    let mut vtv = 0.0;
    let mut residuals = vec![];
    for (i, (v, w)) in src.iter().zip(&dst).enumerate() {
        let rv = rotate(r, *v, cf);
        let d: V3 = [0, 1, 2].map(|k| t[k] + (1.0 + p[3]) * rv[k] - w[k]);
        vtv += d.iter().map(|x| x * x).sum::<f64>();
        let mm = |x: f64| display::number(x * 1000.0, Precision::Decimals(1), ctx.options.format);
        residuals.push(Json::obj([
            ("name", Json::str(names[i].clone())),
            (
                "residual",
                Json::str(format!("{}, {}, {} mm", mm(d[0]), mm(d[1]), mm(d[2]))),
            ),
        ]));
    }
    let dof = 3 * n - 7;
    let rms = sqrt(vtv / (3 * n) as f64);
    let scale_ppm = p[3] * 1e6;
    if dof < 3 {
        ctx.warnings.push(Warning::new(
            "NO_REDUNDANCY",
            format!("Only {dof} degrees of freedom: too few to judge the fit. Add common points."),
        ));
    }
    if scale_ppm.abs() > 100.0 {
        ctx.warnings.push(Warning::new("SCALE_SUSPECT", format!("The scale differs from one by {} ppm, more than 100 ppm: check the units and the points.", display::number(scale_ppm, Precision::Decimals(1), ctx.options.format))));
    }
    let q = |v: f64, qt, u| Q {
        value: v,
        unit: unit(qt, u),
    };
    let mut out = vec![
        ("tx", ctx.out("tx", q(t[0], QT::Length, "m"))),
        ("ty", ctx.out("ty", q(t[1], QT::Length, "m"))),
        ("tz", ctx.out("tz", q(t[2], QT::Length, "m"))),
        ("rx", ctx.out("rx", q(r[0] * ARCSEC, QT::Angle, "arcsec"))),
        ("ry", ctx.out("ry", q(r[1] * ARCSEC, QT::Angle, "arcsec"))),
        ("rz", ctx.out("rz", q(r[2] * ARCSEC, QT::Angle, "arcsec"))),
        (
            "scale",
            ctx.out("scale", q(scale_ppm, QT::Dimensionless, "ppm")),
        ),
        (
            "convention",
            Json::str(if cf {
                "coordinate-frame"
            } else {
                "position-vector"
            }),
        ),
    ];
    if dof > 0 {
        let s02 = vtv / dof as f64;
        // Translation errors propagate through T = c_t + t'' − (1 + s)(I + Ω) c_s.
        let mut a = [[0.0; 7]; 3];
        for k in 0..3 {
            a[k][k] = 1.0;
            a[k][3] = -rc[k];
        }
        let k1 = (1.0 + p[3]) * sign;
        let d = [
            [0.0, k1 * cs[2], -k1 * cs[1]],
            [-k1 * cs[2], 0.0, k1 * cs[0]],
            [k1 * cs[1], -k1 * cs[0], 0.0],
        ];
        for k in 0..3 {
            for j in 0..3 {
                a[k][4 + j] = -d[k][j];
            }
        }
        let var_t = |k: usize| {
            (0..7)
                .map(|i| (0..7).map(|j| a[k][i] * inv[i][j] * a[k][j]).sum::<f64>())
                .sum::<f64>()
                * s02
        };
        let f = |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), ctx.options.format);
        let se = format!(
            "tx ±{} m, ty ±{} m, tz ±{} m; rx ±{}\", ry ±{}\", rz ±{}\"; scale ±{} ppm",
            f(sqrt(var_t(0)), 4),
            f(sqrt(var_t(1)), 4),
            f(sqrt(var_t(2)), 4),
            f(sqrt(inv[4][4] * s02) * ARCSEC, 5),
            f(sqrt(inv[5][5] * s02) * ARCSEC, 5),
            f(sqrt(inv[6][6] * s02) * ARCSEC, 5),
            f(sqrt(inv[3][3] * s02) * 1e6, 4),
        );
        out.push(("standard_errors", Json::str(se)));
        out.push(("reference_variance", Json::Num(s02)));
    }
    out.push(("rms", ctx.out("rms", q(rms, QT::Length, "m"))));
    out.push(("degrees_of_freedom", Json::Num(dof as f64)));
    out.push(("residuals", Json::Arr(residuals)));
    Ok(Json::obj(out))
}
