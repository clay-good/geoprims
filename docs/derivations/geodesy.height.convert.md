<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Ellipsoidal and orthometric height (`geodesy.height.convert`)

## Method

A GPS receiver reports height above the **ellipsoid**, a smooth mathematical surface. A map, a benchmark and a flood map report height above **mean sea level**, which follows the geoid — the equipotential surface the oceans would settle into, lumpy because the Earth's mass is not evenly distributed. The two differ by tens of metres, and which one you have is rarely written on the number.

The relation is one line:

**h = H + N**

where h is the ellipsoidal height, H the orthometric height, and N the geoid height — the geoid's own height above the ellipsoid at that point, positive where the geoid is above. N is not small: it runs from about +85 m near New Guinea to −106 m south of India, so treating a GPS height as a sea-level height is an error of that size, not a rounding.

N comes from a gravimetric model on a grid, EGM96 at 15 arcminutes here, interpolated to the point. Interpolation is either bicubic, which is smooth and is the default, or bilinear, which is what some older software used and is offered so an old number can be reproduced.

## Equations

- h = H + N; H = h − N.
- N from the EGM96 15′ grid, interpolated bicubically or bilinearly.
- With a terrain height, the height above ground follows from whichever of h and H the terrain is quoted in.

## Symbols and units

`lat`, `lon` and `height`, with `from` saying whether the height given is `ellipsoidal` or `orthometric`. `model` picks the geoid grid and `interpolation` the scheme. Out come the `converted` height, both heights side by side, the `geoid_height` used, and `agl` when a terrain height is given.

## Domain

Anywhere on Earth, including the poles and the antimeridian, where the grid wraps.

## Approximations

EGM96 is a model of the geoid, not the geoid: it is quoted at about 0.5 to 1 m against GPS-levelled benchmarks, better in well-surveyed regions and worse in mountains. The grid interpolation adds a few centimetres at 15′ spacing. For survey work a national geoid model — GEOID18 in the United States, for instance — is the right reference and this is not it.

## Worked example

- sourcePublisher: Charles Karney (GeographicLib); NASA/NIMA (the EGM96 model)
- sourceTitle: GeographicLib's `GeoidEval` with the egm96-15 grid
- sourceEdition: GeographicLib 2.7
- sourceLocator: h = H + N, with N the geoid height the egm96-15 grid gives at the point
- independent: yes
- inputs: 26 points with heights from −50 m to 3,000 m, including both poles, the antimeridian, grid edges, and eighteen drawn across the committed EGM96 fixture
- outputs: the converted height, the geoid height used, and both heights together
- tolerance: 5e-5 m, which is half the last digit `GeoidEval` prints
- verifiedBy: golden vectors v001 to v026, run by the core on every build
- verifiedOn: 2026-09-23

The geoid heights come from `GeoidEval`, GeographicLib's own evaluator, run over the same grid. That grid is not installed on this machine, so rather than guess at N the values are read from `core/crates/gp-geodesy/tests/data/egm96_diff.csv` — 2,010 points `GeoidEval` produced in an earlier run and that the repository keeps. It is the same reference at one remove, and saying so is the point: the numbers were not computed by the thing they are checking.

## Differential tests

- `core/crates/gp-geodesy/tests/geoid.rs`: the full 2,010-point fixture, both interpolation schemes
- `tools/vectors/gen_geodesy_last.py`: 18 of the 26 vectors, from that same fixture
- `core/vectors/geodesy.height.convert.jsonl`: 26 conversions across the globe and six height bands

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `height_convert_invariants`: the two directions are exact inverses, so a height converted and converted back is the height it started as, to the last bit; h − H is the reported geoid height at every point, which is h = H + N stated as an identity rather than assumed; the geoid height does not depend on the height given, only on the place; it agrees with `geodesy.geoid.geoid-height` at the same point, so the two tools share one model rather than carrying copies; the two interpolation schemes differ by centimetres and not metres; and the sign is the right way round — where the geoid is above the ellipsoid, the sea-level height is the smaller of the two
