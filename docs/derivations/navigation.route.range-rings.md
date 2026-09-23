<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Range rings (`navigation.route.range-rings`)

## Method

Circles of constant distance around a point — a fuel radius, a radio horizon, a search area, a restricted zone. On a sphere or an ellipsoid these are not circles on a map, and drawing them as such is a common and expensive mistake: at high latitude a projected "circle" of fixed map radius covers far less ground on one side than the other.

Each ring here is built from the geodesic direct problem. Step the azimuth in equal increments from the centre — counterclockwise, with the default of 144 points giving one every two and a half degrees — and solve for the point that distance away along each. Every vertex is then exactly the requested distance from the centre, by construction, whatever the latitude and whatever the projection it is later drawn in.

The enclosed area is measured with Karney's geodesic polygon algorithm on the ring that results, so it is the area of the polygon actually returned rather than πr², which would be the answer on a plane and is not the answer here.

## Equations

- Ring point k: the direct problem from the centre at azimuth 360°·k/n and distance r.
- Area: Karney's geodesic polygon area over those n points.
- Chord sag: a ring of n points sags below the true circle by r(1 − cos(π/n)), which at n = 144 is 0.024% of the radius.

## Symbols and units

`lat` and `lon` give the centre; `radii` is a list of radius objects; `points` sets how many vertices per ring. Out come the `ring_count`, a per-ring `summary` with its area, and the `rings` themselves, with a GeoJSON file.

## Domain

Any centre, any radii. A ring around a pole wraps the longitude circle, and a ring larger than a quarter of the Earth is still a valid geodesic circle though no longer a useful one.

## Approximations

The vertices are exact; the ring between them is a chord, not an arc. At the default 144 points each chord sags 0.024% of the radius below the true circle — 24 m on a 100 km ring — so the reported area is very slightly under the true geodesic disc. Raising `points` reduces it as 1/n².

## Worked example

- sourcePublisher: Charles Karney (GeographicLib)
- sourceTitle: GeographicLib's `GeodSolve` and `Planimeter`
- sourceEdition: GeographicLib 2.7
- sourceLocator: the direct problem at equal azimuth steps; `Planimeter` for the geodesic polygon area
- independent: yes
- inputs: 22 ring sets, including single and multiple radii, a centre on the equator, one at 89° north, one in the far southern ocean, and radii from 25 to 2,000 km
- outputs: the ring count and each ring's enclosed area
- tolerance: a part in a million of the area
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The reference builds each ring itself with `GeodSolve` and then hands the vertices to `Planimeter`. Neither step asks the core anything, and the area comes from the same polygon algorithm the geometry domain is checked against elsewhere.

## Differential tests

- `tools/vectors/gen_nav_geodesic.py`: 6 of the 17 vectors, rings from `GeodSolve` and areas from `Planimeter`
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.route.range-rings.jsonl`: 22 ring sets from the equator to 89° north

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `range_rings_invariants`: every point on a ring is the requested distance from the centre, checked through `navigation.geodesic.inverse` rather than trusted from the construction — which is the property the whole tool exists to provide; a ring has the number of points asked for; the rings come back in the order the radii were given and their areas increase with radius; more points give a larger area, since the inscribed polygon approaches the circle from below; and a ring at 89° north spans the whole longitude circle, which a map-circle approximation would not
