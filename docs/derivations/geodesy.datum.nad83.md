<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Transform between NAD 83 and ITRF or WGS 84 (`geodesy.datum.nad83`)

## Method

NAD 83 rides on the North American plate and the ITRF does not, so the two drift apart and the difference between them in the conterminous United States is over a meter. The National Geodetic Survey publishes the relationship as fourteen-parameter Helmert transformations in its HTDP program, and the way HTDP applies them is part of the definition: everything routes through ITRF94, the parameters are evaluated at the coordinates' own epoch from their rates, and the position is carried in geocentric Cartesian coordinates through the chain. That routing is followed here rather than being collapsed into a single composed transformation, so the answer is the one HTDP gives rather than one that merely resembles it. The three NAD 83 realizations — (2011), (PA11), (MA11) — sit on three different plates and each has its own parameters.

## Equations

- A parameter at epoch t: p(t) = p(t₀) + ṗ · (t − t₀), with t₀ the parameters' reference epoch.
- Position-vector Helmert: X′ = T + (1 + s)·R(rx, ry, rz)·X.
- The chain from frame A to frame B: X_B = H_B( H_ITRF94( H_A⁻¹( X_A ) ) ), as HTDP composes it.
- The reported difference: X′ − X resolved into east, north, and up. The reported shift is the horizontal part alone, √(east² + north²), with its azimuth; the vertical part is the separate up figure. (The ITRF tool's shift is the full 3D distance — the two fields are labelled accordingly, "Horizontal shift" against "3D distance", and are not the same quantity.)

## Symbols and units

X is the geocentric position in meters on GRS80, which NAD 83 and the ITRF share; T the translations in meters, rx, ry, rz the rotations in milliarcseconds, s the scale in parts per billion, each with a yearly rate; t the epoch in decimal years. Heights are ellipsoidal, never orthometric.

## Domain

NAD 83 (2011), (PA11), and (MA11), against ITRF2020, ITRF2014, ITRF2008, ITRF2005, ITRF2000, and the WGS 84 realizations aligned with them, at any epoch. The transformations are defined for the plate each realization belongs to; applying (2011) to a Pacific or Marianas position, or the reverse, is a meters-level error that no warning can catch, since the coordinates alone do not say which realization they belong to.

## Approximations

The small-rotation Helmert form, exact well below a millimeter at these rotation sizes, and linear parameter rates, which is the form NGS publishes. Nothing is approximated in the routing: the chain through ITRF94 is walked as HTDP walks it, and an inverse transformation is applied as an inverse rather than by negating parameters.

## Worked example

- sourcePublisher: National Geodetic Survey, and independently the PROJ contributors from the EPSG dataset
- sourceTitle: HTDP, Horizontal Time-Dependent Positioning; and PROJ through pyproj
- sourceEdition: HTDP 3.6.0 compiled from htdp.f; PROJ 9.3.0, pyproj 3.6.1
- sourceLocator: HTDP menu option 4; and `Transformer.from_crs(CRS.from_epsg(9988), CRS.from_epsg(6319))` at epoch 2026.7
- independent: yes
- inputs: 38.5, −98, 500 m in WGS 84 (G2296) at epoch 2026.7, to NAD 83 (2011)
- outputs: latitude 38.49999435498922, longitude −97.99998619989742, height 501.02342184743014 m, a difference of 1.3572544389 m toward 117.4985129°
- tolerance: 1 mm against HTDP, the accuracy NGS states for the program's own output
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-23

Two references, reached by different routes, and they agree. The vectors come from HTDP itself, compiled from the Fortran NGS publishes, driven through its menu — the definition of the transformation rather than a description of it. PROJ was then asked the same question from the other end, given EPSG codes rather than parameters so that it resolved its own values and built its own pipeline, reported as "ITRF2020 to NAD83(2011) (1)". It returns latitude 38.499994354989305, longitude −97.9999861998974, and height 501.0234218472615: 9.5 nanometres, 1.2 nanometres, and 0.2 nanometres from the tool.

Worth keeping in proportion. The two implementations agree at the nanometre, but the published transformation is only good to one or two centimeters in the conterminous United States — seven orders of magnitude coarser. What the agreement establishes is that the arithmetic, the routing through ITRF94, the epoch propagation, and the realization mapping are all right; it says nothing about the geodesy being better than NGS says it is, and the limitations say so.

## Differential tests

- `tools/vectors/gen_datum.py`: 23 vectors from NGS HTDP compiled from `htdp.f` and `initbd.f`, menu option 4, across realizations, frames, positions, and epochs
- `core/crates/gp-geodesy/tests/datum.rs` `nad83_invariants`: the round trips, the epoch behavior, and the size and direction of the difference
- `core/vectors/geodesy.datum.nad83.jsonl`: 26 vectors, of which v001 is this worked example

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `nad83_invariants`: a frame to itself moves a position by nothing; every frame pair round trips back to within 110 nanometres, testing the inverse as an inverse rather than as negated parameters; the difference between NAD 83 (2011) and any ITRF in the conterminous United States is between one and two meters, which is the scale NGS documents and not an arbitrary band; that difference grows with epoch, since the plate keeps moving away from the frame; the east and north components recompose to the reported shift exactly, to the last bit, which pins that the shift is the horizontal distance and not the 3D one its sibling reports, and the azimuth is the one those two components imply; and WGS 84 (G2296) gives the same answer as ITRF2020, which is the coincidence the model claims and would otherwise be invisible
