//! Ground control and checkpoints (drone/mission-planning spec, "Ground
//! control and checkpoints"). The checkpoint count is ASPRS Edition 2, Annex
//! C, Table C.1 by product area; the accuracy the control and checkpoints need
//! is sections 7.9 and 7.12. The GCP layout is a stated rule of thumb (Pix4D):
//! a point inset from each corner, one near the middle, and optionally more
//! on a grid, every one tested inside the area with the point-in-polygon test
//! the buffer tools use.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::inside_rings;
use libm::{acos, ceil, hypot, round, sin, sqrt};

use crate::mission::{P, VERTEX, read_polygon};

const ASPRS: Reference = Reference {
    title: "ASPRS Positional Accuracy Standards for Digital Geospatial Data, Edition 2",
    issuer: "American Society for Photogrammetry and Remote Sensing",
    year: 2023,
    edition: "Edition 2 (Version 1.0, February 2023, as read; Version 2.0 was approved in 2024)",
    locator: "Section 7.9 (ground control accuracy), 7.12 and 7.13 (checkpoint accuracy, density, and distribution), Annex C.3 and Table C.1 (recommended number of checkpoints based on area)",
    url: "https://asprs.org/Main/Main/Standards/Positional-Accuracy-Standards.aspx",
};
const PIX4D_GCP: Reference = Reference {
    title: "Getting GCPs on the field or through other sources",
    issuer: "Pix4D support documentation (PIX4Dmapper)",
    year: 2025,
    edition: "Vendor guidance, not a standard",
    locator: "Step 1, part 4: at least 3 GCPs, 5 to 10 usually enough; place them evenly, one in the center, and not exactly at the edges",
    url: "https://support.pix4d.com/hc/en-us/articles/202557489",
};

/// ASPRS Edition 2, Table C.1: checkpoints for a land-cover area (km²),
/// counted in whole square kilometers as the table is. None past 2,500 km²,
/// where the table ends.
pub fn table_c1(km2: f64) -> Option<u32> {
    let a = round(km2);
    if a <= 500.0 {
        Some(30)
    } else if a <= 2500.0 {
        Some(30 + 5 * ceil((a - 500.0) / 250.0) as u32)
    } else {
        None
    }
}

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
    }
}

const fn count(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Number { min: 0.0, max: 1e6 })
        .precision(Precision::Decimals(0))
}

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

const fn degrees(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7))
}

const GCP_ROW: &[Field] = &[
    count("point", "Point", "Its number"),
    degrees("lat", "Latitude"),
    degrees("lon", "Longitude"),
    Field::new(
        "place",
        "Place",
        "corner, middle, or grid",
        Kind::Text { max_len: 8 },
    ),
];
const CHECK_ROW: &[Field] = &[
    count("point", "Point", "Its number"),
    degrees("lat", "Latitude"),
    degrees("lon", "Longitude"),
];

pub static GCP_PLAN: ToolDef = ToolDef {
    id: "drone.photogrammetry.gcp-plan",
    title: "GCP and checkpoint plan",
    summary: "How many checkpoints an area needs under the ASPRS Positional Accuracy Standards, Edition 2, how accurate the ground control must be, and a suggested layout of ground control points and checkpoints inside the area.",
    aliases: &[
        "how many ground control points",
        "how many checkpoints ASPRS",
        "GCP layout",
        "ground control point plan",
    ],
    keywords: &[
        "GCP",
        "ground control",
        "checkpoints",
        "ASPRS",
        "accuracy",
        "photogrammetry",
        "survey control",
        "layout",
    ],
    inputs: &[
        Field::new(
            "area",
            "Area",
            "Corners of the product area (ring 0) and any holes (ring 1, 2, …), in order, like 40, -105",
            Kind::List {
                items: VERTEX,
                min: 3,
                max: 2000,
            },
        )
        .required()
        .core(),
        cm(
            "horizontal_class",
            "Horizontal accuracy class",
            "The product's RMSE_H target, like 5 cm",
        )
        .core(),
        cm(
            "vertical_class",
            "Vertical accuracy class",
            "The elevation product's RMSE_V target, if any, like 10 cm",
        )
        .core(),
        Field::new(
            "vegetated",
            "Vegetated share (%)",
            "Of the area, for the added vegetated checkpoints, like 30; default 0",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .core(),
        Field::new(
            "spacing",
            "GCP grid spacing",
            "Extra control inside the area this far apart, like 300 m; default none",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .core(),
        Field::new(
            "inset",
            "Inset from the edge",
            "How far inside the boundary points go, like 10 m (the default)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
    ],
    outputs: &[
        count(
            "checkpoints",
            "Checkpoints",
            "ASPRS Table C.1, open and vegetated ground together",
        ),
        count(
            "checkpoints_open",
            "Checkpoints on open ground",
            "Horizontal and NVA testing",
        ),
        count(
            "checkpoints_vegetated",
            "Checkpoints in vegetation",
            "Added for VVA testing",
        )
        .optional(),
        count("gcps", "Ground control points", "In the suggested layout"),
        Field::new(
            "area_size",
            "Area",
            "Of the polygon, less any holes",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Decimals(3)),
        cm(
            "gcp_rmse_h",
            "GCP horizontal accuracy",
            "RMSE_H at most, ASPRS 7.9",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        cm(
            "gcp_rmse_v",
            "GCP vertical accuracy",
            "RMSE_V at most, ASPRS 7.9",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        cm(
            "checkpoint_rmse_h",
            "Checkpoint horizontal accuracy",
            "RMSE_H at most, ASPRS 7.12",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        cm(
            "checkpoint_rmse_v",
            "Checkpoint vertical accuracy",
            "RMSE_V at most, ASPRS 7.12",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "layout",
            "Layout",
            "How the points were placed",
            Kind::Text { max_len: 400 },
        ),
        Field::new(
            "gcp_points",
            "Ground control points",
            "Suggested spots, inside the area",
            Kind::List {
                items: GCP_ROW,
                min: 0,
                max: 5_000,
            },
        ),
        Field::new(
            "checkpoint_points",
            "Checkpoints",
            "Suggested spots, spread evenly inside the area",
            Kind::List {
                items: CHECK_ROW,
                min: 0,
                max: 1_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Checkpoints: ASPRS Edition 2, Table C.1, on the open and on the vegetated share of the area, each in whole km²: 30 up to 500 km², then 5 more per 250 km², to 70 at 2,500 km². Control accuracy (7.9): RMSE_H(GCP) ≤ ½ RMSE_H(product); RMSE_V(GCP) ≤ ½ RMSE_V(DEM), or ≤ RMSE_H(product) for planimetric work only. Checkpoints twice as accurate as the product (7.12). Layout, a rule of thumb after Pix4D: one GCP inset from each corner that turns more than 20°, one at the point farthest from the edges, and a grid at the spacing given; checkpoints on an even grid between them. Every point passes the even-odd point-in-polygon test, clear of the edges by the inset, on a local transverse Mercator plane",
    accuracy: "Counts are the standard's table values. Positions are suggestions: every one is inside the area and at least the inset from its edges, but not checked for ground cover, slope, access, or visibility from the air.",
    when_to_use: "Use this when you plan ground control for a drone mapping job and will report accuracy by ASPRS Edition 2: it says how many checkpoints the area needs, how accurately the control and checkpoints must be surveyed for your accuracy class, and suggests where to put the targets, as coordinates you can load into a data collector.",
    limitations: "The checkpoint count is the standard's recommendation by area; a client can ask for more. The layout is a rule of thumb, not a requirement of the standard, and it cannot see the ground: move points off roofs, slopes over 10%, vegetation, and places a target cannot be seen from the air. It does not say how many GCPs a given accuracy needs; that depends on the camera, the flight, and the software.",
    references: &[ASPRS, PIX4D_GCP],
    examples: &[Example {
        id: "primary",
        title: "An L-shaped 6 ha site for a 5 cm map and a 10 cm surface",
        input: r#"{"area":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99648},{"lat":40.0009,"lon":-104.99648},{"lat":40.0009,"lon":-104.99824},{"lat":40.0027,"lon":-104.99824},{"lat":40.0027,"lon":-105.0}],"horizontal_class":"5 cm","vertical_class":"10 cm"}"#,
        source: "ASPRS Edition 2, Table C.1: 30 checkpoints for an area of 500 km² or less; 7.9: GCPs to 2.5 cm horizontal and 5 cm vertical",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("area", "area_size")],
        },
        Layer {
            kind: "point",
            map: &[("marks", "gcp_points")],
        },
        Layer {
            kind: "point",
            map: &[("points", "checkpoint_points")],
        },
    ],
    related: &[
        Related {
            id: "drone.photogrammetry.asprs-accuracy",
            reason: "next",
        },
        Related {
            id: "drone.mission.survey-grid",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
    ],
    sentence: "Survey {checkpoints} checkpoints and about {gcps} ground control points.{if gcp_rmse_h > 0} Set the control to {gcp_rmse_h} horizontally or better.{/if} The layout is a rule of thumb.",
    assumptions: &[
        Assumption {
            name: "Checkpoints for an area of 500 km² or less (Table C.1)",
            value: "30",
            unit: "1",
            source: "asprs-pas",
        },
        Assumption {
            name: "Checkpoints added per further 250 km², to 2,500 km² (Table C.1)",
            value: "5",
            unit: "1",
            source: "asprs-pas",
        },
    ],
    limits: &[("batchRows", 100)],
    run: run_gcp,
    ..ToolDef::BLANK
};

/// Distance from `p` to the nearest edge of any ring.
fn edge_distance(rings: &[Vec<P>], p: P) -> f64 {
    let mut best = f64::INFINITY;
    for r in rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let l2 = dx * dx + dy * dy;
            let t = if l2 > 0.0 {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            best = best.min(hypot(p.0 - a.0 - t * dx, p.1 - a.1 - t * dy));
        }
    }
    best
}

/// Inside the area (holes are outside) and at least `clear` from every edge.
fn fits(rings: &[Vec<P>], p: P, clear: f64) -> bool {
    inside_rings(rings, p) && edge_distance(rings, p) >= clear
}

fn near(pts: &[P], p: P, d: f64) -> bool {
    pts.iter().any(|q| hypot(q.0 - p.0, q.1 - p.1) < d)
}

/// A point `inset` inside each corner that turns more than 20°, along the
/// bisector of its two edges.
fn corners(rings: &[Vec<P>], inset: f64) -> Vec<P> {
    let outer = &rings[0];
    let n = outer.len();
    let mut out: Vec<P> = Vec::new();
    for i in 0..n {
        let v = outer[i];
        let (a, b) = (outer[(i + n - 1) % n], outer[(i + 1) % n]);
        let (la, lb) = (hypot(a.0 - v.0, a.1 - v.1), hypot(b.0 - v.0, b.1 - v.1));
        if la == 0.0 || lb == 0.0 {
            continue;
        }
        let ua = ((a.0 - v.0) / la, (a.1 - v.1) / la);
        let ub = ((b.0 - v.0) / lb, (b.1 - v.1) / lb);
        let turn = acos((ua.0 * ub.0 + ua.1 * ub.1).clamp(-1.0, 1.0));
        if turn > 160f64.to_radians() {
            continue; // nearly straight: not a corner
        }
        let s = (ua.0 + ub.0, ua.1 + ub.1);
        let ls = hypot(s.0, s.1);
        if ls < 1e-12 {
            continue;
        }
        let bis = (s.0 / ls, s.1 / ls);
        // Inside a convex corner the bisector points in; at a reflex corner,
        // away. Either way the point sits `inset` from both edges.
        let reach = inset / sin(turn / 2.0).max(0.2);
        let found = [1.0, -1.0].iter().find_map(|&k| {
            let p = (v.0 + k * bis.0 * reach, v.1 + k * bis.1 * reach);
            fits(rings, p, inset * 0.999).then_some(p)
        });
        if let Some(p) = found
            && !near(&out, p, 2.0 * inset)
        {
            out.push(p);
        }
    }
    out
}

/// Grid points `step` apart through `origin` over the box, that fit.
fn grid(rings: &[Vec<P>], bbox: (f64, f64, f64, f64), origin: P, step: f64, clear: f64) -> Vec<P> {
    let (x0, x1, y0, y1) = bbox;
    let i0 = ((x0 - origin.0) / step).floor() as i64;
    let i1 = ((x1 - origin.0) / step).ceil() as i64;
    let j0 = ((y0 - origin.1) / step).floor() as i64;
    let j1 = ((y1 - origin.1) / step).ceil() as i64;
    let mut out = Vec::new();
    for j in j0..=j1 {
        for i in i0..=i1 {
            let p = (origin.0 + i as f64 * step, origin.1 + j as f64 * step);
            if fits(rings, p, clear) {
                out.push(p);
            }
        }
    }
    out
}

fn run_gcp(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (plane, outer, holes) = read_polygon(ctx)?;
    let mut rings = vec![outer];
    rings.extend(holes);
    let twice = |r: &Vec<P>| -> f64 {
        (0..r.len())
            .map(|i| {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f64>()
            .abs()
    };
    let area = (twice(&rings[0]) - rings[1..].iter().map(twice).sum::<f64>()) / 2.0;
    if area <= 0.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The area has no size: check the corners are in order.",
        )
        .at("/area"));
    }
    let veg = ctx.number("vegetated")?.map_or(0.0, |p| p / 100.0);
    let km2 = area / 1e6;
    let (open_km2, veg_km2) = (km2 * (1.0 - veg), km2 * veg);
    let beyond = || {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "ASPRS Table C.1 stops at 2,500 km² of each land cover; past it the standard adds checkpoints by the client's criteria.",
        )
        .at("/area")
    };
    let open_n = table_c1(open_km2).ok_or_else(beyond)?;
    let veg_n = if veg > 0.0 {
        Some(table_c1(veg_km2).ok_or_else(beyond)?)
    } else {
        None
    };
    let total = open_n + veg_n.unwrap_or(0);
    let meters = |name: &str, ctx: &mut Ctx| -> Result<Option<f64>, ToolError> {
        match ctx.quantity(name)? {
            Some(v) if v.base() <= 0.0 => Err(ToolError::invalid(
                &format!("/{name}"),
                "Distances must be positive.",
            )),
            Some(v) => Ok(Some(v.base())),
            None => Ok(None),
        }
    };
    let inset = meters("inset", ctx)?.unwrap_or(10.0);
    let spacing = meters("spacing", ctx)?;
    let h_class = meters("horizontal_class", ctx)?;
    let v_class = meters("vertical_class", ctx)?;
    // The box around the area, for the grids.
    let bbox = rings[0]
        .iter()
        .fold((f64::MAX, f64::MIN, f64::MAX, f64::MIN), |b, p| {
            (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1))
        });
    if let Some(s) = spacing
        && (bbox.1 - bbox.0) * (bbox.3 - bbox.2) / (s * s) > 4_000.0
    {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            "That spacing would place more than about 4,000 points. Use a wider spacing.",
        )
        .at("/spacing"));
    }
    // The middle: the point of a fine grid farthest from every edge.
    let fine = ((bbox.1 - bbox.0).max(bbox.3 - bbox.2)) / 40.0;
    let middle = grid(&rings, bbox, (bbox.0, bbox.2), fine, 0.0)
        .into_iter()
        .map(|p| (edge_distance(&rings, p), p))
        .fold(None, |best: Option<(f64, P)>, c| match best {
            Some(b) if b.0 >= c.0 => Some(b),
            _ => Some(c),
        });
    let mut gcps: Vec<(P, &str)> = corners(&rings, inset)
        .into_iter()
        .map(|p| (p, "corner"))
        .collect();
    let taken = |g: &Vec<(P, &str)>| g.iter().map(|x| x.0).collect::<Vec<P>>();
    if let Some((d, p)) = middle
        && d >= inset
        && !near(&taken(&gcps), p, 2.0 * inset)
    {
        gcps.push((p, "middle"));
    }
    if let Some(s) = spacing {
        let origin = middle.map_or((bbox.0, bbox.2), |m| m.1);
        for p in grid(&rings, bbox, origin, s, inset) {
            if !near(&taken(&gcps), p, s / 2.0) {
                gcps.push((p, "grid"));
            }
        }
    }
    // Checkpoints: the coarsest even grid, half a step off the control, that
    // holds enough points clear of the edges and the control; then every
    // k-th of them, so they spread over the whole area.
    let need = total as usize;
    let mut step = sqrt(area / need as f64);
    let control = taken(&gcps);
    let mut cands: Vec<P> = Vec::new();
    for _ in 0..40 {
        let origin = middle.map_or((bbox.0, bbox.2), |m| {
            (m.1.0 + step / 2.0, m.1.1 + step / 2.0)
        });
        cands = grid(&rings, bbox, origin, step, inset.min(step / 4.0))
            .into_iter()
            .filter(|&p| !near(&control, p, step / 3.0))
            .collect();
        if cands.len() >= need {
            break;
        }
        step *= 0.85;
    }
    let checks: Vec<P> = if cands.len() > need {
        (0..need).map(|k| cands[k * cands.len() / need]).collect()
    } else {
        cands
    };
    let fmt = ctx.options.format;
    if ctx.explaining() {
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Area",
            "the polygon less its holes, on a local plane",
            format!("{} corners", rings[0].len()),
            format!("{} km²", n(km2, 3)),
        );
        if let Some(vn) = veg_n {
            ctx.step(
                "Vegetated checkpoints",
                "ASPRS Table C.1 on the vegetated share",
                format!("{} km² vegetated", n(veg_km2, 3)),
                n(f64::from(vn), 0),
            );
        }
        ctx.step(
            "Open-ground checkpoints",
            "ASPRS Table C.1: 30 up to 500 km², then 5 more per 250 km²",
            format!("{} km² open", n(open_km2, 3)),
            n(f64::from(open_n), 0),
        );
        // The card shows the count bare; the last step reads the same.
        ctx.step(
            "Checkpoints",
            "open ground + vegetation",
            format!("{} + {}", open_n, veg_n.unwrap_or(0)),
            n(f64::from(total), 0),
        );
    }
    let deg = |v: f64| q(v, QT::Angle, "deg").to_json();
    let gcp_rows: Vec<Json> = gcps
        .iter()
        .enumerate()
        .map(|(i, (p, place))| {
            let (la, lo) = plane.inv(p.0, p.1);
            Json::obj([
                ("point", Json::Num((i + 1) as f64)),
                ("lat", deg(la)),
                ("lon", deg(lo)),
                ("place", Json::str(*place)),
            ])
        })
        .collect();
    let check_rows: Vec<Json> = checks
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (la, lo) = plane.inv(p.0, p.1);
            Json::obj([
                ("point", Json::Num((i + 1) as f64)),
                ("lat", deg(la)),
                ("lon", deg(lo)),
            ])
        })
        .collect();
    let show = |x: f64| display::quantity(x, "m", Precision::Decimals(0), fmt);
    let layout = format!(
        "A rule of thumb after Pix4D, not part of the ASPRS standard: a GCP {} inside each corner, one where the area is widest{}, and checkpoints on an even grid between them. Move any point that lands on a roof, slope, vegetation, or hidden spot.",
        show(inset),
        spacing.map_or(String::new(), |s| format!(", more {} apart", show(s)))
    );
    let cmq = |x: f64| q(x, QT::Length, "m");
    let mut o = vec![
        ("checkpoints", Json::Num(f64::from(total))),
        ("checkpoints_open", Json::Num(f64::from(open_n))),
    ];
    if let Some(vn) = veg_n {
        o.push(("checkpoints_vegetated", Json::Num(f64::from(vn))));
    }
    o.push(("gcps", Json::Num(gcps.len() as f64)));
    o.push(("area_size", ctx.out("area_size", q(area, QT::Area, "m2"))));
    if let Some(h) = h_class {
        o.push(("gcp_rmse_h", ctx.out("gcp_rmse_h", cmq(h / 2.0))));
        o.push((
            "gcp_rmse_v",
            ctx.out("gcp_rmse_v", cmq(v_class.map_or(h, |v| v / 2.0))),
        ));
        o.push((
            "checkpoint_rmse_h",
            ctx.out("checkpoint_rmse_h", cmq(h / 2.0)),
        ));
    } else if let Some(v) = v_class {
        o.push(("gcp_rmse_v", ctx.out("gcp_rmse_v", cmq(v / 2.0))));
    }
    if let Some(v) = v_class {
        o.push((
            "checkpoint_rmse_v",
            ctx.out("checkpoint_rmse_v", cmq(v / 2.0)),
        ));
    }
    o.push(("layout", Json::str(layout)));
    o.push(("gcp_points", Json::Arr(gcp_rows)));
    o.push(("checkpoint_points", Json::Arr(check_rows)));
    Ok(Json::obj(o))
}
