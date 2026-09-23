<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Voronoi cells (`geometry.mesh.voronoi`)

## Method

Every point gets the territory that is nearer to it than to any other point. The boundary between two neighbours is the perpendicular bisector of the line joining them, and the cell of a point is the intersection of the half-planes on its side of every such bisector. Only the neighbours matter, and the neighbours are exactly the points a Delaunay edge joins it to — so the diagram is built from the triangulation rather than from every pair.

A corner of the diagram, where three cells meet, is a point equidistant from three generators: the circumcentre of the Delaunay triangle they form. The two structures are the same information seen from either side.

The outer cells are unbounded, because nothing stops the territory of an edge point running away from the set. On the planar surface they are cut at a box around the points, padded by a tenth of their spread. On the spherical surface no cutting is needed: every cell closes on a sphere.

## Equations

- Bisector between points p and q: the locus of x with |x − p| = |x − q|.
- Cell of p: the intersection over its Delaunay neighbours q of the half-plane {x : |x − p| ≤ |x − q|}.
- Diagram corner: the circumcentre of a Delaunay triangle, equidistant from its three vertices.
- Clip box: the points' extent, grown on every side by a tenth of the larger of the two spreads.

## Symbols and units

The points are latitude and longitude in degrees. `cells` lists the corners of every cell, each tagged with the one-based index of the `point` whose territory it is; `cell_count` is how many cells came back; `surface_used` says whether the plane or the sphere was used.

## Domain

Three or more points, not all on one line, on the plane or the sphere. There is one cell per point. The outer cells depend on the box and are therefore a presentation of the diagram rather than the diagram itself; the interior cells are determined by the points alone.

## Approximations

None in the construction: the bisectors are exact and the corners are exact circumcentres. The clip box is a choice, not an approximation, and it is the one thing about the answer that is not determined by the points — which is why it is stated in the model rather than left implicit.

## Worked example

- sourcePublisher: the GEOS contributors
- sourceTitle: GEOS through shapely, `voronoi_polygons`
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0
- sourceLocator: `shapely.voronoi_polygons(points, extend_to=box)` on `+proj=aeqd +ellps=WGS84` coordinates, with cell areas by geographiclib's PolygonArea
- independent: yes
- inputs: the same fifteen point sets the Delaunay note uses — scatters, jittered grids, rings, two clusters, a long thin line, and sets at the equator, at 70° north and across the antimeridian
- outputs: every cell's corner count and its area on the ellipsoid
- tolerance: exact on the corner counts; 1e-5 relative on the areas, against a worst seen of 4.4e-7
- verifiedBy: golden vectors v006 to v020 and the fixture `core/crates/gp-geometry/tests/data/voronoi_geos.json`
- verifiedOn: 2026-09-23

Getting the clip box right turned out to be the whole difficulty, and it is worth recording because it looked exactly like a defect. Padding each axis by a tenth of its own spread rather than by a tenth of the larger one gives a quite different box for a long thin set: on eight points strung out east to west, my reference produced cells of 159,000 m² against the tool's 1,220,000 m², a 633% disagreement that had nothing to do with the diagram. Once the box matched, all fifteen sets agreed on every cell's corner count and on every area to 4.4e-7 relative.

The vectors pin the cell count and a fixture pins the cells, because a cell count is only the number of points and would pass for any diagram at all.

## Differential tests

- `core/crates/gp-geometry/tests/mesh.rs` `matches_qhull`: the pre-existing check against Qhull through SciPy, on both surfaces
- `core/crates/gp-geometry/tests/mesh_parity.rs` `voronoi_matches_geos_cell_for_cell`: fifteen diagrams against GEOS, comparing each cell's corner count and its area measured by `geometry.area.polygon`
- `core/vectors/geometry.mesh.voronoi.jsonl`: 20 vectors, the first five from the original scenarios and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/mesh.rs` `voronoi_invariants`: there is exactly one cell per point and `cell_count` is the number actually returned; each generator lies inside its own cell, by `geometry.predicate.point-in-polygon` rather than by anything this tool decided; a point taken from inside a cell is nearer to that cell's generator than to any other, measured with geographiclib — which is the definition of a Voronoi cell and the only test that would catch cells assigned to the wrong points; the cells' areas sum to the clip box's area, so they tile it with no gap and no overlap; and every corner shared by three cells is equidistant from their three generators, which is what makes it a circumcentre
