<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Polygon area and perimeter on the ellipsoid (`geometry.area.polygon`)

## Method

The area a ring of points encloses on the ellipsoid, with each edge taken as the geodesic between its ends, which is what a boundary on the ground is. Karney's method sums a term for each edge that depends only on that edge, so the total is the area to the left of the ring as it is walked; the sign of the sum gives the winding, and a ring that takes in a pole is recognised by the total turning rather than by any test of the vertices. The perimeter is the sum of the edges' geodesic lengths.

## Equations

- Each edge contributes the area of the quadrilateral between it, the equator, and the two meridians through its ends, from the geodesic's own parameters.
- The signed total is the enclosed area; its magnitude is reported with the winding beside it.
- Pole enclosure: the ring's total change in longitude is ±360° when a pole is inside, and the area is completed by adding the remaining spherical excess.
- Perimeter: the sum of the geodesic distances, each from the inverse geodesic problem.

## Symbols and units

Latitudes and longitudes in degrees, closed implicitly: the last point joins the first. Area in square kilometres, perimeter in kilometres. Orientation is clockwise or counter-clockwise as seen from outside the ellipsoid. Holes are given as their own rings.

## Domain

Any simple ring on any ellipsoid in the registry, from a field to a continent, across the antimeridian, and around either pole. A ring that crosses or touches itself has no single area and is refused, naming `geometry.validity.make-valid` as the repair.

## Approximations

None beyond the geodesic solver itself, which is exact to the precision of a double. Two consequences are worth stating because they surprise people. A ring whose corners all lie on one parallel still encloses area: a parallel is not a geodesic, every edge bows towards the pole, and the long closing edge bows further than the short hops, leaving a sliver — three points a degree apart at latitude 10 enclose 18.2 km². And a ring strung along a meridian retraces its own path, which is a self-touching ring rather than a zero-area one, so it is refused; GeographicLib's Planimeter answers zero for the same input, because it does not test validity.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: Planimeter, the area of a geodesic polygon on WGS 84
- sourceEdition: GeographicLib version 2.7
- sourceLocator: `printf "37 -109.05\n41 -109.05\n41 -102.05\n37 -102.05\n" | Planimeter` gives 4 points, perimeter 2099854.381923 m, area −269154549884.0 m²; run for this note
- independent: yes
- inputs: the four corners of Colorado, 37° to 41° N and 109.05° to 102.05° W
- outputs: area 269154.5498840107 km², perimeter 2099.8543819229058 km, orientation clockwise, pole none
- tolerance: 1e-6 km² on the area and 1e-9 km on the perimeter, which is the precision Planimeter prints
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_area_diff.py`: 500 polygons against Planimeter, from fields to continents, both windings, across the antimeridian, and around the poles
- `core/crates/gp-geometry/tests/area.rs`: that fixture, run on every build
- `core/vectors/geometry.area.polygon.jsonl`: 29 vectors from the same reference

## Invariants

- `core/crates/gp-geometry/tests/area.rs` `polygon_area_invariants`: starting the ring at any of its vertices gives the same area, perimeter, and winding; walking it the other way turns the winding round and leaves the numbers alone; Colorado cut along 39° N has the area of its two halves to within a square metre; a ring along a meridian is refused as self-touching; and three points on a parallel enclose the 18.2185662 km² Planimeter gives for them
