//! Contours (raster/terrain-analysis, "Slope, aspect, hillshade, and
//! contours"): lines of equal elevation at an interval, as vector lines, by
//! marching squares over a grid of elevations.
//!
//! Each grid square is looked at on its own. A contour crosses an edge of the
//! square where one corner is above the level and the other at or below it, at
//! the point found by linear interpolation along that edge, and the crossings
//! in a square are joined in pairs. When all four edges are crossed (a
//! saddle), the mean of the four corners decides whether the high corners
//! connect through the middle. Every line is oriented with higher ground on
//! its left, so crossings computed once per edge join the squares' pieces
//! into lines without searching.

use std::collections::HashMap;

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};

const LORENSEN: Reference = Reference {
    title: "Marching cubes: A high resolution 3D surface construction algorithm",
    issuer: "Lorensen, W. E., and Cline, H. E., ACM SIGGRAPH Computer Graphics",
    year: 1987,
    edition: "Volume 21, issue 4, pages 163-169",
    locator: "The cell-by-cell case table with linear interpolation along edges, of which marching squares is the two-dimensional case",
    url: "https://doi.org/10.1145/37402.37422",
};

/// Grid size limits, per side.
const MAX_SIDE: usize = 300;
/// Levels allowed in one run.
const MAX_LEVELS: usize = 500;
/// Contour vertices allowed in the output.
const MAX_VERTICES: usize = 100_000;

const ROW: &[Field] = &[Field::new(
    "row",
    "Elevations",
    "One row of elevations west to east, like 101.2, 100.6, 100.2",
    Kind::Text { max_len: 4_000 },
)];

const fn meters(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

const LINE_ROW: &[Field] = &[
    Field::new(
        "line",
        "Line",
        "Its number, from 1, like 3",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    meters("level", "Elevation", "The contour's elevation, like 110 m")
        .precision(Precision::Decimals(3)),
    meters(
        "east",
        "Distance east",
        "From the north-west grid point, like 45.5 m",
    )
    .precision(Precision::Decimals(3)),
    meters(
        "south",
        "Distance south",
        "From the north-west grid point, like 12.25 m",
    )
    .precision(Precision::Decimals(3)),
];
const LEVEL_ROW: &[Field] = &[
    meters("level", "Elevation", "The contour's elevation, like 110 m")
        .precision(Precision::Decimals(3)),
    Field::new(
        "lines",
        "Lines",
        "Separate lines at this elevation, like 2",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "closed",
        "Closed loops",
        "Lines that close on themselves, like 1",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    meters(
        "length",
        "Length",
        "All the lines at this elevation, like 412.6 m",
    )
    .precision(Precision::Decimals(3)),
];

pub static CONTOURS: ToolDef = ToolDef {
    id: "raster.terrain.contours",
    stability: gp_base::tool::Stability::Stable,
    title: "Contour lines from a grid of elevations",
    summary: "Lines of equal elevation at a chosen interval from a grid of elevations, as vector lines with their lengths, by marching squares.",
    aliases: &[
        "contour lines",
        "contours from dem",
        "isolines",
        "topo lines",
        "generate contours",
        "marching squares",
    ],
    keywords: &[
        "contour",
        "isoline",
        "elevation",
        "interval",
        "dem",
        "topographic",
        "terrain",
    ],
    inputs: &[
        Field::new(
            "elevations",
            "Elevation grid",
            "Rows of elevations in meters, north row first, west to east, like 101.2, 100.6, 100.2",
            Kind::List {
                items: ROW,
                min: 2,
                max: MAX_SIDE,
            },
        )
        .required()
        .core(),
        meters(
            "interval",
            "Contour interval",
            "The rise between lines, like 5 m",
        )
        .required()
        .core(),
        meters(
            "cell_size",
            "Cell size",
            "The ground distance between grid points, like 30 m",
        )
        .required()
        .core(),
        meters(
            "base",
            "Base elevation",
            "Contours fall on this plus whole intervals, like 0 m (the default)",
        ),
        Field::new(
            "no_data",
            "No-data value",
            "A value that marks a missing elevation, like -9999",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "line_count",
            "Lines",
            "Separate contour lines at every level, like 7",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "level_count",
            "Levels",
            "Elevations with at least one line, like 4",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        meters(
            "length",
            "Total length",
            "All the lines together, like 1,250 m",
        )
        .precision(Precision::Decimals(3)),
        meters("lowest", "Lowest elevation", "In the grid, like 96.1 m")
            .precision(Precision::Decimals(3)),
        meters("highest", "Highest elevation", "In the grid, like 131.4 m")
            .precision(Precision::Decimals(3)),
        Field::new(
            "levels",
            "Each level",
            "Lines, closed loops, and length per elevation",
            Kind::List {
                items: LEVEL_ROW,
                min: 0,
                max: MAX_LEVELS,
            },
        ),
        Field::new(
            "contours",
            "Contour vertices",
            "Each line's points in order, higher ground on the left",
            Kind::List {
                items: LINE_ROW,
                min: 0,
                max: MAX_VERTICES,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    warnings: &[],
    model: "Marching squares (the two-dimensional case of Lorensen and Cline 1987): in each grid square, a level crosses an edge whose corners lie on either side of it (a corner exactly at the level counts as below, as in contourpy), at the linearly interpolated point; the crossings are paired, and a square crossed on all four edges joins its high corners through the middle when the mean of its corners is above the level. Levels are the base plus whole intervals within the grid's range. Squares with a no-data corner are skipped",
    accuracy: "Exact for the surface made of linear pieces along the grid lines; a real hillside between grid points is only as well known as the grid",
    when_to_use: "Use this to turn a small grid of elevations into contour lines: a survey of spot heights on a regular grid, a DEM clip exported as numbers, or a site model, when you want lines to draw, measure, or hand to CAD rather than a picture. It gives each line's points in order with higher ground on the left, the lines and closed loops per elevation, and their lengths, so a closed loop can be read as a hill or a hollow at a glance.",
    limitations: "The grid must be regular and square-celled, with elevations in meters and positions given as distances east and south of the north-west grid point rather than as coordinates. Between grid points the ground is assumed to change linearly along each grid line, so a contour cannot show a feature smaller than the grid spacing, and where a square is crossed on all four sides the choice of which way to join is a convention (the mean of the corners, as contourpy and Matplotlib use) rather than something the grid knows. Squares that touch a no-data value are left out, so lines stop at gaps. At most 300 by 300 points, 500 levels, and 100,000 contour vertices.",
    references: &[LORENSEN],
    examples: &[Example {
        id: "primary",
        title: "5 m contours on a small hill",
        input: r#"{"elevations":[{"row":"100, 102, 104, 102, 100"},{"row":"102, 106, 110, 106, 102"},{"row":"104, 110, 118, 110, 104"},{"row":"102, 106, 110, 106, 102"},{"row":"100, 102, 104, 102, 100"}],"interval":"5 m","cell_size":"10 m"}"#,
        source: "contourpy 1.3.0 (the contouring engine behind Matplotlib) gives the same three closed loops, at 105, 110, and 115 m, with the same lengths to 1e-9",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.terrain.slope-aspect",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.ruggedness",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.profile-grades",
            reason: "next",
        },
    ],
    sentence: "{line_count} contour {plural line_count \"line\" \"lines\"} at {level_count} {plural level_count \"level\" \"levels\"}, {length} in all.",
    limits: &[("batchRows", MAX_SIDE as u64)],
    run: run_contours,
    ..ToolDef::BLANK
};

type P = (f64, f64);

fn key(p: P) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

fn read_grid(ctx: &mut Ctx, no_data: Option<f64>) -> Result<Vec<Vec<f64>>, ToolError> {
    let rows = ctx.rows("elevations")?;
    let mut grid = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let at = format!("/elevations/{i}/row");
        let text = r.get("row").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::invalid(
                &at,
                "Each row is elevations west to east, like 101.2, 100.6, 100.2.",
            )
        })?;
        let mut row = Vec::new();
        for p in text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
        {
            let v: f64 = p
                .parse()
                .ok()
                .filter(|v: &f64| v.is_finite())
                .ok_or_else(|| ToolError::invalid(&at, format!("{p} is not a number.")))?;
            row.push(if Some(v) == no_data { f64::NAN } else { v });
        }
        if row.len() < 2 || row.len() > MAX_SIDE {
            return Err(ToolError::invalid(
                &at,
                format!(
                    "Row {} has {} elevations; each row needs 2 to {MAX_SIDE}.",
                    i + 1,
                    row.len()
                ),
            ));
        }
        if let Some(first) = grid.first().map(Vec::len)
            && row.len() != first
        {
            return Err(ToolError::invalid(
                &at,
                format!(
                    "Row {} has {} elevations and row 1 has {first}; every row needs the same number.",
                    i + 1,
                    row.len()
                ),
            ));
        }
        grid.push(row);
    }
    Ok(grid)
}

/// The directed pieces of one level: each from one edge crossing to another,
/// with higher ground on the left.
fn pieces(z: &[Vec<f64>], cell: f64, level: f64) -> Vec<(P, P)> {
    let (rows, cols) = (z.len(), z[0].len());
    let pos = |i: usize, j: usize| (j as f64 * cell, i as f64 * cell);
    // The crossing on the edge between two grid points, always computed from
    // the first (north or west) end, so the squares on either side get the
    // same bits.
    let cross = |a: (usize, usize), b: (usize, usize)| -> P {
        let (za, zb) = (z[a.0][a.1], z[b.0][b.1]);
        let t = (level - za) / (zb - za);
        let (pa, pb) = (pos(a.0, a.1), pos(b.0, b.1));
        if t <= 0.0 {
            pa
        } else if t >= 1.0 {
            pb
        } else {
            (pa.0 + t * (pb.0 - pa.0), pa.1 + t * (pb.1 - pa.1))
        }
    };
    let mut out = Vec::new();
    for i in 0..rows - 1 {
        for j in 0..cols - 1 {
            // Corners clockwise from the north-west: nw, ne, se, sw.
            let c = [(i, j), (i, j + 1), (i + 1, j + 1), (i + 1, j)];
            let v = c.map(|(a, b)| z[a][b]);
            if v.iter().any(|x| x.is_nan()) {
                continue;
            }
            let up = v.map(|x| x > level);
            // Edges: north (nw-ne), east (ne-se), south (sw-se), west (nw-sw),
            // each with its canonical ends and the corners it joins.
            let edges = [
                (c[0], c[1], 0, 1),
                (c[1], c[2], 1, 2),
                (c[3], c[2], 3, 2),
                (c[0], c[3], 0, 3),
            ];
            let crossed: Vec<usize> = (0..4)
                .filter(|&e| up[edges[e].2] != up[edges[e].3])
                .collect();
            let pairs: Vec<(usize, usize, Option<usize>)> = match crossed.len() {
                2 => {
                    let (e0, e1) = (crossed[0], crossed[1]);
                    // Adjacent edges cut off the corner they share.
                    let shared = [edges[e0].2, edges[e0].3]
                        .into_iter()
                        .find(|k| *k == edges[e1].2 || *k == edges[e1].3);
                    vec![(e0, e1, shared)]
                }
                4 => {
                    // A saddle: cut off the two corners on the side the middle
                    // is not on.
                    let middle_up = v.iter().sum::<f64>() / 4.0 > level;
                    let corner_edges = [(0, 3), (0, 1), (1, 2), (2, 3)];
                    (0..4)
                        .filter(|&k| up[k] != middle_up)
                        .map(|k| (corner_edges[k].0, corner_edges[k].1, Some(k)))
                        .collect()
                }
                _ => Vec::new(),
            };
            for (e0, e1, cut) in pairs {
                let p = cross(edges[e0].0, edges[e0].1);
                let q = cross(edges[e1].0, edges[e1].1);
                if p == q {
                    continue;
                }
                // Higher ground on the left of p → q, in east-south
                // coordinates (a left turn there is clockwise on the ground,
                // so "left" here is the sign flipped).
                let left_of = |k: usize| {
                    let (x, y) = pos(c[k].0, c[k].1);
                    let cr = (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0);
                    -cr
                };
                let side = match cut {
                    Some(k) => {
                        if up[k] {
                            left_of(k)
                        } else {
                            -left_of(k)
                        }
                    }
                    // Opposite edges: any corner off the piece tells the side.
                    None => (0..4)
                        .map(|k| if up[k] { left_of(k) } else { -left_of(k) })
                        .find(|s| *s != 0.0)
                        .unwrap_or(1.0),
                };
                out.push(if side > 0.0 { (p, q) } else { (q, p) });
            }
        }
    }
    out
}

/// The pieces joined end to start into lines: (points, closed).
fn join(pieces: &[(P, P)]) -> Vec<(Vec<P>, bool)> {
    let mut from: HashMap<(u64, u64), Vec<usize>> = HashMap::new();
    let mut into: HashMap<(u64, u64), usize> = HashMap::new();
    for (k, &(a, b)) in pieces.iter().enumerate() {
        from.entry(key(a)).or_default().push(k);
        *into.entry(key(b)).or_default() += 1;
    }
    let mut used = vec![false; pieces.len()];
    let mut lines = Vec::new();
    // Open lines start where nothing leads in; loops take whatever is left.
    let starts: Vec<usize> = (0..pieces.len())
        .filter(|&k| !into.contains_key(&key(pieces[k].0)))
        .chain(0..pieces.len())
        .collect();
    for s in starts {
        if used[s] {
            continue;
        }
        used[s] = true;
        let mut pts = vec![pieces[s].0, pieces[s].1];
        loop {
            let end = *pts.last().expect("nonempty");
            let next = from
                .get(&key(end))
                .and_then(|v| v.iter().copied().find(|&k| !used[k]));
            let Some(k) = next else { break };
            used[k] = true;
            pts.push(pieces[k].1);
        }
        lines.extend(untangle(pts));
    }
    lines
}

/// A line that passes through one point twice (two contours touching at a
/// grid point exactly on the level) split into the loop between the visits
/// and the rest, so a closed contour is never folded into another line.
fn untangle(mut pts: Vec<P>) -> Vec<(Vec<P>, bool)> {
    let mut out = Vec::new();
    'again: loop {
        let mut seen: HashMap<(u64, u64), usize> = HashMap::new();
        let last = pts.len() - 1;
        for (j, p) in pts.iter().enumerate() {
            if let Some(&i) = seen.get(&key(*p))
                && !(i == 0 && j == last)
            {
                out.push((pts[i..=j].to_vec(), true));
                pts.drain(i + 1..=j);
                continue 'again;
            }
            seen.insert(key(*p), j);
        }
        let closed = pts.len() > 2 && key(pts[0]) == key(pts[last]);
        out.push((pts, closed));
        return out;
    }
}

fn run_contours(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let no_data = ctx.number("no_data")?;
    let z = read_grid(ctx, no_data)?;
    let interval = ctx.quantity("interval")?.expect("required").to(m);
    let cell = ctx.quantity("cell_size")?.expect("required").to(m);
    let base = ctx.quantity("base")?.map_or(0.0, |v| v.to(m));
    if interval.is_nan() || interval <= 0.0 {
        return Err(ToolError::invalid(
            "/interval",
            "The contour interval must be more than zero.",
        ));
    }
    if cell.is_nan() || cell <= 0.0 {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        ));
    }
    let known: Vec<f64> = z
        .iter()
        .flatten()
        .copied()
        .filter(|v| !v.is_nan())
        .collect();
    if known.is_empty() {
        return Err(ToolError::invalid(
            "/elevations",
            "Every elevation is the no-data value.",
        ));
    }
    let lo = known.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = known.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (k0, k1) = (
        libm::ceil((lo - base) / interval),
        libm::floor((hi - base) / interval),
    );
    if k1 - k0 + 1.0 > MAX_LEVELS as f64 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "That interval gives {} levels between {lo} and {hi} m; at most {MAX_LEVELS}. Use a larger interval.",
                k1 - k0 + 1.0
            ),
        )
        .at("/interval"));
    }
    let q = |v: f64| Q { value: v, unit: m }.to_json();
    let (mut vertices, mut rows, mut per_level) = (0usize, Vec::new(), Vec::new());
    let (mut line_count, mut total) = (0usize, 0.0);
    let mut k = k0;
    while k <= k1 {
        let level = base + k * interval;
        k += 1.0;
        let lines = join(&pieces(&z, cell, level));
        if lines.is_empty() {
            continue;
        }
        let (mut len_level, mut closed) = (0.0, 0usize);
        for (pts, is_closed) in &lines {
            line_count += 1;
            closed += usize::from(*is_closed);
            vertices += pts.len();
            if vertices > MAX_VERTICES {
                return Err(ToolError::new(
                    ErrorCode::LimitExceeded,
                    format!("Over {MAX_VERTICES} contour vertices. Use a larger interval or a smaller grid."),
                )
                .at("/interval"));
            }
            for w in pts.windows(2) {
                len_level += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
            }
            for p in pts {
                rows.push(Json::obj([
                    ("line", Json::Num(line_count as f64)),
                    ("level", q(level)),
                    ("east", q(p.0)),
                    ("south", q(p.1)),
                ]));
            }
        }
        total += len_level;
        per_level.push(Json::obj([
            ("level", q(level)),
            ("lines", Json::Num(lines.len() as f64)),
            ("closed", Json::Num(closed as f64)),
            ("length", q(len_level)),
        ]));
    }
    Ok(Json::obj([
        ("line_count", Json::Num(line_count as f64)),
        ("level_count", Json::Num(per_level.len() as f64)),
        (
            "length",
            ctx.out(
                "length",
                Q {
                    value: total,
                    unit: m,
                },
            ),
        ),
        ("lowest", ctx.out("lowest", Q { value: lo, unit: m })),
        ("highest", ctx.out("highest", Q { value: hi, unit: m })),
        ("levels", Json::Arr(per_level)),
        ("contours", Json::Arr(rows)),
    ]))
}
