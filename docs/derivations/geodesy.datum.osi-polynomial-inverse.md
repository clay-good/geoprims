<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ETRS89 to Irish Grid by the OSi and LPS polynomial (`geodesy.datum.osi-polynomial-inverse`)

## Method

The OSi/LPS polynomial gives the shift from TM75 to ETRS89 as a function of the TM75 position, so the reverse has no closed form. As Ordnance Survey's guide says, it is found by iteration. Start from the ETRS89 position, subtract the shifts computed there, compute the shifts again at the new estimate, and repeat until the estimate moves by less than 1e-11 degrees, about a micrometer. The TM75 latitude and longitude then go onto the Irish Grid by OS's annexe B formulas. The OSGM15 geoid height on the datum named, taken at the ETRS89 point by the method described for the forward tool, is subtracted from the ellipsoidal height.

## Equations

- `φₖ₊₁ = φ_ETRS − Δφ(φₖ, λₖ)/3600`, `λₖ₊₁ = λ_ETRS − Δλ(φₖ, λₖ)/3600`, from `(φ₀, λ₀) = (φ_ETRS, λ_ETRS)`.
- Stop when both move by under 1e-11 degrees, within 50 iterations.
- `H_orthometric = h_ellipsoidal − N_OSGM15`.

## Symbols and units

Latitude and longitude in degrees on ETRS89; eastings, northings, and heights in meters; shifts in arcseconds.

## Domain

- The height datum is required: Belfast or Malin Head.
- Outside OSGM15's Irish grids the answer is `OUT_OF_DOMAIN`.
- The input is taken to be ETRS89, the `REALIZATION_ASSUMED` warning.

## Approximations

The polynomial's own 0.4 m (95%) against the old triangulation. The iteration converges to well under a millimeter of easting and northing within a few steps. Heights carry the resampling difference described in the forward note, at most 0.64 mm against OS's published results.

## Worked example

- sourcePublisher: Ordnance Survey
- sourceTitle: OSGM15 Northern Ireland test files, in the OSTN15-OSGM15 Developers Pack
- sourceEdition: Test files of 2016-05-09
- sourceLocator: OSGM15_NI_TestOutput_ETRS89toIG.txt, NI_ETRS89_1
- independent: yes
- inputs: lat 55°10'57.600354"N, lon 6°23'6.102018"W, height 124.5604 m, datum Belfast
- outputs: E 302925.602 m, N 438473.935 m, 68.4590 m
- tolerance: 0.0005 m on the easting and northing; 0.001 m on the height
- verifiedBy: golden vector v001 and `core/crates/gp-geo/tests/osi.rs` `ni_test_point_both_ways`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_osi.py` builds all 100 vectors in `core/vectors/geodesy.datum.osi-polynomial-inverse.jsonl` from Ordnance Survey's OSGM15_NI_TestOutput_ETRS89toIG.txt: every easting and northing to the printed millimeter, and every orthometric height to a millimeter.

## Invariants

- `core/crates/gp-geo/tests/osi.rs` `ni_test_point_both_ways`: the iteration returns the TM75 point the forward polynomial started from, to 1e-10 degrees, in under ten steps.
