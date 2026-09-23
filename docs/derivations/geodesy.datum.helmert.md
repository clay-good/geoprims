<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Helmert transformation (`geodesy.datum.helmert`)

## Method

A similarity transformation between two geocentric (ECEF) frames: rotate the position, scale it, and shift it. Seven parameters do this at one epoch; fourteen add a rate for each, so the parameters are first carried from their reference epoch to the epoch of the coordinate and then applied. The two published sign conventions differ only in which way the rotations turn, and the tool refuses rotations until it is told which one the parameters belong to, because the same numbers give different answers under each.

The reverse is computed by inverting the transformation, not by negating the parameters, which is an approximation the reverse of a rotation is not.

## Equations

- Parameters at epoch t: p(t) = p + rate × (t − t0), each of the seven in its own units.
- Rotation, linearized as EPSG defines it, with s = +1 for position-vector and −1 for coordinate-frame: R = [[1, −s·rz, s·ry], [s·rz, 1, −s·rx], [−s·ry, s·rx, 1]], the rotations in radians.
- Forward: V_T = M · R · V_S + T, with M = 1 + dS × 10⁻⁶.
- Reverse: V_S = R⁻¹ · (V_T − T) / M, solved by Cramer's rule on the 3 × 3 matrix (GN 7-2, "Reversibility").

## Symbols and units

V_S, V_T are ECEF X, Y, Z in metres. T is tx, ty, tz in metres. rx, ry, rz are arc-seconds, converted by π/(180 × 3600). dS is scale difference in parts per million. Rates are per year in the same units, with t and t0 decimal years. Position-vector is EPSG methods 1033 and 1053 (the IERS convention); coordinate-frame is 1032 and 1056.

## Domain

Any geocentric position, with the parameter sets published for frame ties and legacy datums: translations of hundreds of metres, rotations of a few arc-seconds, scale of a few parts per million. The linearized rotation is what EPSG and IERS publish parameters against, so it is the definition here rather than a shortcut; it departs from an exact rotation only in third order, which at an arc-second is far below a micrometre.

## Approximations

The rotation matrix is the linearized one, by definition of the EPSG methods. Nothing else is approximated: the scale and translation are exact, and the reverse inverts rather than negating. Negating the parameters instead would miss the starting point by 0.02 mm for a modern frame tie (4.5 m, 0.554″, 0.219 ppm), 1.6 mm for legacy-datum parameters (−146, 507, 685 m), and 208 mm at rotations of tens of arc-seconds with 100 ppm of scale.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Geomatics Guidance Note 7-2, Coordinate Conversions and Transformations including Formulas
- sourceEdition: IOGP 373-7-2, September 2019
- sourceLocator: §4.2.3, the position-vector worked example (WGS 72 to WGS 84: tz 4.5 m, rz 0.554″, dS 0.219 ppm)
- independent: yes
- inputs: x 3657660.66 m, y 255768.55 m, z 5201382.11 m, tz 4.5 m, rz 0.554 arcsec, scale 0.219 ppm, convention position-vector
- outputs: x 3657660.78 m, y 255778.43 m, z 5201387.75 m
- tolerance: 0.01 m (the note publishes the result to the centimetre)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_datum.py`: the GN 7-2 worked examples of §4.2.3 and §4.2.5, and PROJ's `+proj=helmert` through pyproj over both conventions and the time-dependent form
- `core/vectors/geodesy.datum.helmert.jsonl`: 24 vectors from those two sources, run through the core on every build

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `helmert_invariants`: over four positions, four parameter sets, and both conventions, the reverse returns the starting position within 1e-6 m, the two conventions agree once the rotations change sign, and zero parameters move nothing
- `core/crates/gp-geodesy/tests/datum.rs` `convention_required`: rotations without a convention are refused, naming the reason
