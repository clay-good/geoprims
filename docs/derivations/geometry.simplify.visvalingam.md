<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Visvalingam-Whyatt simplification (`geometry.simplify.visvalingam`)

## Method

Every interior vertex forms a triangle with the vertex before it and the vertex after. The area of that triangle is how much the line would change if the vertex were dropped — Visvalingam and Whyatt call it the effective area. The rule is then to repeatedly drop the vertex with the least effective area, recomputing its two neighbours' triangles each time, until nothing left is under the threshold or the wanted number of vertices remains.

One detail makes the result stable. Removing a vertex can leave a neighbour with a *smaller* effective area than the one just removed, which would make the order of removal depend on rounding. So an effective area is never recorded as less than that of the vertex removed before it. The sequence of removals is then monotone, and the same line simplified to different thresholds gives nested results rather than unrelated ones.

The areas are computed on an azimuthal equidistant plane at the shape's centre; the deviation that is reported afterwards is a geodesic distance.

## Equations

- Effective area of vertex vᵢ: the area of the triangle vᵢ₋₁ vᵢ vᵢ₊₁, |(vᵢ − vᵢ₋₁) × (vᵢ₊₁ − vᵢ₋₁)| / 2.
- Removal order: least effective area first, with each recorded area raised to at least the previous one removed.
- Stop when every remaining interior vertex has effective area ≥ the threshold, or the target count is reached.
- `max_deviation`: the greatest geodesic distance from a dropped vertex to the line that replaced it.

## Symbols and units

The points are latitude and longitude in degrees; `area` is the effective-area threshold on the ground in the chosen area unit, and `target_vertices` is an alternative way to stop. `vertices_in` and `vertices_out` count before and after, `restored` counts vertices put back to stop edges crossing, and `max_deviation` is in the chosen length unit.

## Domain

Lines and polygons of any length, over shapes up to a few hundred kilometres across where the plane the areas are computed on is faithful. A threshold of zero, or a target equal to the input count, keeps everything; a target of two reduces a line to its ends.

## Approximations

The triangle areas are planar, exact to about a part in a thousand for shapes of a few hundred kilometres. The rule itself is exact: which vertex has least area is a comparison, and the answer is either right or wrong. The reported deviation is a true geodesic distance rather than a planar one.

## Worked example

- sourcePublisher: Stephan Hügel (urschrei)
- sourceTitle: the `simplification` crate, Visvalingam-Whyatt
- sourceEdition: simplification 0.7, through its Python binding
- sourceLocator: `simplify_coords_vw(coords, epsilon)` on `+proj=aeqd +ellps=WGS84` coordinates
- independent: yes
- inputs: six shapes — a gentle curve, a zigzag, a near-straight line, a sharp dogleg, a long wandering track and a high-latitude track — each at thresholds of 1,000, 20,000, 200,000 and 2,000,000 m²
- outputs: which vertices survive
- tolerance: exact — the same vertices, in all twenty-four cases
- verifiedBy: golden vectors v006 to v029, run by the core on every build
- verifiedOn: 2026-09-23

GEOS does not implement Visvalingam-Whyatt, so the reference is an unrelated implementation of the same paper: urschrei's Rust crate, reached through its Python binding. It shares no code and no lineage with the core's. Over six shapes at four thresholds the two keep exactly the same vertices, every time — which is the claim worth making, since which vertices survive is the entire answer.

The reference had to be installed against python3.11, because the crate has no wheel for the default interpreter here; the generator's header says so, since a generator that cannot be re-run is a dead end.

The shapes are deliberately the same ones the Douglas-Peucker vectors use, so the two rules can be compared on identical input. They behave differently on purpose: on the zigzag at a middling threshold Douglas-Peucker keeps the vertices furthest from the chord while this keeps the ones that carry area, and neither is a better answer to the other's question.

## Differential tests

- `tools/vectors/gen_vw_simplification.py`: twenty-four vectors from the `simplification` crate, appended to the frozen file
- `core/crates/gp-geometry/tests/simplify.rs` `visvalingam_invariants`: the subset property, the monotonicity, and the target-count path
- `core/vectors/geometry.simplify.visvalingam.jsonl`: 29 vectors, the first five from the original implementations and the rest from the crate

## Invariants

- `core/crates/gp-geometry/tests/simplify.rs` `visvalingam_invariants`: the result is the input's own vertices in their own order, matched numerically rather than by formatting, so nothing was moved or invented; the first and last points always survive; raising the threshold never keeps more vertices, and the results nest — everything kept at a high threshold is still kept at a lower one, which is what the monotone removal order buys and what an unstable implementation would break; asking for a target number of vertices returns exactly that many; a target of two returns the two ends; and preserving topology never returns fewer vertices than not preserving it
