<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# State plane coordinates (SPCS2022 beta), inverse (`geodesy.spcs.spcs2022-inverse`)

## Method

The inverse uses the same asset-backed NGS zone definition as the forward tool. Lambert and Hotine recover latitude by iterating the isometric-latitude equation to double precision. Transverse Mercator uses the reverse sixth-order Krüger series.

## Equations

- LC1 solves `t = (r / aF)^(1/n)` and iterates `φ` from isometric latitude; `λ = λ₀ + θ/n`.
- TM reverses the conformal coordinates and Krüger series, then recovers geodetic latitude.
- OMC reverses the variant-B grid rotation and projection-center offset before solving the Hotine isometric latitude.

## Symbols and units

`E` and `N` are easting and northing in meters or international feet. `φ` and `λ` are latitude and longitude in the zone's named 2022 terrestrial reference frame.

## Domain

The zone, unit, and reference frame must be the ones used to create the coordinates. Results outside the published zone bounds carry `OUTSIDE_ZONE_EXTENT`.

## Approximations

The same double-precision series and iterative solutions as the forward projection are used. No rounded display value enters the calculation.

## Worked example

- sourcePublisher: National Geodetic Survey, NOAA
- sourceTitle: SPCS2022 Example Coordinates and Distortion Values
- sourceEdition: Beta, June 1, 2026
- sourceLocator: 1 published coordinate in each of 953 zones
- independent: yes
- inputs: official easting and northing check values
- outputs: latitude and longitude in the zone's reference frame
- tolerance: `1e-8` degree from the millimeter-rounded NGS coordinates
- verifiedBy: public-tool golden vectors and forward/inverse round trips
- verifiedOn: 2026-09-28

## Differential tests

- `core/crates/gp-geodesy/tests/spcs.rs`: runs NGS examples through the public inverse tool; all 953 unrounded forward results are inverted by the shared whole-registry test.

## Invariants

- `core/crates/gp-geodesy/src/spcs2022.rs` `every_zone_matches_the_official_ngs_example`: each forward result returns to its source latitude and longitude within `1e-11` degree. Beta status adds `NON_OFFICIAL_DATUM` with the publication date.
