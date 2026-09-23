<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Transform between ITRF and WGS 84 realizations (`geodesy.datum.itrf`)

## Method

Each realization of the International Terrestrial Reference Frame is tied to ITRF2020 by a fourteen-parameter Helmert transformation: three translations, three rotations, a scale, and a rate for each, all referred to epoch 2015.0. A position is converted to geocentric Cartesian coordinates, the parameters for the requested frame are evaluated at the coordinates' own epoch by adding the rates times the elapsed years, the transformation is applied, and the result is converted back. Going between two frames that are not ITRF2020 chains through it: into ITRF2020 by the inverse of one set, out by the other. The WGS 84 realizations are carried as coincident with the ITRF each was aligned with, which is what the defining documents say and which the tool states in a warning rather than leaving implicit.

## Equations

- A parameter at epoch t: p(t) = p(2015.0) + ṗ · (t − 2015.0).
- The transformation, position-vector convention: X′ = T + (1 + s)·R(rx, ry, rz)·X, with R the small-rotation matrix and s in parts per billion.
- Between two frames A and B: X_B = H_B( H_A⁻¹( X_A ) ), each H evaluated at t.
- The reported shift is the length of X′ − X, resolved into east, north, and up at the position.

## Symbols and units

X is the geocentric position in meters; T the translations in meters, rx, ry, rz the rotations in milliarcseconds, s the scale in parts per billion, each with a rate per year; t is the epoch in decimal years. Latitude, longitude, and height may be given instead of X, Y, Z, in which case the ellipsoid is GRS80, on which the ITRF is defined.

## Domain

ITRF2020 back to ITRF88, and the WGS 84 realizations aligned with them, at any epoch. The parameters are published for the whole Earth and carry no area of use. Epochs far outside the span the rates were fitted over extrapolate rather than interpolate, which the rates permit arithmetically but which no publication supports.

## Approximations

The small-rotation form of the Helmert transformation, which is exact to well below a millimeter for rotations of a few milliarcseconds, and linear rates, which is the form IERS publishes. No approximation is made in chaining: the inverse of a Helmert transformation is applied as an inverse rather than by negating the parameters, so a round trip returns the position it started from.

## Worked example

- sourcePublisher: PROJ contributors, from the EPSG dataset
- sourceTitle: PROJ, through pyproj
- sourceEdition: PROJ 9.3.0, pyproj 3.6.1
- sourceLocator: `Transformer.from_crs(CRS.from_epsg(9988), CRS.from_epsg(7912))`, ITRF2020 to ITRF2014, at epoch 2026.72
- independent: yes
- inputs: 40.446111, −79.982222, 300 m in ITRF2020 at epoch 2026.72
- outputs: latitude 40.44611101524038, longitude −79.98222202049854, height 300.0011211390832 m
- tolerance: 1e-8 m, four times the difference actually seen
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-23

PROJ was given the EPSG codes rather than the parameters, so it looked up its own values from the EPSG dataset and built its own pipeline — reported as "Inverse of ITRF2014 to ITRF2020 (1)" — rather than being handed the numbers this tool uses. The latitude and longitude come back identical to the tool in every digit a double holds, and the height differs by 1.8 nanometres. That is agreement at the limit of the representation, not merely within a tolerance, and it covers the parameter values, the epoch propagation, the rotation convention, and the geocentric round trip at once, since any one of them being wrong would show at the millimeter level or larger.

## Differential tests

- `tools/vectors/gen_datum.py`: 21 vectors from PROJ's `+proj=helmert` with its ITRF2020 parameter file, through pyproj, over random positions, frame pairs, and epochs
- `core/crates/gp-geodesy/tests/datum.rs` `itrf_invariants`: the round trips, the chaining, and the epoch behavior
- `core/vectors/geodesy.datum.itrf.jsonl`: 25 vectors, of which v001 is this worked example and already carried PROJ's height of 300.0011211372912 m before PROJ was rerun on it here

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `itrf_invariants`: a frame to itself moves a position by under a fifth of a nanometre at any epoch, the worst being 1.16e-10 m on ITRF88 at 2030, which is not identically zero because the chain still runs out through ITRF2020 and back; all 25 frame pairs round trip to within four nanometres, which tests the inverse as an inverse rather than as negated parameters; going from ITRF2020 to ITRF2000 directly agrees with going through ITRF2008 to two nanometres, so the chaining is consistent whichever way it is assembled; the shift grows with the age of the frame — 2.7 mm to ITRF2014 against 179 mm to ITRF88 at the same epoch, sixty-six times larger — which is the ordering the published parameters imply; the shift changes with epoch for every pair, since all of them carry rates; and the east, north, and up components recompose to the reported shift to 3e-17 m
