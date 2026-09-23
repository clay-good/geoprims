<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Delaunay triangulation (`geometry.mesh.delaunay`)

## Method

The Delaunay triangulation is the one where no point falls inside any triangle's circumcircle — the triangulation that avoids thin slivers wherever a fatter arrangement exists, which is why it is the one interpolation is built on. It is found here by the lifting map: raise every point from the plane onto the paraboloid z = x² + y², take the convex hull of the lifted points, and the faces looking downward project back to exactly the Delaunay triangles. The circumcircle test becomes a question of which side of a plane a lifted point lies on, which is a determinant rather than a construction.

On the planar surface the points are first projected to an azimuthal equidistant plane at their centre. On the spherical surface no projection is needed: the points become directions on the unit sphere and the triangulation is the convex hull of those directions, which is the same argument one dimension up.

## Equations

- Lifting: (x, y) ↦ (x, y, x² + y²).
- In-circle test: the sign of the determinant of the 4×4 matrix of the lifted coordinates, which is positive exactly when the fourth point lies inside the first three's circumcircle.
- Triangle count for n points with h on the convex hull: 2n − 2 − h.
- Spherical: the triangulation of the unit vectors is the convex hull of those vectors.

## Symbols and units

The points are latitude and longitude in degrees. `triangles` gives one-based indices into the input; `triangle_count` is how many there are; `surface_used` says whether the answer was computed on the plane or the sphere.

## Domain

Three or more points, not all on one line. On the planar surface the spread should be within the range where an azimuthal equidistant plane is faithful, tens to hundreds of kilometres; beyond that the spherical surface is the right choice and is offered for it. Points that repeat are a degenerate input rather than a hard one.

## Approximations

None in the combinatorics: the in-circle test is a determinant whose sign is either right or wrong, and the triangulation that comes out is exact for the surface chosen. The projection is the only modelling step, and it is what `surface` selects. Where four or more points share a circle the triangulation is not unique — either diagonal of the quadrilateral is correct — and one is chosen consistently rather than arbitrarily per run.

## Worked example

- sourcePublisher: the GEOS contributors
- sourceTitle: GEOS through shapely, `delaunay_triangles`
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0
- sourceLocator: `shapely.delaunay_triangles` on `+proj=aeqd +ellps=WGS84` coordinates
- independent: yes
- inputs: fifteen point sets — five points, scatters from seven to sixteen points, jittered 3×3 and 4×4 grids, a long thin scatter, two clusters, rings with and without a centre, and sets at the equator, at 70° north, in the southern hemisphere and across the antimeridian
- outputs: the full triangulation of each, as index triples
- tolerance: exact — every triangle, on every set
- verifiedBy: golden vectors v006 to v020 and the fixture `core/crates/gp-geometry/tests/data/delaunay_geos.json`
- verifiedOn: 2026-09-23

This is one of the few things in the catalog where a reference does not merely have to agree closely. A Delaunay triangulation of points in general position is unique, so GEOS's answer and this one must be the same set of triangles or one of them is wrong. Over fifteen point sets they are the same set every time.

The vectors pin the triangle count and the fixture pins the triangles themselves, because a count alone would pass for a triangulation with the right number of wrong triangles. The comparison is made as a set of sorted index triples, so neither the order the triangles come out in nor the order of the vertices within one can hide a difference.

Cocircular points are kept out deliberately. Two of the cases are rings, and a perfect ring is cocircular: four points on a circle can be split either way and neither is wrong. They happened to agree when the ring was perfectly circular, but that was the projection breaking the tie rather than the question having one answer, so the radii are nudged per point and the sets are now in general position by construction.

## Differential tests

- `core/crates/gp-geometry/tests/mesh.rs` `matches_qhull`: the pre-existing check against Qhull through SciPy, on both surfaces
- `core/crates/gp-geometry/tests/mesh_parity.rs` `delaunay_matches_geos_triangle_for_triangle`: fifteen point sets against GEOS's own triangulation, compared as sets — a second independent implementation, reached by a different algorithm than Qhull's
- `tools/vectors/gen_delaunay_geos.py`: the same fifteen as vectors, pinning the triangle count
- `core/vectors/geometry.mesh.delaunay.jsonl`: 20 vectors, the first five from the original scenarios and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/mesh_parity.rs`: the reported `triangle_count` is the number of distinct triangles actually returned, so a duplicate cannot inflate it; every input point appears in at least one triangle, so none was silently dropped; and no triangle refers to a point that does not exist
- `core/crates/gp-geometry/tests/mesh.rs` `delaunay_invariants`: the triangle count obeys Euler's relation, 2n − 2 − h for n points with h on the hull, with h taken from `geometry.shape.enclosing` rather than counted here; no triangle repeats a vertex; every interior edge is shared by exactly two triangles and every hull edge by one, which is what makes the result a triangulation rather than a set of triangles; and the triangles' areas sum to the hull's area, taken from `geometry.area.polygon`
