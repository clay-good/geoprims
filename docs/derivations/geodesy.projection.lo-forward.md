<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to South African Lo (`geodesy.projection.lo-forward`)

## Method

South African survey diagrams, general plans, and municipal datasets are drawn on the Lo system: transverse Mercator belts two degrees wide, each about an odd meridian from 15°E to 33°E, on the Hartebeesthoek94 datum. EPSG registers them as Hartebeesthoek94 / Lo15 to Lo33, codes 2046 to 2055, by the method IOGP Guidance Note 7-2 calls Transverse Mercator (South Orientated), EPSG 9808.

What sets the method apart is the axes. Y is a **westing** and X a **southing**: both count from the equator on the belt's central meridian and grow to the west and south. There is no false origin and the central scale factor is 1. So Y and X are the ordinary transverse Mercator easting and northing with their signs turned, and the tool computes them that way: Krüger's series in Karney's arrangement on WGS 84, then `Y = −E`, `X = −N`.

The belt comes from the longitude, the odd meridian nearest it, or from a belt you name when a job runs across a belt edge, which `NONSTANDARD_ZONE` reports. Beside Y and X the tool returns the **grid convergence** and the **point scale factor**. The south-orientated axes turn the grid round but do not change which way grid north points, so the convergence is the ordinary transverse Mercator one.

## Equations

- Belt: `λ₀ = 2⌊λ/2⌋ + 1`, kept to 15 ≤ λ₀ ≤ 33.
- `E = k₀·B₁·a·η`, `N = k₀·B₁·a·ξ`, with ξ and η Krüger's series in the conformal latitude and `λ − λ₀`.
- `Y = −E`, `X = −N`.
- Convergence `γ = γ₀ + atan2(q, p)`, near enough `(λ − λ₀)·sin φ`; point scale factor `k = k₀·B₁·k′·|p + iq|`.
- Parameters: `φ₀ = 0`, the belt's `λ₀`, `k₀ = 1`, no false westing or southing, WGS 84 (`a = 6378137 m`, `1/f = 298.257223563`).

## Symbols and units

Latitude and longitude in degrees on Hartebeesthoek94, north and east positive, so South Africa's latitudes are negative; Y (westing) and X (southing) in meters, positive west and south; the belt as its central meridian in degrees east; convergence in degrees; the point scale factor dimensionless.

## Domain

- Belts Lo15 to Lo33 on odd meridians. A longitude whose belt falls outside them is refused with `OUT_OF_DOMAIN` naming the range; so is a forced belt on an even meridian.
- Outside Hartebeesthoek94's published area, about 13°E to 43°E and 22°S to 50°S, the arithmetic still holds and the answer carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched; SAGEOID2010 is a separate asset.

## Approximations

The coordinates are taken to be on Hartebeesthoek94, which is ITRF91 at epoch 1994.0 on the WGS 84 ellipsoid. That is the `REALIZATION_ASSUMED` warning: a modern GNSS position has moved with the African plate since 1994, a few centimeters a year, and Cape datum coordinates are on the Clarke 1880 ellipsoid and are not this grid at all.

IOGP's worked example prints a westing 1.0 cm larger than this tool, PROJ, and GeographicLib all give. Its formula is the USGS series truncated at the fifth power, and the exact projection is the one the tool and the other two references agree on to a micrometer.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)
- sourceEdition: Revised September 2019
- sourceLocator: Section 3.2.3.3, Transverse Mercator (South Orientated), the Hartebeesthoek94 / Lo29 example
- independent: yes
- inputs: lat 25°43'55.302"S, lon 28°16'57.479"E
- outputs: Lo29, Y 71984.49 m, X 2847342.74 m as printed; 71984.480 m and 2847342.735 m exactly
- tolerance: 0.02 m on Y and 0.01 m on X against the printed values, for the series difference above
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/projections.rs` `lo_round_trips_every_belt_and_counts_west_and_south`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_lo.py` builds 18 of the 23 vectors in `core/vectors/geodesy.projection.lo-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, applying each belt's own EPSG record (checked against `projinfo`, axes `wsu` included, on every run): 17 places from Alexander Bay to Kosi Bay covering all ten belts within a millimeter, and one forced-belt row that requires `NONSTANDARD_ZONE`.
- `tools/vectors/gen_lo.py` builds 4 from GeographicLib's TransverseMercatorProj on WGS 84 with k = 1, negated for the south-orientated axes: the convergence, the point scale factor, and the coordinates to a micrometer, one of them on Lo27's meridian where the convergence is zero.
- The first row is IOGP's own example as printed.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `lo_round_trips_every_belt_and_counts_west_and_south`: in every belt a point west of the meridian has positive Y and X, round-trips to a nanodegree, and has a convergence of about `(λ − λ₀)·sin φ` and a scale factor just above 1; the point the same distance east has the same Y with the sign turned; the EPSG code is named; and at the origin both axes are zero with a scale factor of exactly 1.
- `core/crates/gp-geodesy/tests/projections.rs` `lo_refuses_an_even_or_foreign_belt_and_flags_a_forced_one`: even meridians and a longitude outside the belts are refused with `OUT_OF_DOMAIN`, and a forced neighbouring belt warns `NONSTANDARD_ZONE`.
