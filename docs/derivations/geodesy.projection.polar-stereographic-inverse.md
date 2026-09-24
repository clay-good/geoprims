<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Polar Stereographic inverse (`geodesy.projection.polar-stereographic-inverse`)

## Method

A polar stereographic projects the ellipsoid from one pole onto a plane at the other, conformally, so it keeps shapes and angles. It is the projection of the polar caps: UPS, and the Arctic and Antarctic grids of sea-ice, ice-sheet, and satellite products. Meridians are straight lines from the pole and parallels are circles around it.

The method follows IOGP Guidance Note 7-2 in its two common forms. Variant A (EPSG method 9810) gives the scale factor at the pole; UPS is variant A with 0.994. Variant B (EPSG method 9829) gives instead the latitude of true scale, the standard parallel, from which the scale at the pole follows. The south case is the north case mirrored, with the latitude negated and the northing direction reversed. Grid north is the direction of the central meridian, so the convergence is the longitude difference itself, with its sign set by the pole.

This tool is the inverse: an easting and northing back to latitude and longitude, with the same parameters as the forward direction.

## Equations

- ρ′ = √((E − FE)² + (N − FN)²); t′ = ρ′ K/(2 a k₀)
- φ from t′ by the same fixed-point iteration as the Lambert conic, negated for the south pole
- North: λ = λ₀ + atan2(E − FE, −(N − FN)); south: λ = λ₀ + atan2(E − FE, N − FN); at the pole, λ₀

## Symbols and units

`easting` and `northing` in any length unit, with the same parameters as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

Any easting and northing. The pole itself returns the central meridian as its longitude.

## Approximations

None beyond double precision; the inverse latitude is iterated to 1e-14 radians.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.2.2.1 (variant A, WGS 84 / UPS North) and 3.2.2.2 (variant B, WGS 84 / Australian Antarctic Polar Stereographic)
- independent: yes
- inputs: E = 3,320,416.75 m, N = 632,668.43 m; E = 7,255,380.79 m, N = 7,053,389.56 m
- outputs: 73° N, 44° E on UPS North; 75° S, 120° E on the Australian Antarctic grid
- tolerance: 0.01 m on the grid; 1e-7° on the way back
- verifiedBy: golden vectors v001 and v002, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the grid from PROJ's stere, both variants and both poles, on five ellipsoids; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves
- `tools/diff/runner.mjs`: `polar-stereographic-forward` covers the forward direction; the inverse is checked by returning each of the 300 reference grid points to its latitude and longitude, within 0.01 mm of ground

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, from 1° to 89° of latitude
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: on the central meridian the convergence is zero; mirroring a point across the central meridian mirrors the easting and the convergence; the two scales are equal; the scale at the pole is the scale factor; and with the pole, 0.994, and 2,000 km falsings it gives the same easting and northing as the UPS tool
