<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Densify a line or polygon (`geometry.shape.densify`)

## Method

A line between two points means the geodesic between them, but almost everything that consumes it — a map renderer, a projection, a planar overlay — draws the straight chord instead. Over a long edge that chord can sit tens of kilometres from the curve it stands for. Densifying closes the gap by adding vertices along the real geodesic, close enough together that the chords between them follow it.

Each edge is handled on its own. Its geodesic length comes from the inverse problem, it is divided into the fewest equal pieces no longer than the maximum asked for, and the new vertices are placed along it by the direct problem. Equal pieces rather than as-many-full-length-pieces-as-fit, because a run of maximum-length pieces followed by a short remainder is an uglier and less useful sampling than an even one.

## Equations

- Edge length s from the geodesic inverse problem on WGS 84.
- Number of pieces: n = ⌈s / max⌉, with the ratio snapped to an integer when it is within 1e-12 of one.
- The k-th new vertex: the direct problem from the edge's start along its initial azimuth for a distance k·s/n.
- `longest_piece` is the largest s/n over all edges, and `length` the edges' lengths added.

## Symbols and units

The points are latitude and longitude in degrees; `max_length` is the greatest allowed spacing on the ground in the chosen unit. `vertices_in` and `vertices_out` count before and after, `longest_piece` is the largest spacing actually produced, and `length` is the total geodesic length of the shape.

## Domain

Lines and polygons of two or more points, anywhere on the ellipsoid, including across the antimeridian and over a pole. A very small maximum over a long route would produce more vertices than the result may carry, and is refused with the count it would have needed rather than truncated.

## Approximations

None. The inverse and direct problems are Karney's, exact to nanometres, and the division into equal pieces is arithmetic. The one judgement is the snapping described below, which decides an otherwise arbitrary case rather than approximating anything.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: geographiclib, the direct and inverse geodesic problems
- sourceEdition: geographiclib 2.1
- sourceLocator: `Geodesic.InverseLine(...).Position(s)` for each new vertex
- independent: yes
- inputs: fifteen cases — lines and polygons, a three-leg route, a transatlantic line, a meridian, the equator, the antimeridian, high latitudes, and an edge that is an exact multiple of the maximum
- outputs: the vertex count, the longest piece, the total length, and the position of an interior vertex
- tolerance: exact on the counts, 1e-9 relative on the lengths, 1e-9° on the placements
- verifiedBy: golden vectors v007 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The reference is a different implementation of Karney's own algorithms — his Python library against the core's Rust port — so the counts, which are decisions, and the placements, which are geometry, are both checked. The counts match on all fifteen, the lengths agree to 1.6e-13 relative, and the interior vertices land within 2.1e-14°.

Building this found a real defect, in the one place a densifier can have one. Walking 120 km by the direct problem and measuring back gives 119999.99999999964 m; a different solver gives a few femtometres over. At a 40 km maximum the first is ⌈2.999999999999991⌉ = 3 pieces and the second is ⌈3.0000000000000004⌉ = 4. So a user asking to cut a leg at exactly a third of its length got three pieces or four depending on the last bit of a double, with no way to predict which. The ratio is now snapped to an integer when it is within a part in a million million of one — 0.12 µm over that 120 km — so a third of a leg means three pieces either way. No published vector changed, since none of the existing six sat on that edge.

## Differential tests

- `tools/vectors/gen_densify_karney.py`: fifteen vectors from geographiclib, appended to the frozen file
- `core/crates/gp-geometry/tests/densify.rs` `densify_invariants`: the spacing promise, the subset property, and the exact-multiple case
- `core/vectors/geometry.shape.densify.jsonl`: 21 vectors, the first six from the original scenarios and the rest from geographiclib

## Invariants

- `core/crates/gp-geometry/tests/densify.rs` `densify_invariants`: no consecutive pair of output vertices is further apart than the maximum asked for, measured with geographiclib-rs rather than taken from the tool, which is the promise the tool makes; every input point survives, in order, so the result is a superset and nothing was moved; the total length does not change, since adding points along a geodesic does not lengthen it; a maximum longer than every edge leaves the shape untouched; halving the maximum never reduces the vertex count; an edge that is an exact multiple of the maximum is cut into exactly that many pieces rather than one more, which is the case the snapping exists for; and a polygon comes back closed, with one more edge than a line over the same points
