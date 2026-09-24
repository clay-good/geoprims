<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Boolean overlay of two polygons (`geometry.overlay.boolean`)

## Method

Boolean overlay is a planar operation, so the first job is to get onto a plane honestly. Both polygons' geodesic edges are cut into pieces no longer than 5 km and projected onto a single azimuthal equidistant plane placed at the mean of all their corners — one plane for both, because two polygons compared on different planes would not line up. There the ordinary overlay runs: each boundary piece is tested for which side of the other polygon it lies on by the even-odd rule, and a piece survives exactly when the answer the operation asks for differs across it. The surviving pieces are chained into rings and mapped back, and their areas are measured on the ellipsoid rather than in the plane.

## Equations

- The four operations, as sets: A ∩ B, A ∪ B, A \ B, and A △ B = (A \ B) ∪ (B \ A).
- Keep a piece of ∂A when inside(B) differs across it for the operation, and likewise for ∂B; the surviving pieces close into rings.
- Even-odd membership: a point is inside when a ray from it crosses the boundary an odd number of times.
- Area of a returned ring: the geodesic polygon area on WGS 84 (Karney 2013 §6), holes subtracted.

## Symbols and units

The polygons are latitude and longitude in degrees; `area` is the result's area, `area_a` and `area_b` the inputs' own, all on the ellipsoid in the chosen unit; `parts` counts the disjoint pieces of the result; `result` is the boundary, with `part` and `ring` on every vertex so holes can be told from outlines.

## Domain

Two polygons whose corners and result all lie within 5,000 km of their shared centre. Beyond that the plane stops being faithful and the tool refuses rather than answering approximately. Rings may cross the antimeridian, which the projection handles as ordinary geometry since it never compares longitudes. An input that crosses itself has no well-defined inside and should be repaired first.

## Approximations

One: the 5 km densification, which makes a geodesic edge a chain of chords, so a returned boundary follows the true geodesic to about a millimetre over shapes of a few hundred kilometres. The overlay itself is exact on the pieces it is given, and the areas are exact geodesic areas of the corners returned — the approximation is in where the corners are, not in what is then measured.

## Worked example

- sourcePublisher: the GEOS contributors, with PROJ and geographiclib
- sourceTitle: GEOS through shapely, on PROJ's azimuthal equidistant plane, measured by geographiclib's PolygonArea
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0, geographiclib 2.1
- sourceLocator: `Polygon.__and__`, `__or__`, `__sub__` and `__xor__` on `+proj=aeqd +ellps=WGS84` coordinates
- independent: yes
- inputs: two overlapping squares near Boulder, one square inside another, two that do not meet, and two across the antimeridian, through all four operations
- outputs: the result's area and part count, and each input's own area
- tolerance: 1e-10 relative on areas, exact on part counts
- verifiedBy: golden vectors v013 to v022, run by the core on every build
- verifiedOn: 2026-09-23

GEOS is the reference implementation for this operation — it is what PostGIS, QGIS and most of the field compute overlays with — so the only question worth asking is whether the core agrees with it, and the answer is that it does to eleven or twelve digits. Over ten cases spanning all four operations the worst area disagreement is 2.2e-11 relative, on the intersection of nested squares, and most are nearer 1e-12. Every part count matches exactly, including the two-part cases: the symmetric difference of overlapping squares, and the union of two squares that never meet.

The comparison is fair because both sides work on the same plane. The reference does not re-derive the projection or the densification; it takes the same construction the tool's model describes, has PROJ place the plane and geographiclib walk the edges, and then asks GEOS to do the one thing being tested. What is left over after that is the overlay itself, which is what the vectors pin.

## Differential tests

- `tools/vectors/gen_overlay_geos.py`: ten vectors from GEOS across all four operations, appended to the frozen file
- `core/crates/gp-geometry/tests/overlay.rs` `overlay_invariants`: the set identities that must hold between the four operations
- `core/vectors/geometry.overlay.boolean.jsonl`: 27 vectors, the first twelve from Planimeter and Clairaut and the rest from GEOS; v023 to v027 are regressions for 1.1.0
- `tools/vectors/gen_overlay_grid.py` and `core/crates/gp-geometry/tests/overlay_parity.rs` `overlay_matches_geos_on_degenerate_shapes`: 400 pairs on a small grid built to be degenerate (shared edges, corners on edges, pieces touching at a point, holes) through all four operations, 4,800 checks: the planar overlay agrees with GEOS exactly on the grid, gives the same answers with every corner moved by up to 0.2 µm, and the geodesic tool at a one-meter step returns GEOS's parts with areas within 2 cm². Version 1.0.0 failed 630 of these checks: a corner a fraction of a micron off an edge (as a corner on an edge is once projected) was nudged across it by the side test, which lost or kept the wrong pieces, and pieces touching at a point came back as one ring touching itself. 1.1.0 puts corners within a millionth of the shapes' size (at most 1 mm) of another ring onto it before the overlay, and joins rings by the sharpest left turn where they meet at a corner

## Invariants

- `core/crates/gp-geometry/tests/overlay.rs` `overlay_invariants`: the four operations are tied to each other by arithmetic that no single one of them could satisfy alone — the union plus the intersection equals the two input areas added, the symmetric difference equals the union minus the intersection, and the difference equals A minus the intersection, each to a part in a billion of the polygon areas — judged against those areas rather than against the answer, since A\B minus B\A is a difference of two numbers near 0.758 km² and measuring an error of 1e-12 km² against it would be measuring the cancellation instead of the overlay; intersection and union are symmetric in their arguments while difference is not, and A \ B and B \ A differ by exactly the two input areas; a polygon against itself gives itself for intersection and union and nothing at all for difference; two polygons that do not meet intersect in nothing and unite into two parts; and each input's reported area is the one `geometry.area.polygon` gives it, taken from that tool rather than recomputed here
