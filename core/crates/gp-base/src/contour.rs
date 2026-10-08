//! Contour lines on a regular grid by marching squares, shared by the raster
//! contour tool and the isogonic overlay: each grid square's crossings of a
//! level as directed pieces, with the higher side on the left, then the
//! pieces joined into lines and loops.

use std::collections::HashMap;

/// A position on the grid's plane: x east, y south, from the north-west
/// corner, in the grid's own unit.
pub type P = (f64, f64);

fn key(p: P) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

/// The directed pieces of one level: each from one edge crossing to another,
/// with higher ground on the left.
pub fn pieces(z: &[Vec<f64>], cell: f64, level: f64) -> Vec<(P, P)> {
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
pub fn join(pieces: &[(P, P)]) -> Vec<(Vec<P>, bool)> {
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
