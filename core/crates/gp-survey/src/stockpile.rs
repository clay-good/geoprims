//! Stockpiles and simple solids (add-survey-suite, survey/earthwork-and-
//! grade, "Stockpile and simple solid volumes"): a stockpile's volume from a
//! Delaunay TIN over its base polygon and surface points, measured above the
//! plane fitted to the base; and cones, frustums, and prisms from dimensions.
//! No angle-of-repose table is given: the reader enters the angle and its source.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::point::plain_angle;
use serde_json::{Map, Value};

use crate::{common_unit, len, len_out, unit};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapter 26 (volumes: TIN-based volumes and volumes of solids)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const XYZ: &[Field] = &[
    len("easting", "Easting", "Like 1000.0 ft").required(),
    len("northing", "Northing", "Like 2000.0 ft").required(),
    len("elevation", "Elevation", "Like 101.5 ft").required(),
];

type P3 = (f64, f64, f64);

/// Delaunay triangulation by Bowyer–Watson: triangles as index triples.
pub(crate) fn delaunay(pts: &[(f64, f64)]) -> Vec<[usize; 3]> {
    let n = pts.len();
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in pts {
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
    }
    let d = (maxx - minx).max(maxy - miny).max(1e-9) * 20.0;
    let (cx, cy) = ((minx + maxx) / 2.0, (miny + maxy) / 2.0);
    let mut all: Vec<(f64, f64)> = pts.to_vec();
    all.extend([(cx - d, cy - d), (cx + d, cy - d), (cx, cy + d)]);
    let circum = |t: &[usize; 3]| -> (f64, f64, f64) {
        let (a, b, c) = (all[t[0]], all[t[1]], all[t[2]]);
        let dd = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
        let (a2, b2, c2) = (
            a.0 * a.0 + a.1 * a.1,
            b.0 * b.0 + b.1 * b.1,
            c.0 * c.0 + c.1 * c.1,
        );
        let ux = (a2 * (b.1 - c.1) + b2 * (c.1 - a.1) + c2 * (a.1 - b.1)) / dd;
        let uy = (a2 * (c.0 - b.0) + b2 * (a.0 - c.0) + c2 * (b.0 - a.0)) / dd;
        (ux, uy, (a.0 - ux) * (a.0 - ux) + (a.1 - uy) * (a.1 - uy))
    };
    let mut tris: Vec<([usize; 3], (f64, f64, f64))> = vec![([n, n + 1, n + 2], (0.0, 0.0, 0.0))];
    tris[0].1 = circum(&tris[0].0);
    for (i, &p) in all.iter().enumerate().take(n) {
        let (bad, keep): (Vec<_>, Vec<_>) = tris.into_iter().partition(|(_, (ux, uy, r2))| {
            (p.0 - ux) * (p.0 - ux) + (p.1 - uy) * (p.1 - uy) < *r2 * (1.0 + 1e-12)
        });
        // The hole's boundary: edges that belong to exactly one bad triangle.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (t, _) in &bad {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                if let Some(k) = edges
                    .iter()
                    .position(|&(x, y)| (x == b && y == a) || (x == a && y == b))
                {
                    edges.swap_remove(k);
                } else {
                    edges.push((a, b));
                }
            }
        }
        tris = keep;
        for (a, b) in edges {
            let t = [a, b, i];
            tris.push((t, circum(&t)));
        }
    }
    tris.into_iter()
        .map(|(t, _)| t)
        .filter(|t| t.iter().all(|&v| v < n))
        .collect()
}

fn inside(poly: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut c = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi, xj, yj) = (poly[i].0, poly[i].1, poly[j].0, poly[j].1);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            c = !c;
        }
        j = i;
    }
    c
}

/// The least-squares plane z = a + b·x + c·y through the base vertices.
fn plane(base: &[P3]) -> Option<(f64, f64, f64)> {
    let n = base.len() as f64;
    let (mx, my, mz) = base.iter().fold((0.0, 0.0, 0.0), |s, p| {
        (s.0 + p.0 / n, s.1 + p.1 / n, s.2 + p.2 / n)
    });
    let (mut sxx, mut sxy, mut syy, mut sxz, mut syz) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for &(x, y, z) in base {
        let (dx, dy, dz) = (x - mx, y - my, z - mz);
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
        sxz += dx * dz;
        syz += dy * dz;
    }
    let det = sxx * syy - sxy * sxy;
    if det.abs() < 1e-12 * (sxx * syy).max(1e-300) {
        return None;
    }
    let b = (sxz * syy - syz * sxy) / det;
    let c = (syz * sxx - sxz * sxy) / det;
    Some((mz - b * mx - c * my, b, c))
}

pub static STOCKPILE: ToolDef = ToolDef {
    id: "survey.earthwork.stockpile",
    title: "Stockpile volume (TIN)",
    summary: "A stockpile's volume from its base outline and surface shots: a Delaunay TIN measured above the plane fitted to the base, in cubic units and cubic yards.",
    aliases: &[
        "stockpile volume",
        "pile volume",
        "TIN volume",
        "stockpile survey",
    ],
    keywords: &[
        "stockpile",
        "pile",
        "TIN",
        "Delaunay",
        "volume",
        "base",
        "surface",
        "aggregate",
        "earthwork",
    ],
    inputs: &[
        Field::new(
            "base",
            "Base outline",
            "The toe of the pile, in order around it: easting, northing, elevation, like 0, 0, 100",
            Kind::List {
                items: XYZ,
                min: 3,
                max: 10_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "surface",
            "Surface shots",
            "Points on the pile: easting, northing, elevation, like 25, 25, 108.2",
            Kind::List {
                items: XYZ,
                min: 1,
                max: 20_000,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "volume",
            "Volume",
            "Above the base plane",
            Kind::Quantity {
                q: QT::Volume,
                unit: "ft3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "cubic_yards",
            "Volume, cubic yards",
            "The same volume",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "base_area",
            "Base area",
            "Inside the outline",
            Kind::Quantity {
                q: QT::Area,
                unit: "ft2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "triangles",
            "Triangles",
            "In the TIN inside the base",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        len_out(
            "max_height",
            "Highest point above the base",
            "Of the shots given",
        ),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Delaunay TIN (Bowyer–Watson) over the base vertices and surface shots; base plane z = a + bx + cy by least squares through the base vertices; volume = Σ triangle area × mean height above the plane, over triangles inside the outline (Ghilani & Wolf 2018, ch. 26)",
    accuracy: "Exact for the TIN; a real pile's volume depends on how densely its surface was shot, especially along ridges and breaks",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A pile on a 50 ft square base, peaking 10 ft up",
        input: r#"{"base":[{"easting":"0 ft","northing":"0 ft","elevation":"100 ft"},{"easting":"50 ft","northing":"0 ft","elevation":"100 ft"},{"easting":"50 ft","northing":"50 ft","elevation":"100 ft"},{"easting":"0 ft","northing":"50 ft","elevation":"100 ft"}],"surface":[{"easting":"25 ft","northing":"25 ft","elevation":"110 ft"}]}"#,
        source: "A square pyramid: base area × height / 3 = 8,333.33 ft³",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.earthwork.solid-volume",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.shrink-swell",
            reason: "next",
        },
    ],
    sentence: "The stockpile holds {volume}, or {cubic_yards}.",
    limits: &[("batchRows", 100)],
    run: run_stockpile,
    ..ToolDef::BLANK
};

fn read_xyz(
    ctx: &mut Ctx,
    list: &str,
    qs: &mut Vec<(&'static str, Q)>,
) -> Result<Vec<(Q, Q, Q)>, ToolError> {
    let rows: Vec<Map<String, Value>> = ctx.rows(list)?;
    let mut out = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let e = ctx
            .row_quantity(list, i, row, "easting")?
            .expect("required");
        let n = ctx
            .row_quantity(list, i, row, "northing")?
            .expect("required");
        let z = ctx
            .row_quantity(list, i, row, "elevation")?
            .expect("required");
        qs.extend([("easting", e), ("northing", n), ("elevation", z)]);
        out.push((e, n, z));
    }
    Ok(out)
}

fn volume_unit(u: &gp_base::units::Unit) -> (&'static str, &'static str) {
    if u.symbol.starts_with("ft") {
        ("ft3", "ft2")
    } else {
        ("m3", "m2")
    }
}

fn run_stockpile(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut qs = Vec::new();
    let base = read_xyz(ctx, "base", &mut qs)?;
    let surf = read_xyz(ctx, "surface", &mut qs)?;
    let u = common_unit(&qs)?;
    let to = |v: &[(Q, Q, Q)]| -> Vec<P3> {
        v.iter()
            .map(|(e, n, z)| (e.to(u), n.to(u), z.to(u)))
            .collect()
    };
    let (base, surf) = (to(&base), to(&surf));
    let Some((a, b, c)) = plane(&base) else {
        return Err(ToolError::invalid(
            "/base",
            "The base points lie on a line, so they fix no base plane.",
        ));
    };
    let outline: Vec<(f64, f64)> = base.iter().map(|p| (p.0, p.1)).collect();
    let pts: Vec<P3> = base.iter().chain(surf.iter()).copied().collect();
    for (i, p) in pts.iter().enumerate() {
        if pts[..i]
            .iter()
            .any(|q| (q.0 - p.0).abs() < 1e-9 && (q.1 - p.1).abs() < 1e-9)
        {
            let (list, k) = if i < base.len() {
                ("base", i)
            } else {
                ("surface", i - base.len())
            };
            return Err(ToolError::invalid(
                &format!("/{list}/{k}"),
                "Two points share a position; give each position once.",
            ));
        }
    }
    let height = |p: &P3| p.2 - (a + b * p.0 + c * p.1);
    let xy: Vec<(f64, f64)> = pts.iter().map(|p| (p.0, p.1)).collect();
    let (mut vol, mut area, mut count) = (0.0, 0.0, 0.0);
    for t in delaunay(&xy) {
        let (p, q, r) = (pts[t[0]], pts[t[1]], pts[t[2]]);
        let (gx, gy) = ((p.0 + q.0 + r.0) / 3.0, (p.1 + q.1 + r.1) / 3.0);
        if !inside(&outline, gx, gy) {
            continue;
        }
        let ar = ((q.0 - p.0) * (r.1 - p.1) - (r.0 - p.0) * (q.1 - p.1)).abs() / 2.0;
        vol += ar * (height(&p) + height(&q) + height(&r)) / 3.0;
        area += ar;
        count += 1.0;
    }
    if count == 0.0 {
        return Err(ToolError::invalid(
            "/surface",
            "No part of the surface lies inside the base outline.",
        ));
    }
    let per_m = Q {
        value: 1.0,
        unit: u,
    }
    .to(unit(QT::Length, "m"));
    let (vs, asym) = volume_unit(u);
    let v3 = Q {
        value: vol * per_m * per_m * per_m,
        unit: unit(QT::Volume, "m3"),
    };
    let a2 = Q {
        value: area * per_m * per_m,
        unit: unit(QT::Area, "m2"),
    };
    let top = surf.iter().map(height).fold(f64::MIN, f64::max);
    Ok(Json::obj(vec![
        ("volume", ctx.emit("volume", v3, unit(QT::Volume, vs))),
        (
            "cubic_yards",
            ctx.emit("cubic_yards", v3, unit(QT::Volume, "yd3")),
        ),
        ("base_area", ctx.emit("base_area", a2, unit(QT::Area, asym))),
        ("triangles", Json::Num(count)),
        (
            "max_height",
            ctx.emit(
                "max_height",
                Q {
                    value: top,
                    unit: u,
                },
                u,
            ),
        ),
    ]))
}

pub static SOLID: ToolDef = ToolDef {
    id: "survey.earthwork.solid-volume",
    title: "Cone, frustum, and prism volumes",
    summary: "The volume of a cone, frustum, or prism from its dimensions, or of a conical pile from its base and an angle of repose you enter with its source.",
    aliases: &[
        "cone volume",
        "frustum volume",
        "conical pile volume",
        "prism volume",
    ],
    keywords: &[
        "cone",
        "frustum",
        "prism",
        "pile",
        "angle of repose",
        "volume",
        "solid",
    ],
    inputs: &[
        Field::new(
            "shape",
            "Shape",
            "cone, frustum, or prism",
            Kind::Choice(&["cone", "frustum", "prism"]),
        )
        .required()
        .core(),
        len("radius", "Base radius", "Cone or frustum, like 20 ft").core(),
        len("height", "Height", "Like 12 ft").core(),
        len("top_radius", "Top radius", "Frustum, like 8 ft"),
        Field::new(
            "base_area",
            "Base area",
            "Prism: the cross-section, like 150 ft²",
            Kind::Quantity {
                q: QT::Area,
                unit: "ft2",
            },
        ),
        Field::new(
            "repose_angle",
            "Angle of repose",
            "Instead of the height, for a cone: the angle you measured or looked up, like 34°",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded"),
        Field::new(
            "repose_source",
            "Angle source",
            "Where the angle comes from, like measured on site 2026-09-20",
            Kind::Text { max_len: 120 },
        ),
    ],
    outputs: &[
        Field::new(
            "volume",
            "Volume",
            "Of the solid",
            Kind::Quantity {
                q: QT::Volume,
                unit: "ft3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "cubic_yards",
            "Volume, cubic yards",
            "The same volume",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .precision(Precision::Decimals(2)),
        len_out(
            "height_used",
            "Height",
            "Given, or from the angle of repose",
        )
        .optional(),
        Field::new(
            "repose_source",
            "Angle source",
            "As you cited it",
            Kind::Text { max_len: 120 },
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Cone πr²h/3; frustum πh(R² + Rr + r²)/3; prism A·h; a conical pile's height r·tan(angle of repose)",
    accuracy: "Exact for the shape; real piles are rarely perfect cones",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A cone 20 ft in radius and 12 ft high",
        input: r#"{"shape":"cone","radius":"20 ft","height":"12 ft"}"#,
        source: "πr²h/3 = 5,026.55 ft³",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.earthwork.stockpile",
        reason: "alternative",
    }],
    sentence: "The volume is {volume}, or {cubic_yards}.",
    limits: &[("batchRows", 10_000)],
    run: run_solid,
    ..ToolDef::BLANK
};

fn run_solid(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let shape = ctx.choice("shape")?.expect("required");
    let (r, h, rt) = (
        ctx.quantity("radius")?,
        ctx.quantity("height")?,
        ctx.quantity("top_radius")?,
    );
    let mut qs = Vec::new();
    for (nm, q) in [("radius", r), ("height", h), ("top_radius", rt)] {
        qs.extend(q.map(|q| (nm, q)));
    }
    let area = ctx.quantity("base_area")?;
    let u = if qs.is_empty() {
        unit(QT::Length, "ft")
    } else {
        common_unit(&qs)?
    };
    let pos = |q: Option<Q>, f: &str| -> Result<f64, ToolError> {
        let v = q.map(|q| q.to(u)).ok_or_else(|| {
            ToolError::invalid(&format!("/{f}"), "This shape needs that dimension.")
        })?;
        if v > 0.0 {
            Ok(v)
        } else {
            Err(ToolError::invalid(
                &format!("/{f}"),
                "Dimensions must be positive.",
            ))
        }
    };
    let pi = core::f64::consts::PI;
    let mut source = None;
    let (v, hused) = match shape {
        "cone" => {
            let rr = pos(r, "radius")?;
            let hh = match (h, plain_angle(ctx, "repose_angle")?) {
                (Some(_), Some(_)) => {
                    return Err(ToolError::invalid(
                        "/repose_angle",
                        "Give the height or the angle of repose, not both.",
                    ));
                }
                (Some(_), None) => pos(h, "height")?,
                (None, Some(a)) => {
                    if !(0.0 < a && a < 90.0) {
                        return Err(ToolError::invalid(
                            "/repose_angle",
                            "An angle of repose is between 0° and 90°.",
                        ));
                    }
                    source = Some(ctx.text("repose_source")?.ok_or_else(|| ToolError::invalid("/repose_source", "Say where the angle of repose comes from: published values vary widely and are not design values."))?);
                    rr * libm::tan(a.to_radians())
                }
                (None, None) => {
                    return Err(ToolError::invalid(
                        "/height",
                        "Give the cone's height, or its angle of repose.",
                    ));
                }
            };
            (pi * rr * rr * hh / 3.0, hh)
        }
        "frustum" => {
            let (big, small, hh) = (pos(r, "radius")?, pos(rt, "top_radius")?, pos(h, "height")?);
            (
                pi * hh * (big * big + big * small + small * small) / 3.0,
                hh,
            )
        }
        _ => {
            let hh = pos(h, "height")?;
            let a = area.ok_or_else(|| {
                ToolError::invalid("/base_area", "A prism needs its cross-section area.")
            })?;
            let per_m = Q {
                value: 1.0,
                unit: u,
            }
            .to(unit(QT::Length, "m"));
            let a_u2 = a.to(unit(QT::Area, "m2")) / (per_m * per_m);
            if a_u2 <= 0.0 {
                return Err(ToolError::invalid(
                    "/base_area",
                    "The area must be positive.",
                ));
            }
            (a_u2 * hh, hh)
        }
    };
    let per_m = Q {
        value: 1.0,
        unit: u,
    }
    .to(unit(QT::Length, "m"));
    let v3 = Q {
        value: v * per_m * per_m * per_m,
        unit: unit(QT::Volume, "m3"),
    };
    let (vs, _) = volume_unit(u);
    let mut out = vec![
        ("volume", ctx.emit("volume", v3, unit(QT::Volume, vs))),
        (
            "cubic_yards",
            ctx.emit("cubic_yards", v3, unit(QT::Volume, "yd3")),
        ),
        (
            "height_used",
            ctx.emit(
                "height_used",
                Q {
                    value: hused,
                    unit: u,
                },
                u,
            ),
        ),
    ];
    if let Some(s) = source {
        out.push(("repose_source", Json::Str(s)));
    }
    Ok(Json::obj(out))
}
