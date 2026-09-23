<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Centroid and interior point (`geometry.shape.centroid`)

## Method

The centre of mass of a region on a curved surface is not defined until you say how area is to be weighted, so the choice of map is part of the answer rather than an implementation detail. The choice made here is an equal-area one: the polygon's geodesic edges are cut into pieces no longer than 5 km, those points are mapped onto a Lambert azimuthal equal-area plane centred at the corners' mean, the ordinary planar centroid is taken there with holes subtracted, and the result is mapped back. Because the map preserves area exactly, weighting in the plane is weighting on the ellipsoid, and the answer does not depend on how the shape happens to be oriented.

A second point comes back beside it. For a concave shape the centre of mass can lie outside the polygon — a C, a horseshoe, a ring — which is correct and useless for a label. So the pole of inaccessibility is also computed on the same plane, the point inside the shape furthest from any edge, and its clearance is reported as a geodesic distance.

## Equations

- Authalic latitude β from geodetic φ, so that equal areas on the ellipsoid map to equal areas on the sphere (Snyder eq. 3-11 to 3-16).
- Lambert azimuthal equal-area about (β₀, λ₀): k = √(2 / (1 + sin β₀ sin β + cos β₀ cos β cos Δλ)), x = k cos β sin Δλ, y = k (cos β₀ sin β − sin β₀ cos β cos Δλ), scaled by the authalic radius (Snyder ch. 24).
- Planar centroid of a ring: Cx = (1/6A) Σ (xᵢ + xᵢ₊₁)(xᵢ yᵢ₊₁ − xᵢ₊₁ yᵢ), Cy likewise, with A the signed area; holes enter with the opposite sign.
- Interior point: the pole of inaccessibility by Agafonkin's polylabel, a quadtree refined until the cell is within 0.1% of the shape's narrow side.

## Symbols and units

φ and λ are geodetic latitude and longitude in degrees; β the authalic latitude; x and y the plane coordinates in meters; A the area, reported in the chosen area unit. `clearance` is the geodesic distance from the interior point to the nearest edge, in meters. `centroid_inside` says whether the centre of mass lies within the polygon.

## Domain

One to a few thousand vertices, with holes by ring, anywhere on the ellipsoid including across the antimeridian and over a pole. The equal-area map is exact everywhere, but a shape spanning a large fraction of the Earth is squeezed hard on the far side of the projection's centre, so this is meant for shapes up to a few hundred kilometers, where it agrees with a local plane to well under a meter. A self-intersecting outline has no well-defined interior and should be repaired first.

## Approximations

Two, and both are bounded and stated. Edges are cut into 5 km pieces, so a geodesic edge is represented by a chain of chords in the plane; the chord sag over 5 km is millimetres, and the test below measures what it costs. The interior point is found to within 0.1% of the shape's narrow side rather than exactly, since the pole of inaccessibility has no closed form; for an extreme sliver it is the best found within a fixed amount of work, and it is always genuinely inside. The centroid itself is exact for the plane it is computed on — the modelling choice is which plane, not how well it is solved.

## Worked example

- sourcePublisher: PROJ contributors and the GEOS contributors
- sourceTitle: PROJ's ellipsoidal Lambert azimuthal equal-area through pyproj, with GEOS's planar centroid through shapely
- sourceEdition: PROJ 9.3.0, GEOS 3.11.4, geographiclib 2.1
- sourceLocator: `+proj=laea +ellps=WGS84 +lat_0=… +lon_0=…`, with `Polygon.centroid` and `Polygon.area` over edges walked by `Geodesic.InverseLine`
- independent: yes
- inputs: twelve shapes — a square, an L, a C, a triangle, a narrow wedge, a long strip, a twelve-sided ring, and squares at the equator, at 70° north, in the southern hemisphere and across the antimeridian
- outputs: the centroid and the area of each
- tolerance: 1e-6° on the centroid and 5e-8 relative on the area, a few times the residual actually measured
- verifiedBy: golden vectors v013 to v024, run by the core on every build
- verifiedOn: 2026-09-23

Nothing in the reference comes from the core. PROJ's ellipsoidal LAEA does the authalic conversion internally, GEOS computes the planar centroid and area, and geographiclib walks the geodesic edges. Over the twelve shapes the worst centroid disagreement is 2.9e-7° — 23 millimetres, on the C shape — and the worst area disagreement is 4.6e-9 relative, on the narrow wedge. On a square in Colorado the area agrees to 2.3e-11.

Two things went wrong building this reference and both are worth recording, because each looked at first like a fault in the tool. Projecting the geodetic latitudes with a spherical `+R=` LAEA rather than the ellipsoidal `+ellps=WGS84` one skips the authalic conversion and puts every area out by 0.11% — small enough to look like a disagreement rather than a mistake. And averaging the longitudes of the antimeridian shape gave (179.6 + −179.7 + −179.7 + 179.6)/4 = 0, a projection centre on the opposite side of the planet, which moved the centroid 800 m; longitudes have to be unwrapped before they can be averaged. The tool had both right.

## Differential tests

- `tools/vectors/gen_centroid_geos.py`: twelve vectors from PROJ and GEOS, appended to the frozen file rather than rewriting it
- `core/crates/gp-geometry/tests/shape.rs` `centroid_invariants`: the symmetries, the area, the concave case, and the interior point's clearance
- `core/vectors/geometry.shape.centroid.jsonl`: 24 vectors, the first twelve from symmetry and local-plane centroids and the rest from PROJ and GEOS

## Invariants

- `core/crates/gp-geometry/tests/shape.rs` `centroid_invariants`: for a shape symmetric about a meridian the centroid lies on that meridian exactly, and for one symmetric about the equator it lies on the equator, which no amount of projection error could fake; the reported area agrees with `geometry.area.polygon` for the same polygon to a part in ten million, taken from that tool rather than recomputed — they are not identical, and should not be, since this one measures the densified polygon on the equal-area plane and that one integrates the geodesic edges; reversing the winding of the vertices changes nothing; the centroid of a convex shape is inside it and the centroid of a C shape is not, with `CENTROID_OUTSIDE` raised only in the second case; the interior point is confirmed inside by `geometry.predicate.point-in-polygon` in both cases, which is the property it exists to guarantee; and the clearance is that tool's own distance from the point to the nearest edge, which it reaches by a different route and agrees on to under a nanometre
