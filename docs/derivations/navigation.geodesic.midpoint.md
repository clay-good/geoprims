<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geodesic midpoint (`navigation.geodesic.midpoint`)

## Method

The point halfway along the shortest path between two points on the ellipsoid — halfway by **distance travelled**, not halfway in latitude and longitude.

Averaging the coordinates is the wrong answer and is wrong by a lot. The mean of New York and London is at 46.06° N, 37.12° W; the true midpoint is at 52.24° N, 41.29° W — over 700 km away. On a long route the difference is larger still, and averaging across the antimeridian gives a point on the wrong side of the planet.

The method is two calls to Karney's algorithm and no interpolation at all. The inverse problem gives the initial azimuth and the total distance; the direct problem then walks half that distance along that azimuth. Because both are exact solutions rather than series approximations, the midpoint inherits their accuracy — about fifteen nanometres on WGS 84.

## Equations

- Inverse: (α₁, α₂, s₁₂) from the two endpoints.
- Direct: the point at distance s₁₂/2 along α₁ from the first point.
- Half distance = s₁₂/2.

## Symbols and units

`lat1`, `lon1`, `lat2`, `lon2` in degrees, with an optional `ellipsoid` or explicit `a` and `inverse_flattening`. Out come the midpoint's `lat` and `lon`, the `azimuth` there, and the `half_distance` in kilometres.

## Domain

Any two points. Nearly antipodal pairs are the hard case for any geodesic solver, and Karney's converges where Vincenty's does not.

## Approximations

None beyond the geodesic algorithm's own, which is about fifteen nanometres.

## Worked example

- sourcePublisher: Charles Karney (GeographicLib)
- sourceTitle: GeographicLib's `GeodSolve`
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeodSolve -i` for the azimuth and distance, `GeodSolve` for the direct problem at half that distance
- independent: yes
- inputs: 23 pairs, including JFK to Heathrow, Sydney to San Francisco, a nearly antipodal pair on the equator, a route across the antimeridian, one over the pole, a meridian, and a pair a few kilometres apart
- outputs: the midpoint and the half distance
- tolerance: 1e-9° on the coordinates, 1e-9 km on the distance
- verifiedBy: golden vectors v001 to v023, run by the core on every build
- verifiedOn: 2026-09-23

The reference calls `GeodSolve` twice per case — once inverse, once direct — rather than asking any tool for a midpoint. Two cases leave the longitude unpinned on purpose: a route over the pole has its midpoint *at* the pole, where longitude names no direction, and a midpoint landing on the antimeridian may be written +180 or −180, which is one place spelled two ways.

## Differential tests

- `tools/vectors/gen_nav_geodesic.py`: 12 of the 17 vectors, from `GeodSolve`
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.midpoint.jsonl`: 23 pairs across every awkward geometry

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `midpoint_invariants`: the midpoint is equidistant from both ends, checked through `navigation.geodesic.inverse`, which is the definition and not the construction; each of those distances is the reported half distance; the midpoint lies on the geodesic, so the distance from end to midpoint to end adds up to the whole with nothing left over; swapping the two endpoints gives the same point; the midpoint of two points on the equator is on the equator, and of two points on a meridian is on that meridian; and it is not the coordinate average — for New York to London the two differ by more than 700 km, which the test asserts rather than assumes
