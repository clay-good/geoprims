//! Area partition (add-survey-partition-and-adjustment, "Area partition by a
//! cutoff line"): the line that cuts a required area off a closed plane
//! figure, parallel to a side, through a fixed point, or at a bearing.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{atan2, hypot, sqrt};

use crate::staking::brent;
use crate::{common_unit, direction, fmt_station, is_metric, len, len_out, unit};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 12 (area), the partitioning of land",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const MAX_POINTS: usize = 1_000;
/// Brent's method stops when the bracket is narrower than this, in the
/// coordinate unit (the spec asks for 1e-6); it is limited to 200 steps.
const TOL: f64 = 1e-7;

const VERTEX: &[Field] = &[
    len("northing", "Northing", "Like 0").required(),
    len("easting", "Easting", "Like 100").required(),
    len(
        "radius",
        "Curve radius",
        "A curve call to the next corner, like 500 ft; partition takes straight sides only",
    ),
];

const fn area_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Area,
            unit: "ft2",
        },
    )
}

const fn text_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 120 })
}

pub static AREA_PARTITION: ToolDef = ToolDef {
    id: "survey.cogo.area-partition",
    title: "Area partition (cut off a required area)",
    summary: "The cutoff line that takes a required area off a parcel: parallel to a side, through a fixed point, or at a given bearing, with both end points, the line's bearing and length, and both areas.",
    aliases: &[
        "partition of land",
        "cut off acreage",
        "parcel split by area",
    ],
    keywords: &[
        "partition",
        "cutoff",
        "area",
        "parcel",
        "split",
        "acreage",
        "subdivide",
    ],
    inputs: &[
        Field::new(
            "points",
            "Corners",
            "Northing and easting of each corner, in order, like 0, 0",
            Kind::List {
                items: VERTEX,
                min: 3,
                max: MAX_POINTS,
            },
        )
        .required()
        .core(),
        area_field("area", "Area to cut off", "Like 2 ac or 20,000 ft²")
            .required()
            .core(),
        Field::new(
            "method",
            "Cutoff line",
            "Parallel to a side, through a fixed point, or at a bearing",
            Kind::Choice(&["parallel", "through-point", "bearing"]),
        )
        .core(),
        Field::new(
            "side",
            "Side",
            "The side the cut starts from, like 1 for corner 1 to corner 2",
            Kind::Number {
                min: 1.0,
                max: MAX_POINTS as f64,
            },
        )
        .required()
        .core(),
        len(
            "distance",
            "Fixed point along the side",
            "From the side's first corner, like 0 ft for the corner itself",
        ),
        Field::new(
            "bearing",
            "Bearing of the line",
            "For the bearing case, like N 45 00 00 E",
            Kind::Text { max_len: 40 },
        ),
    ],
    outputs: &[
        area_field("cut_area", "Cut area", "The piece taken off").precision(Precision::Decimals(2)),
        area_field("remainder_area", "Remaining area", "The rest of the figure")
            .precision(Precision::Decimals(2)),
        area_field("total_area", "Whole figure", "Both pieces together")
            .precision(Precision::Decimals(2)),
        Field::new(
            "bearing",
            "Line bearing",
            "From the start to the end",
            Kind::Text { max_len: 32 },
        ),
        len_out("length", "Line length", "From the start to the end"),
        len_out("start_northing", "Start northing", ""),
        len_out("start_easting", "Start easting", ""),
        text_out("start_on", "Start on", "Side, station along it, and offset"),
        len_out("end_northing", "End northing", ""),
        len_out("end_easting", "End easting", ""),
        text_out("end_on", "End on", "Side, station along it, and offset"),
        text_out(
            "cut_piece",
            "Cut piece",
            "The corners on the cut side of the line",
        ),
        len_out(
            "offset",
            "Distance from the side",
            "Parallel case: from the chosen side to the line",
        )
        .optional(),
        text_out(
            "check",
            "Trapezoid check",
            "Parallel case ending on the two neighbor sides: the quadratic to check by hand",
        )
        .optional(),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::NoSolution,
        ErrorCode::DegenerateGeometry,
        ErrorCode::UnitMismatch,
    ],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Cut area as a function of one unknown: the line's offset (parallel or bearing case), from a half-plane clip of the figure, or the free end's distance along the boundary (fixed-point case), from the coordinate method; solved by Brent's method to 1e-7 of the unit within 200 steps; pieces by the coordinate method (Ghilani & Wolf 2021, ch. 12)",
    accuracy: "The line is found to 1e-7 of the coordinate unit; the two areas add to the whole within rounding",
    when_to_use: "Use this to split a parcel by area: to take 2 acres off one end of a tract, to cut a lot with a line parallel to the road, or to run a line from a fixed corner. Enter the corners in order, the area to cut, and the case; it returns where the line meets the boundary, its bearing and length, and both areas.",
    limitations: "The figure must be a simple closed shape with straight sides; a curve call is refused, so replace it with chords first. The cut must make exactly two pieces, so a line that would cross the boundary more than twice is refused. Coordinates are plane, and a grid area is not a ground area until the combined factor is applied. A legal partition still follows the deed, the statute, and professional judgment.",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Cut 2,000 ft² off a 100 ft square, parallel to its south side",
        input: r#"{"points":[{"northing":0,"easting":0},{"northing":0,"easting":100},{"northing":100,"easting":100},{"northing":100,"easting":0}],"area":"2000 ft2","method":"parallel","side":1}"#,
        source: "A 100 ft square: the line runs 20 ft north of the south side",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("value", "cut_area")],
    }],
    related: &[
        Related {
            id: "survey.cogo.area-by-coordinates",
            reason: "parent",
        },
        Related {
            id: "survey.land.deed-plot",
            reason: "parent",
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "next",
        },
    ],
    sentence: "Run the cutoff line {bearing} for {length}. It takes off {cut_area} and leaves {remainder_area}.",
    limits: &[("batchRows", 100)],
    run: run_partition,
    ..ToolDef::BLANK
};

type P = (f64, f64);

/// Twice the signed area by the coordinate method, about the first point.
fn twice_area(p: &[P]) -> f64 {
    let (x0, y0) = p[0];
    let mut s = 0.0;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        s += (a.0 - x0) * (b.1 - y0) - (b.0 - x0) * (a.1 - y0);
    }
    s
}

fn orient(a: P, b: P, c: P) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Whether segments ab and cd share any point.
fn segments_meet(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    let on = |p: P, q: P, r: P| {
        r.0 >= p.0.min(q.0) && r.0 <= p.0.max(q.0) && r.1 >= p.1.min(q.1) && r.1 <= p.1.max(q.1)
    };
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    (o1 == 0.0 && on(a, b, c))
        || (o2 == 0.0 && on(a, b, d))
        || (o3 == 0.0 && on(c, d, a))
        || (o4 == 0.0 && on(c, d, b))
}

fn inside(p: &[P], q: P) -> bool {
    let mut c = false;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        if (a.1 > q.1) != (b.1 > q.1) && q.0 < a.0 + (q.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
            c = !c;
        }
    }
    c
}

/// Area of the part of the figure where s(p) ≤ t (Sutherland-Hodgman on one
/// half-plane; exact in area for a simple figure, convex or not).
fn clip_area(p: &[P], s: &[f64], t: f64) -> f64 {
    let mut out: Vec<P> = Vec::with_capacity(p.len() + 4);
    for i in 0..p.len() {
        let j = (i + 1) % p.len();
        let (sa, sb) = (s[i] - t, s[j] - t);
        if sa <= 0.0 {
            out.push(p[i]);
        }
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            let f = sa / (sa - sb);
            out.push((
                p[i].0 + f * (p[j].0 - p[i].0),
                p[i].1 + f * (p[j].1 - p[i].1),
            ));
        }
    }
    if out.len() < 3 {
        0.0
    } else {
        twice_area(&out).abs() / 2.0
    }
}

fn degenerate(at: &str, msg: impl Into<String>) -> ToolError {
    ToolError::new(ErrorCode::DegenerateGeometry, msg.into()).at(at)
}

fn run_partition(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("points")?;
    let mut tagged: Vec<(String, Q)> = vec![];
    let mut corners: Vec<(Q, Q)> = vec![];
    for (i, r) in rows.iter().enumerate() {
        if r.get("radius").is_some_and(|v| !v.is_null()) {
            return Err(degenerate(
                &format!("/points/{i}/radius"),
                format!(
                    "Partition handles straight sides only, and corner {} starts a curve. Replace the curve with chords, or cut the figure without it.",
                    i + 1
                ),
            ));
        }
        let n = ctx
            .row_quantity("points", i, r, "northing")?
            .expect("required");
        let e = ctx
            .row_quantity("points", i, r, "easting")?
            .expect("required");
        tagged.push((format!("/points/{i}/northing"), n));
        tagged.push((format!("/points/{i}/easting"), e));
        corners.push((n, e));
    }
    let distance = ctx.quantity("distance")?;
    if let Some(d) = distance {
        tagged.push(("/distance".to_owned(), d));
    }
    let u = common_unit(
        &tagged
            .iter()
            .map(|(k, q)| (k.as_str(), *q))
            .collect::<Vec<_>>(),
    )?;
    let to_m = Q {
        value: 1.0,
        unit: u,
    }
    .to(unit(QT::Length, "m"));
    let m2 = unit(QT::Area, "m2");
    let want = ctx.req_quantity("area")?.to(m2) / (to_m * to_m);
    let method = ctx.choice("method")?.unwrap_or("parallel");
    let side_no = ctx.number("side")?.expect("required");
    let n = corners.len();
    if side_no.fract() != 0.0 || side_no as usize > n {
        return Err(ToolError::invalid(
            "/side",
            format!(
                "Side is a whole number from 1 to {n}: side k runs from corner k to the next corner."
            ),
        ));
    }
    let k = side_no as usize - 1;
    // Work about the first corner so large grid coordinates keep their digits.
    let (e0, n0) = (corners[0].1.to(u), corners[0].0.to(u));
    let p: Vec<P> = corners
        .iter()
        .map(|(nq, eq)| (eq.to(u) - e0, nq.to(u) - n0))
        .collect();
    for i in 0..n {
        let j = (i + 1) % n;
        if p[i] == p[j] {
            return Err(degenerate(
                &format!("/points/{j}"),
                format!("Corners {} and {} are the same point.", i + 1, j + 1),
            ));
        }
    }
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            if segments_meet(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]) {
                return Err(degenerate(
                    "/points",
                    format!(
                        "Sides {} and {} cross or touch: the corners must trace a simple closed figure, in order.",
                        i + 1,
                        j + 1
                    ),
                ));
            }
        }
    }
    let twice = twice_area(&p);
    let total = twice.abs() / 2.0;
    if total == 0.0 {
        return Err(degenerate("/points", "The corners enclose no area."));
    }
    let fmt = ctx.options.format;
    let area_unit = match u.symbol {
        "ft" => "ft2",
        "ftUS" => "ftUS2",
        _ => "m2",
    };
    let au = unit(QT::Area, area_unit);
    let area_q = |a: f64| Q {
        value: a * to_m * to_m,
        unit: m2,
    };
    let show_area =
        |a: f64| display::quantity(area_q(a).to(au), au.symbol, Precision::Decimals(2), fmt);
    if want <= 0.0 {
        return Err(ToolError::invalid(
            "/area",
            "The area to cut off must be more than zero.",
        ));
    }
    if want >= total {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            format!(
                "The whole figure is {}, so it cannot give up {}.",
                show_area(total),
                show_area(want)
            ),
        )
        .at("/area"));
    }
    let sigma = twice.signum();
    let side_len = hypot(p[(k + 1) % n].0 - p[k].0, p[(k + 1) % n].1 - p[k].1);

    // (edge, point) for both ends, the cut piece, the rest, and the offset.
    let (e1, e2, cut, rest, offset) = match method {
        "parallel" | "bearing" => {
            let d = if method == "parallel" {
                let a = p[k];
                let b = p[(k + 1) % n];
                ((b.0 - a.0) / side_len, (b.1 - a.1) / side_len)
            } else {
                let Some(raw) = ctx.text("bearing")? else {
                    return Err(ToolError::invalid(
                        "/bearing",
                        "The bearing case needs the line's bearing, like N 45 00 00 E.",
                    ));
                };
                let az = direction::parse(&raw)
                    .map_err(|m| ToolError::invalid("/bearing", m))?
                    .to_radians();
                (libm::sin(az), libm::cos(az))
            };
            // Normal to the line: inward from the side for the parallel case;
            // for a bearing, pointed so the sweep starts at the chosen side.
            let mut nrm = (-d.1 * sigma, d.0 * sigma);
            let origin = p[k];
            let sv = |nrm: P| -> Vec<f64> {
                p.iter()
                    .map(|q| nrm.0 * (q.0 - origin.0) + nrm.1 * (q.1 - origin.1))
                    .collect()
            };
            let mut s = sv(nrm);
            if method == "bearing" {
                let lo = s.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let mid = (s[k] + s[(k + 1) % n]) / 2.0;
                if mid - lo > hi - mid {
                    nrm = (-nrm.0, -nrm.1);
                    s = sv(nrm);
                }
            }
            let lo = s.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let t = brent(|t| clip_area(&p, &s, t) - want, lo, hi, TOL).ok_or_else(|| {
                ToolError::new(
                    ErrorCode::NoSolution,
                    "The search for the line did not settle.",
                )
                .at("/area")
            })?;
            let crossings: Vec<(usize, P)> = (0..n)
                .filter_map(|i| {
                    let j = (i + 1) % n;
                    let (sa, sb) = (s[i] - t, s[j] - t);
                    ((sa < 0.0) != (sb < 0.0)).then(|| {
                        let f = sa / (sa - sb);
                        (
                            i,
                            (
                                p[i].0 + f * (p[j].0 - p[i].0),
                                p[i].1 + f * (p[j].1 - p[i].1),
                            ),
                        )
                    })
                })
                .collect();
            if crossings.len() != 2 {
                return Err(degenerate(
                    "/side",
                    format!(
                        "The cutoff line crosses the boundary {} times, so it would make more than two pieces. Split the figure first, or start from another side.",
                        crossings.len()
                    ),
                ));
            }
            let ((i, a), (j, b)) = (crossings[0], crossings[1]);
            let first: Vec<P> = std::iter::once(a)
                .chain((i + 1..=j).map(|v| p[v]))
                .chain(std::iter::once(b))
                .collect();
            let second: Vec<P> = std::iter::once(b)
                .chain((j + 1..n).chain(0..=i).map(|v| p[v]))
                .chain(std::iter::once(a))
                .collect();
            let first_is_cut = s[(i + 1) % n] - t < 0.0;
            let (cut, rest) = if first_is_cut {
                ((i, a, j, b, first), second)
            } else {
                ((j, b, i, a, second), first)
            };
            let (ci, ca, cj, cb, cut_poly) = cut;
            let off = (method == "parallel").then_some(t);
            ((ci, ca), (cj, cb), cut_poly, rest, off)
        }
        _ => {
            let dist = distance.map_or(0.0, |d| d.to(u));
            if !(0.0..=side_len).contains(&dist) {
                return Err(ToolError::invalid(
                    "/distance",
                    format!(
                        "The fixed point must lie on side {}: from 0 to {}.",
                        k + 1,
                        display::number(side_len, Precision::Decimals(3), fmt)
                    ),
                ));
            }
            let b = p[(k + 1) % n];
            let start = (
                p[k].0 + (b.0 - p[k].0) * dist / side_len,
                p[k].1 + (b.1 - p[k].1) * dist / side_len,
            );
            // The boundary from the fixed point forward: the point, then every
            // corner in order, ending with corner k; edge m of the walk lies on
            // side (k + m) mod n.
            let walk: Vec<P> = std::iter::once(start)
                .chain((1..=n).map(|m| p[(k + m) % n]))
                .collect();
            let mut cum = vec![0.0];
            for m in 0..=n {
                let (a, b) = (walk[m], walk[(m + 1) % (n + 1)]);
                cum.push(cum[m] + hypot(b.0 - a.0, b.1 - a.1));
            }
            let per = cum[n + 1];
            let at = |len: f64| -> (usize, P) {
                let m = (0..=n).find(|&m| len <= cum[m + 1]).unwrap_or(n);
                let (a, b) = (walk[m], walk[(m + 1) % (n + 1)]);
                let seg = cum[m + 1] - cum[m];
                let f = if seg > 0.0 { (len - cum[m]) / seg } else { 0.0 };
                (m, (a.0 + f * (b.0 - a.0), a.1 + f * (b.1 - a.1)))
            };
            let piece = |len: f64| -> f64 {
                let (m, q) = at(len);
                let mut poly: Vec<P> = walk[..=m].to_vec();
                poly.push(q);
                if poly.len() < 3 {
                    0.0
                } else {
                    sigma * twice_area(&poly) / 2.0
                }
            };
            let len = brent(|l| piece(l) - want, 0.0, per, TOL).ok_or_else(|| {
                ToolError::new(
                    ErrorCode::NoSolution,
                    "The search for the line did not settle.",
                )
                .at("/area")
            })?;
            let (m, q) = at(len);
            let mut cut_poly: Vec<P> = walk[..=m].to_vec();
            cut_poly.push(q);
            let rest: Vec<P> = std::iter::once(q)
                .chain(walk[m + 1..].iter().copied())
                .chain(std::iter::once(start))
                .collect();
            // The chord must stay inside the figure.
            let mid = ((start.0 + q.0) / 2.0, (start.1 + q.1) / 2.0);
            let crosses = (0..n).any(|i| {
                let (a, b) = (p[i], p[(i + 1) % n]);
                // The sides the chord ends on touch it by construction.
                let l = hypot(b.0 - a.0, b.1 - a.1);
                let on = |r: P| {
                    let f = ((r.0 - a.0) * (b.0 - a.0) + (r.1 - a.1) * (b.1 - a.1)) / (l * l);
                    orient(a, b, r).abs() <= 1e-9 * l * l.max(1.0)
                        && (-1e-12..=1.0 + 1e-12).contains(&f)
                };
                if on(start) || on(q) {
                    return false;
                }
                let (o1, o2) = (orient(start, q, a), orient(start, q, b));
                let (o3, o4) = (orient(a, b, start), orient(a, b, q));
                o1 * o2 < 0.0 && o3 * o4 < 0.0
            });
            if crosses || !inside(&p, mid) {
                return Err(degenerate(
                    "/side",
                    "The line from the fixed point to the end that gives this area leaves the figure. Pick a fixed point that sees the far side, or split the figure first.",
                ));
            }
            ((k, start), ((k + m) % n, q), cut_poly, rest, None)
        }
    };

    let cut_area = twice_area(&cut).abs() / 2.0;
    let rest_area = twice_area(&rest).abs() / 2.0;
    let (dx, dy) = (e2.1.0 - e1.1.0, e2.1.1 - e1.1.1);
    let line_len = hypot(dx, dy);
    // Snap to a nanodegree so a due-east line does not print as S 90° E.
    let az = libm::round(atan2(dx, dy).to_degrees() * 1e9) / 1e9;
    let metric = is_metric(u);
    let n3 = |v: f64| display::number(v, Precision::Decimals(3), fmt);
    let on_side = |(i, q): (usize, P)| {
        let a = p[i];
        let b = p[(i + 1) % n];
        let l = hypot(b.0 - a.0, b.1 - a.1);
        let station = ((q.0 - a.0) * (b.0 - a.0) + (q.1 - a.1) * (b.1 - a.1)) / l;
        let mut off = orient(a, b, q) / l;
        if off.abs() < 5e-4 {
            off = 0.0;
        }
        format!(
            "side {}, station {}, offset {}",
            i + 1,
            fmt_station(station, metric),
            n3(off)
        )
    };
    // Which corners lie on the cut side: every point of the cut piece but its
    // two line ends.
    let cut_corners: Vec<String> = cut[1..cut.len() - 1]
        .iter()
        .filter_map(|q| p.iter().position(|v| v == q).map(|c| (c + 1).to_string()))
        .collect();
    let cut_piece = format!("corners {}", cut_corners.join(", "));
    // The trapezoid case: a parallel line ending on the two neighbor sides,
    // A(d) = L·d − d²(cot α + cot β)/2, solvable by hand.
    let mut check = None;
    if let Some(t) = offset {
        let (prev, next) = ((k + n - 1) % n, (k + 1) % n);
        let ends = [e1.0, e2.0];
        if ends.contains(&prev) && ends.contains(&next) && prev != next {
            let cot = |v: P, a: P, b: P| {
                let (u1, u2) = ((a.0 - v.0, a.1 - v.1), (b.0 - v.0, b.1 - v.1));
                (u1.0 * u2.0 + u1.1 * u2.1) / (u1.0 * u2.1 - u1.1 * u2.0).abs()
            };
            let c = cot(p[k], p[next], p[prev]) + cot(p[next], p[k], p[(k + 2) % n]);
            let disc = side_len * side_len - 2.0 * c * want;
            let dq = if c.abs() < 1e-12 {
                Some(want / side_len)
            } else if disc >= 0.0 {
                Some((side_len - sqrt(disc)) / c)
            } else {
                None
            };
            if let Some(dq) = dq.filter(|dq| (dq - t).abs() <= 1e-6 * side_len.max(1.0)) {
                let s6 = |v: f64| display::number(v, Precision::Significant(6), fmt);
                check = Some(format!(
                    "{} d² − {} d + {} = 0, so d = {} {}",
                    s6(c / 2.0),
                    s6(side_len),
                    s6(want),
                    n3(dq),
                    u.symbol
                ));
            }
        }
    }
    if ctx.explaining() {
        ctx.step(
            "Whole figure",
            "2A = Σ (xᵢ − x₀)(yᵢ₊₁ − y₀) − (xᵢ₊₁ − x₀)(yᵢ − y₀)",
            format!("{n} corners by the coordinate method"),
            show_area(total),
        );
        let (what, how) = match method {
            "parallel" => (
                "Line parallel to the side",
                "find d where the area between side and line = A",
            ),
            "bearing" => (
                "Line at the bearing",
                "slide the line until the area on the side's side = A",
            ),
            _ => (
                "Line through the fixed point",
                "move the far end along the boundary until the area = A",
            ),
        };
        ctx.step(
            what,
            how,
            format!(
                "A = {}, Brent's method to 1e-7 {}",
                show_area(want),
                u.symbol
            ),
            format!(
                "{} for {} {}",
                direction::bearing(az, 0),
                n3(line_len),
                u.symbol
            ),
        );
        if let Some(c) = &check {
            ctx.step(
                "Trapezoid check",
                "(cot α + cot β)/2 · d² − L·d + A = 0",
                "the line ends on the two neighbor sides".to_owned(),
                c.clone(),
            );
        }
        ctx.step(
            "Pieces",
            "cut + rest = whole",
            format!("{} − {} left", show_area(total), show_area(rest_area)),
            show_area(cut_area),
        );
    }
    let len_q = |v: f64| Q { value: v, unit: u };
    let mut out = vec![
        ("cut_area", ctx.emit("cut_area", area_q(cut_area), au)),
        (
            "remainder_area",
            ctx.emit("remainder_area", area_q(rest_area), au),
        ),
        ("total_area", ctx.emit("total_area", area_q(total), au)),
        ("bearing", Json::str(direction::bearing(az, 0))),
        ("length", ctx.emit("length", len_q(line_len), u)),
        (
            "start_northing",
            ctx.emit("start_northing", len_q(e1.1.1 + n0), u),
        ),
        (
            "start_easting",
            ctx.emit("start_easting", len_q(e1.1.0 + e0), u),
        ),
        ("start_on", Json::str(on_side(e1))),
        (
            "end_northing",
            ctx.emit("end_northing", len_q(e2.1.1 + n0), u),
        ),
        (
            "end_easting",
            ctx.emit("end_easting", len_q(e2.1.0 + e0), u),
        ),
        ("end_on", Json::str(on_side(e2))),
        ("cut_piece", Json::str(cut_piece)),
    ];
    if let Some(t) = offset {
        out.push(("offset", ctx.emit("offset", len_q(t), u)));
    }
    if let Some(c) = check {
        out.push(("check", Json::str(c)));
    }
    Ok(Json::obj(out))
}
