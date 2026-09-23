<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Legacy datum to WGS 84 (`geodesy.datum.legacy`)

## Method

A legacy datum is an ellipsoid of its own, positioned to fit one region well and the rest of the Earth not at all. Converting a position to WGS 84 means leaving that ellipsoid: the latitude and longitude are turned into geocentric Cartesian coordinates on the datum's own ellipsoid, the published transformation is applied to those coordinates, and the result is read back as latitude and longitude on WGS 84. The transformation is whichever operation EPSG publishes for that datum — a three-parameter geocentric translation for most, a seven-parameter Helmert where EPSG gives one — used exactly as published, including the choice of which of several competing operations is the one cited. The reverse direction applies the same operation backwards.

## Equations

- To geocentric on the source ellipsoid: X = (N + h) cos φ cos λ, Y = (N + h) cos φ sin λ, Z = (N(1 − e²) + h) sin φ, with h taken as zero since these are geog2D operations.
- Geocentric translation: X′ = X + T.
- Seven-parameter Helmert, position-vector convention: X′ = T + (1 + s)·R(rx, ry, rz)·X.
- Back to geographic on WGS 84 by the closed-form inverse, with the height discarded: these are geog2D operations and the tool carries no height.
- The reported shift is the geodesic distance between the two positions, with its azimuth.

## Symbols and units

φ and λ are latitude and longitude in degrees; X, Y, Z geocentric coordinates in meters; T the translation in meters; rx, ry, rz rotations in arcseconds and s the scale in parts per million, where the operation has them; N the prime vertical radius of the source ellipsoid. The stated accuracy is the EPSG figure for the operation, in meters.

## Domain

Eight datums, each with the EPSG operation cited and its published area of use: ED50 (EPSG:1133), NAD27 (1173), OSGB36 (1314), Tokyo (1305), AGD66 (1108), Pulkovo 1942 (1267), SAD69 (1864), and Arc 1960 (1122). Outside a transformation's area of use the answer is flagged: the parameters were fitted to that region and carry no meaning beyond it. These are two-dimensional operations, so heights pass through untouched.

## Approximations

The approximation is the whole idea, not the arithmetic. One set of parameters is standing in for a continental datum that was surveyed piecemeal over decades and was never internally consistent to better than meters, so the error varies from place to place within the region and the single published figure — 2 m for OSGB36, 35 m for Arc 1960 — is a summary rather than a bound at any given point. Where a national grid exists, such as NADCON5 or OSTN15, the grid models that variation and is the better tool.

## Worked example

- sourcePublisher: PROJ contributors, from the EPSG dataset
- sourceTitle: PROJ, through pyproj
- sourceEdition: PROJ 9.3.0, pyproj 3.6.1
- sourceLocator: `Transformer.from_pipeline("EPSG:1133")`, ED50 to WGS 84 (1), on Paris at 48.8566, 2.3522
- independent: yes
- inputs: ED50, latitude 48.8566, longitude 2.3522
- outputs: latitude 48.85568546266484, longitude 2.350914333016232, a shift of 138.72 m toward 222.85°
- tolerance: 1e-9°, five to ten orders of magnitude inside the 10 m EPSG states for this operation
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-23

PROJ was handed a bare operation code and nothing else — no parameters, no ellipsoid, no convention — so everything it used came from its own copy of the EPSG dataset. It returns 48.85568546266484 and 2.350914333016232, identical to the tool in every digit a double carries.

The other seven datums were checked the same way, each on a point inside its own area of use: NAD27 in Denver, OSGB36 in London, Tokyo in Seoul, AGD66 in Sydney, Pulkovo 1942 in Moscow, SAD69 in São Paulo, Arc 1960 in Nairobi. Every one agrees with PROJ to 1.6 nanometres or better, most of them to 0.8, which is last-bit rounding in the geographic conversion rather than any difference in the transformation. What that establishes is that the right operation is being selected and applied in the right direction with the right ellipsoid — the things that go wrong here — since a wrong operation for a datum is a shift of tens of meters, not nanometres.

Keep the scale in view. The agreement with PROJ is at 1e-9 m; what EPSG says these transformations are worth is 2 to 35 m. Every answer carries `LOW_ACCURACY_TRANSFORM` with the figure for that operation, because the precision of the arithmetic here is in no way the accuracy of the result.

## Differential tests

- `core/crates/gp-geodesy/tests/datum.rs` `legacy_matches_proj_on_every_datum`: all eight datums against PROJ's own EPSG operation, each on a point inside its area of use
- `tools/vectors/gen_datum.py`: 18 vectors from PROJ applying the EPSG operation through pyproj, across datums and points
- `core/vectors/geodesy.datum.legacy.jsonl`: 20 vectors, of which v001 is this worked example

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `legacy_invariants`: converting to WGS 84 and back returns the position to within 5 mm for all eight datums, which tests the reverse direction as an inverse rather than as a second set of parameters. It is not exact, and should not be: these are two-dimensional operations, the tool takes no height, and the vertical part of the translation — tens of meters — is discarded at each crossing, which comes back as a few millimeters horizontally. That is four orders of magnitude inside the 2 to 35 m the transformations are worth, and the test holds it at 5 mm so that a real regression in the reverse direction could not hide in it; the reported shift is the distance between the two positions as geographiclib-rs measures it, not as this tool computes it; every shift is between 10 m and 500 m, which is the scale these datums actually differ by and not an arbitrary band; `LOW_ACCURACY_TRANSFORM` is raised on all eight with the accuracy EPSG states for that operation, never a default; and a point well outside a transformation's area of use raises `OUTSIDE_AREA_OF_USE` while one inside does not
