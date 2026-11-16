<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# South African Lo to latitude and longitude (`geodesy.projection.lo-inverse`)

## Method

This is the forward tool read backwards: Y (westing) and X (southing) on a Lo belt, back to the Hartebeesthoek94 latitude and longitude. The Lo axes count west and south, so the tool turns their signs to an ordinary easting and northing, `E = −Y`, `N = −X`, and runs the inverse of Krüger's series in Karney's arrangement on WGS 84 with the belt's parameters. It is the forward's own inverse, so the round trip closes to a nanodegree.

The belt is an input, and it has to be. Every belt counts from its own meridian, so the same Y and X name a place in each, two degrees of longitude apart.

Beside the coordinates the tool reports the **grid convergence** and the **point scale factor** at the point it found, from projecting the answer forward again.

## Equations

- `E = −Y`, `N = −X`.
- The inverse series from `ξ = N/(k₀·B₁·a)`, `η = E/(k₀·B₁·a)` to the conformal latitude, then the geodetic latitude by Newton's method, and `λ = λ₀ + atan2(sinh η′, cos ξ′)`.
- Parameters: `φ₀ = 0`, the belt's `λ₀`, `k₀ = 1`, no false westing or southing, WGS 84.

## Symbols and units

Y (westing) and X (southing) in meters, positive west and south; the belt as its central meridian, an odd number of degrees east from 15 to 33; latitude and longitude in degrees on Hartebeesthoek94; convergence in degrees; the point scale factor dimensionless.

## Domain

- Belts Lo15 to Lo33 on odd meridians; an even meridian is refused with `OUT_OF_DOMAIN`.
- Y and X must be finite meters; a pair the inverse cannot place is refused on `/southing`.
- An answer outside Hartebeesthoek94's published area carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

The answer is on Hartebeesthoek94; that is the `REALIZATION_ASSUMED` warning. Y and X read off a Cape datum plan are on the Clarke 1880 ellipsoid and are not this grid.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)
- sourceEdition: Revised September 2019
- sourceLocator: Section 3.2.3.3, Transverse Mercator (South Orientated), the Hartebeesthoek94 / Lo29 reverse example
- independent: yes
- inputs: zone 29, westing 71984.49 m, southing 2847342.74 m
- outputs: lat 25°43'55.302"S, lon 28°16'57.479"E
- tolerance: 2e-7 degrees, the 1 cm the printed westing carries
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/projections.rs` `lo_round_trips_every_belt_and_counts_west_and_south`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_lo.py` builds 17 of the 22 vectors in `core/vectors/geodesy.projection.lo-inverse.jsonl` from PROJ 9.9.0 through `cs2cs` in reverse, applying each belt's own EPSG record: places in all ten belts read back within a nanodegree.
- `tools/vectors/gen_lo.py` builds 4 from GeographicLib's TransverseMercatorProj: the place to 1e-11 degrees with the convergence and the point scale factor beside it.
- The first row is IOGP's reverse example as printed.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `lo_round_trips_every_belt_and_counts_west_and_south`: a point in every belt goes out through the forward tool and comes back through this one to a nanodegree, with the same convergence at both ends.
- `core/crates/gp-geodesy/tests/projections.rs` `lo_refuses_an_even_or_foreign_belt_and_flags_a_forced_one`: even meridians are refused on `/zone`.
- `tools/trust/example-pairs.mjs` carries the forward tool's worked example through this one and requires it to land back on the same point.
