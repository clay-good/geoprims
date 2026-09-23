<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Distance between two tracks (`geometry.distance.tracks`)

## Method

Three different questions about the same pair of paths, answered three ways.

The Fréchet distance is the shortest leash two walkers need if each starts at the beginning of one track, ends at its end, and neither may walk backwards. It is computed by the Eiter-Mannila recurrence over the geodesic distances between every pair of points, and because the coupling that achieves it is recorded, the tightest pair — the moment the leash is at full stretch — is reported with it.

The Hausdorff distance forgets order entirely: it is the furthest any point of one track is from the other track, taken both ways round and the larger kept. Each point's distance is measured to the other track's *segments*, not only to its vertices, so a track sampled coarsely is not penalised for the gaps between its points.

The closest approach is simply the nearest the two ever come, which is zero when they cross.

## Equations

- Discrete Fréchet, Eiter-Mannila: F(i, j) = max( d(aᵢ, bⱼ), min( F(i−1, j), F(i−1, j−1), F(i, j−1) ) ), the answer at the far corner.
- Directed Hausdorff: h(A, B) = max over a ∈ A of min over segments s of B of d(a, s).
- Hausdorff: H = max( h(A, B), h(B, A) ).
- Closest approach: the least distance between any segment of A and any segment of B, zero where they cross.
- Every d is a geodesic distance from Karney's inverse problem.

## Symbols and units

A and B are the two tracks as ordered points, latitude and longitude in degrees; `frechet`, `hausdorff` and `closest` are distances in the chosen unit; `frechet_a` and `frechet_b` are the one-based indices of the pair where the leash is tightest.

## Domain

Two tracks of two or more points each; they need not have the same number. Any part of the world, including across the antimeridian. The Fréchet distance is the discrete one over the points as given, so it is a statement about the sampled tracks rather than about the continuous paths they stand for.

## Approximations

None in the recurrence or the maxima, which are exact over the points given. The distances are geodesic and exact to under a micrometre. What is not exact is the question: discrete Fréchet over a coarse sampling is not the continuous Fréchet distance of the paths, and the gap is about the coarser track's own step. That is a property of the measure, not an error, and the limitations say so.

## Worked example

- sourcePublisher: the GEOS contributors, with PROJ
- sourceTitle: GEOS through shapely, on PROJ's azimuthal equidistant plane
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0
- sourceLocator: `shapely.frechet_distance`, and `LineString.distance` for the Hausdorff maxima and the closest approach
- independent: yes
- inputs: fourteen pairs of tracks — beside each other, crossing, curving against straight, unequal in length and in sampling, reversed, and at the equator, at 70° north, in the southern hemisphere and across the antimeridian
- outputs: the Fréchet distance, the Hausdorff distance and the closest approach of each
- tolerance: 5e-8 relative, about eight times the worst seen
- verifiedBy: golden vectors v007 to v020, run by the core on every build
- verifiedOn: 2026-09-23

GEOS carries its own implementation of the discrete Fréchet distance, so `shapely.frechet_distance` is a second answer to the same question rather than the same code twice. Its `distance` is used for the Hausdorff maxima and the closest approach. Everything is computed on the azimuthal equidistant plane, where GEOS measures in the plane while the core measures along the geodesic — two measurements of one distance by different means. Across all fourteen pairs and all three outputs the worst disagreement is 6.3e-9 relative, on the pair straddling the antimeridian.

Two of the cases are there to show what the numbers mean rather than to stress the arithmetic. A track against a coarser version of itself has a Hausdorff distance of 3.7 mm — the coarse track lies along the fine one — but a Fréchet distance of 199.9 m, which is the coarse track's own step, because the leash must stretch from a point on one to the nearest point *in order* on the other. And a track against itself reversed has a Hausdorff distance of zero, the two covering identical ground, against a Fréchet distance of 1,399 m, the whole length of the track, since the walkers must start at opposite ends and may not turn back.

## Differential tests

- `tools/vectors/gen_tracks_geos.py`: fourteen vectors from GEOS, appended to the frozen file
- `core/crates/gp-geometry/tests/distance.rs` `track_distance_invariants`: the metric properties and the relationships between the three measures
- `core/vectors/geometry.distance.tracks.jsonl`: 20 vectors, the first six from GeodSolve and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/distance.rs` `track_distance_invariants`: a track against itself gives zero for the Fréchet distance and the closest approach and a nanometre for the Hausdorff, which is the geodesic solver finding a point on a segment it is already on; all three are symmetric in their arguments, and the Fréchet distance stays symmetric even though its recurrence is not; the ordering closest ≤ Hausdorff ≤ Fréchet holds on every pair, which is forced by what each measures and would break if any one of them were computed wrongly; reversing one track leaves the Hausdorff distance and the closest approach untouched while the Fréchet distance jumps, which is the difference between an ordered and an unordered measure; the closest approach is zero exactly when the tracks cross; moving one track bodily away increases all three together; and the tightest Fréchet pair is a real pair of indices into the two tracks, at a geodesic distance equal to the reported Fréchet distance
