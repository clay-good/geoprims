//! The outline of a set of H3 cells (H3 C cellsToMultiPolygon) on h3o's core
//! API, since h3o's own geo feature breaks reproducible builds.
//!
//! Every cell contributes the segments of its boundary. A segment between two
//! cells of the set is walked once from each side, so it appears twice and is
//! dropped; what is left is the outline, and following those segments end to
//! end traces it. Cells all wind the same way, so an outer ring comes out
//! wound like its cells and a hole wound against them, which is what tells the
//! two apart.
//!
//! The boundary is taken rather than the cell's five or six vertexes because a
//! cell near one of the twelve pentagons carries extra points where its edge
//! crosses an icosahedron face: at resolution 5 such a cell has seven boundary
//! points and the pentagon itself ten. Two cells do not agree to the last bit
//! on a point they share, differing by about 1e-12 degrees, so points are
//! snapped together within a tolerance rather than compared exactly.

use std::collections::HashMap;

use h3o::CellIndex;

/// A closed ring of (lat, lon) degrees; the first point repeats as the last.
pub type Ring = Vec<(f64, f64)>;

/// An outer ring followed by its holes, as GeoJSON orders a polygon.
pub type Poly = Vec<Ring>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The cells are not all at one resolution.
    MixedResolution,
}

/// How close two points must be to be the same point, in degrees: far below
/// the spacing of distinct H3 boundary points, far above the 1e-12 by which
/// two cells disagree about a point they share.
const SNAP: f64 = 1e-9;
const BUCKET: f64 = 1e-8;

/// Gathers points that are the same point to within [`SNAP`], giving each an
/// index, so segments can be matched by whole numbers instead of by floats.
#[derive(Default)]
struct Snap {
    pts: Vec<(f64, f64)>,
    bins: HashMap<(i64, i64), Vec<usize>>,
}

impl Snap {
    fn at(&mut self, p: (f64, f64)) -> usize {
        let key = ((p.0 / BUCKET) as i64, (p.1 / BUCKET) as i64);
        // A point near a bucket edge falls either side, so look around it.
        for di in -1..=1 {
            for dj in -1..=1 {
                for &i in self
                    .bins
                    .get(&(key.0 + di, key.1 + dj))
                    .into_iter()
                    .flatten()
                {
                    let q = self.pts[i];
                    if (q.0 - p.0).abs() < SNAP && (q.1 - p.1).abs() < SNAP {
                        return i;
                    }
                }
            }
        }
        self.pts.push(p);
        self.bins.entry(key).or_default().push(self.pts.len() - 1);
        self.pts.len() - 1
    }
}

/// Traces the outline of `cells`, as polygons with their holes.
pub fn outline(cells: &[CellIndex]) -> Result<Vec<Poly>, Error> {
    let mut set: Vec<CellIndex> = cells.to_vec();
    set.sort_unstable();
    set.dedup();
    let Some(first) = set.first() else {
        return Ok(Vec::new());
    };
    if set.iter().any(|c| c.resolution() != first.resolution()) {
        return Err(Error::MixedResolution);
    }
    let mut snap = Snap::default();
    let mut seen: HashMap<(usize, usize), u32> = HashMap::new();
    let mut directed: Vec<(usize, usize)> = Vec::new();
    for c in &set {
        let ids: Vec<usize> = c
            .boundary()
            .iter()
            .map(|ll| snap.at((ll.lat(), ll.lng())))
            .collect();
        for k in 0..ids.len() {
            let (a, b) = (ids[k], ids[(k + 1) % ids.len()]);
            if a == b {
                continue;
            }
            *seen.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            directed.push((a, b));
        }
    }
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, b) in directed {
        if seen[&(a.min(b), a.max(b))] == 1 {
            out.entry(a).or_default().push(b);
        }
    }
    // Follow the kept segments into closed loops. Where the outline pinches to
    // a point there is more than one way out; any of them closes a loop, so
    // take them in turn.
    let mut rings: Vec<Ring> = Vec::new();
    let mut starts: Vec<usize> = out.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        while out.get(&start).is_some_and(|v| !v.is_empty()) {
            let mut ring: Vec<usize> = Vec::new();
            let mut at = start;
            while let Some(next) = out.get_mut(&at).and_then(Vec::pop) {
                ring.push(next);
                at = next;
                if at == start {
                    break;
                }
            }
            if ring.len() < 3 {
                continue;
            }
            let mut pts: Ring = ring.iter().map(|&i| snap.pts[i]).collect();
            pts.insert(0, *pts.last().expect("a ring has points"));
            rings.push(pts);
        }
    }
    Ok(assemble(rings))
}

/// Twice the signed area of a ring, in degrees squared, with longitudes
/// unwrapped so a ring across the antimeridian keeps its sign.
fn signed_area(ring: &Ring) -> f64 {
    let mut lon = Vec::with_capacity(ring.len());
    let mut prev = ring[0].1;
    for (_, l) in ring {
        prev += (((l - prev + 180.0) % 360.0) + 360.0) % 360.0 - 180.0;
        lon.push(prev);
    }
    let mut sum = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        sum += lon[i] * ring[j].0 - lon[j] * ring[i].0;
    }
    sum
}

/// Whether `p` lies inside `ring`, by ray casting with longitudes taken
/// relative to the point so a ring across the antimeridian still reads right.
fn contains(ring: &Ring, p: (f64, f64)) -> bool {
    let wrap = |l: f64| ((((l - p.1 + 180.0) % 360.0) + 360.0) % 360.0) - 180.0;
    let mut inside = false;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        let (yi, xi) = (ring[i].0, wrap(ring[i].1));
        let (yj, xj) = (ring[j].0, wrap(ring[j].1));
        if (yi > p.0) != (yj > p.0) && 0.0 < (xj - xi) * (p.0 - yi) / (yj - yi) + xi {
            inside = !inside;
        }
    }
    inside
}

/// Sorts rings into polygons: a ring wound against the cells is a hole, and
/// belongs to the smallest ring that contains it.
fn assemble(rings: Vec<Ring>) -> Vec<Poly> {
    let signs: Vec<f64> = rings.iter().map(signed_area).collect();
    // Cells all wind the same way, so the largest ring is wound like an outer
    // one and every ring sharing its sign is an outer ring too.
    let outward = signs
        .iter()
        .max_by(|a, b| a.abs().total_cmp(&b.abs()))
        .map_or(1.0, |s| s.signum());
    let mut polys: Vec<Poly> = Vec::new();
    let mut outer_ix: Vec<usize> = Vec::new();
    for (i, r) in rings.iter().enumerate() {
        if signs[i].signum() == outward {
            outer_ix.push(i);
            polys.push(vec![r.clone()]);
        }
    }
    for (i, r) in rings.iter().enumerate() {
        if signs[i].signum() == outward {
            continue;
        }
        let mut best: Option<usize> = None;
        for (k, &oi) in outer_ix.iter().enumerate() {
            if contains(&rings[oi], r[0])
                && best.is_none_or(|b| signs[oi].abs() < signs[outer_ix[b]].abs())
            {
                best = Some(k);
            }
        }
        match best {
            Some(k) => polys[k].push(r.clone()),
            None => polys.push(vec![r.clone()]),
        }
    }
    polys
}
