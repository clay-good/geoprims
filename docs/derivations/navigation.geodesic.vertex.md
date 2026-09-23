<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geodesic vertex (`navigation.geodesic.vertex`)

## Method

A geodesic on an ellipsoid is not a small circle of latitude: it rises to a highest latitude somewhere along its length and falls away again. That highest point is the **vertex**, and it is where a great-circle route surprises people — a flight from New York to London reaches 53.7° N, well north of either airport.

The vertex is exactly where the geodesic runs **due east**: the azimuth is 90°. Clairaut's relation makes that a formula rather than a search. On the auxiliary sphere the quantity sin α · cos β is constant along a geodesic, where β is the reduced latitude, so with sin α₀ = sin α₁ cos β₁ the vertex sits where α = 90° and cos β = sin α₀ — a quarter turn along the auxiliary sphere from the equator crossing.

The tool also reports whether the vertex falls **between** the two points or beyond one of them, which is the part that matters operationally: a route whose vertex lies outside it never reaches that latitude at all.

## Equations

- Reduced latitude: tan β = (1 − f) tan φ.
- Clairaut: sin α₀ = sin α₁ cos β₁, constant along the geodesic.
- σ₁ = atan2(sin β₁, cos α₁ cos β₁); the northern vertex is at σ = 90°.
- At the vertex, cos β = sin α₀ and the azimuth is 90°.
- `along` is the distance from the first point to the vertex; `within` says whether that falls in [0, s₁₂].

## Symbols and units

Either two points, or a point and an `azimuth`. Out come `vertex_lat`, `vertex_lon`, `along` in metres, `within`, and the `equator_azimuth` α₀.

## Domain

Any geodesic that is not along the equator. A geodesic on the equator has no vertex — it is at its highest latitude everywhere — and a meridian's vertex is the pole, where longitude names nothing.

## Approximations

None beyond the geodesic algorithm's own.

## Worked example

- sourcePublisher: Charles Karney (GeographicLib)
- sourceTitle: GeographicLib's `GeodSolve`
- sourceEdition: GeographicLib 2.7
- sourceLocator: the direct problem walked along the geodesic; the vertex is the latitude maximum
- independent: yes
- inputs: 21 geodesics, including JFK to Heathrow, Sydney to San Francisco whose vertex lies beyond the route, London to Tokyo, Rio to Sydney, and short lines at high latitude
- outputs: the vertex position, its distance along, and whether it falls within the route
- tolerance: 1e-7° on the latitude, 1e-2 m on the distance along
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The reference does not use Clairaut's relation at all. It walks the geodesic with the direct problem and bisects on where the latitude stops rising. So the two sides reach the same point by different routes — one by a closed-form relation on the auxiliary sphere, one by searching the line itself — and agreeing means the relation was applied correctly, not merely restated identically.

Two things about that search were wrong before they were right, and both are properties of the vertex rather than of the code. Latitude is **flat** at a vertex, which is what a vertex is, so maximising it locates the point but not the distance to it — a golden-section search on latitude left `along` seven centimetres out, while bisecting on the *sign* of the latitude derivative converges on the distance itself. And vertices repeat about every 40,008 km: the one meant is the one nearest the start, which may lie **behind** it at a negative distance. Searching only forward found the next one instead, a circumference further on and, because a geodesic does not close on an ellipsoid, a quarter of a degree away in longitude.

## Differential tests

- `tools/vectors/gen_nav_geodesic.py`: 8 of the 15 vectors, by search rather than by formula
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.vertex.jsonl`: 21 geodesics, with the vertex inside and outside the route

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `vertex_invariants`: the azimuth at the vertex is 90°, checked by running the direct problem to the reported distance and reading the azimuth there — which is the definition, tested against the answer rather than assumed; the vertex latitude is at least as far north as either endpoint, since it is the maximum; a point a little either side of it is lower, so it really is a maximum and not just a point on the line; `within` is yes exactly when `along` lies between zero and the route's length; the vertex of a geodesic and of the same geodesic reversed are the same place; and a route whose vertex lies beyond its end reports `within` as no while still naming a latitude neither endpoint reaches
