<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# British National Grid to GNSS position by OSTN15 (`geodesy.datum.ostn15-inverse`)

## Method

OSTN15's shifts are tabulated on the ETRS89 side, so going from OSGB36 back to ETRS89 needs Ordnance Survey's iteration (Transformations and OSGM15 User Guide v1.3, chapter 3):

1. Compute the shifts at the OSGB36 easting and northing themselves, as if they were ETRS89, and subtract them for a first estimate. The geoid height is added to the orthometric height.
2. Compute the shifts at that estimate and subtract them from the OSGB36 coordinates again.
3. Repeat until the easting and northing shifts change by no more than 0.1 mm.
4. Take the converged ETRS89 easting and northing off the National Grid projection on GRS 80 by OS's annexe C.

The tool returns the converged ETRS89 easting and northing, the shifts and datum flag at the answer, and the number of iterations, with every estimate shown in the explanation.

## Equations

- `xₖ₊₁ = E − se(xₖ, yₖ)`, `yₖ₊₁ = N − sn(xₖ, yₖ)`, `hₖ₊₁ = H + sg(xₖ, yₖ)`, starting from `(x₀, y₀) = (E, N)`.
- Stop when `|seₖ₊₁ − seₖ| ≤ 0.0001 m` and `|snₖ₊₁ − snₖ| ≤ 0.0001 m`.
- Annexe C: `φ′` by iterating the meridian arc until `|N − N₀ − M| < 0.01 mm`, then `φ = φ′ − VII·Δ² + VIII·Δ⁴ − IX·Δ⁶` and `λ = λ₀ + X·Δ − XI·Δ³ + XII·Δ⁵ − XIIA·Δ⁷` with `Δ = E − E₀`.

## Symbols and units

Eastings, northings, heights, and shifts in meters; latitude and longitude in degrees on ETRS89; the datum flag a small whole number.

## Domain

- Inside the grid and not where the datum flag is 16: elsewhere `OUT_OF_DOMAIN`.
- The orthometric height given is taken to be on the datum OSGM15 flags at the point.
- Missing or damaged grid: `ASSET_UNAVAILABLE` or `ASSET_INTEGRITY`.

## Approximations

The iteration stops at OS's 0.1 mm, so the answer can sit up to a few tenths of a millimeter from an exact inverse. At Caister it is 0.3 mm east of the point the forward example started from, which is 4e-9 degrees of longitude.

## Worked example

- sourcePublisher: Ordnance Survey
- sourceTitle: Transformations and OSGM15 User Guide
- sourceEdition: v1.3, November 2020
- sourceLocator: Annexe A, the inverse of the Caister Water Tower example, with its iteration table
- independent: yes
- inputs: easting 651409.804 m, northing 313177.450 m, height 63.822 m
- outputs: ETRS89 E 651307.0032 m, N 313255.6860 m, 108.0504 m, in 3 iterations; 52.658007833°N 1.716073972°E
- tolerance: 0.00005 m on the iterated values; 1e-8 degrees on the latitude and longitude
- verifiedBy: golden vector v001 and `core/crates/gp-geo/tests/ostn15.rs` `caister_water_tower_step_by_step`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_ostn15.py` builds the other 40 vectors in `core/vectors/geodesy.datum.ostn15-inverse.jsonl` from Ordnance Survey's own OSGB36-to-ETRS89 test file, TP01 to TP40. Each pins the converged ETRS89 easting, northing, and ellipsoidal height and the final shifts to OS's 0.1 mm convergence tolerance, the datum flag exactly, and the latitude and longitude to 1e-8 degrees. The iteration count is pinned only for Caister: OS's printed counts do not follow its stated stopping rule.

## Invariants

- `core/crates/gp-geo/tests/ostn15.rs` `caister_water_tower_step_by_step`: every iteration's shifts, estimates, and heights against OS's table, and the converged position back to latitude and longitude.
- `tools/trust/example-pairs.mjs` would carry a forward example through this tool if the two shared a title and inputs; here they share Caister, checked in the Rust test above instead, because the forward tool takes latitude and longitude and this one easting and northing.
