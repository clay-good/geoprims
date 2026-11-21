<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# LV03 to LV95 by swisstopo's CHENyx06 (`geodesy.datum.chenyx06`)

## Method

Switzerland's older national grid, LV03, rests on a triangulation adjusted in 1903, and the error propagation of its own measurements left it bent. LV95 is the readjusted grid, and its false origin is a round 2,000 km east and 1,000 km north of LV03's — so subtracting 2,000,000 m and 1,000,000 m looks like the conversion, but it leaves the distortion behind, and swisstopo puts that at up to 1.6 m. swisstopo publishes the difference as the data set CHENyx06 and allows a transformation "on the 1-metre level" to skip it. This tool applies it.

LV03 and LV95 are the same projection, the Swiss oblique Mercator, with different false origins, so the round offset can be taken off at the very end and nothing else changes:

1. **Unproject.** The LV03 coordinates, with the round 2,000 km added so the projection's own false origin applies, go back through the Swiss oblique Mercator (`gp_geo::proj::swiss`, swisstopo's rigorous formulas) to a latitude and longitude on CH1903, the Bessel ellipsoid.
2. **Interpolate.** CHENyx06 gives a latitude offset and a longitude offset in arcseconds at nodes 30 arcseconds apart, from 5°33′E 45°28′N to 11°03′E 48°04′N. The four nodes around the point are weighted bilinearly, through `gp_geo::hgrid`.
3. **Add.** The offsets give the latitude and longitude on CH1903+, the datum LV95 is defined on.
4. **Project.** Forward through the same oblique Mercator to LV95.
5. **Reverse.** From LV95 the offsets are taken away at the latest estimate, repeatedly, until the estimate moves by under 1e-12 degrees, as PROJ's hgridshift inverts an NTv2 grid; the round offset comes off at the end.

The distortion reported is what the grid adds beyond the round offsets, measured LV03 to LV95 whichever way the tool was asked to run.

## Equations

- `x = (λ − 5°33′)/30″`, `y = (φ − 45°28′)/30″`, `c = ⌊x⌋`, `r = ⌊y⌋`, `t = x − c`, `u = y − r`.
- `Δ = (1 − t)(1 − u)Δ₀₀ + t(1 − u)Δ₀₁ + (1 − t)u·Δ₁₀ + tu·Δ₁₁`, for each of `Δφ` and `Δλ`.
- `φ₁₉₀₃₊ = φ₁₉₀₃ + Δφ/3600`, `λ₁₉₀₃₊ = λ₁₉₀₃ + Δλ/3600`.
- Reverse: `φₖ₊₁ = φ₁₉₀₃₊ − Δφ(φₖ, λₖ)/3600`, the same for `λ`, from `(φ₀, λ₀) = (φ₁₉₀₃₊, λ₁₉₀₃₊)`.
- The projection each way is swisstopo's: `E = R·l̄ + 2,600,000`, `N = (R/2)·ln((1 + sin b̄)/(1 − sin b̄)) + 1,200,000`, with `R = 6,378,815.90365 m`.
- Distortion: `ΔE = E₉₅ − (E₀₃ + 2,000,000)`, `ΔN = N₉₅ − (N₀₃ + 1,000,000)`.

## Symbols and units

Easting and northing in metres, on LV03 or LV95 as the direction says; latitude and longitude in degrees on CH1903 and CH1903+ between the steps; the grid's offsets in arcseconds, longitude east positive; the distortion in metres.

## Domain

- Inside the grid, 5°33′E to 11°03′E and 45°28′N to 48°04′N; outside it the answer is `OUT_OF_DOMAIN`.
- The grid is fetched on first use: `ASSET_UNAVAILABLE` without it, `ASSET_INTEGRITY` for a damaged copy.

## Approximations

swisstopo transforms with FINELTRA, a triangulated affine transformation on the projection plane, and offers a grid approximation of the same differences beside it in its own documentation. This is the grid, so it does not reproduce FINELTRA exactly: against the five EUREF points swisstopo publishes it lands within 8.1 mm, against 1.6 m for the round shift alone. The grid is stored as the float32 values swisstopo's file carries, about 1e-6 arcseconds, under a millimetre on the ground. Its 30-arcsecond spacing is written into the package header at full precision, because it is not a round decimal and the node count multiplies it: rounded to nine decimals the spacing moves the offsets the grid interpolates by up to 1.1 um, measured, and at full precision what is left is 4 pm, from GDAL's own rounding of the file's geotransform. Heights are untouched: LN02 to LHN95 is swisstopo's HTRANS, which is not here.

## Worked example

- sourcePublisher: Federal Office of Topography swisstopo
- sourceTitle: Formulas and constants for the calculation of the Swiss conformal cylindrical projection and for the transformation between coordinate systems
- sourceEdition: December 2016 edition
- sourceLocator: Section 7.1, the numerical example: Zimmerwald, a Swiss EUREF point
- independent: yes
- inputs: easting 602030.68 m, northing 191775.03 m
- outputs: LV95 2602030.740 m east, 1191775.030 m north
- tolerance: 0.0081 m, the grid's distance from the FINELTRA result swisstopo prints
- verifiedBy: golden vectors v001 to v005 and `core/crates/gp-geo/tests/chenyx06.rs` `swisstopos_five_euref_points_both_ways`
- verifiedOn: 2026-11-21

## Differential tests

- `tools/vectors/gen_chenyx06.py` builds 32 of the 39 vectors in `core/vectors/geodesy.datum.chenyx06.jsonl` from PROJ 9.9.0's hgridshift applying `ch_swisstopo_CHENyx06a.tif`, the same grid the asset is packed from, between EPSG 21781 and EPSG 2056: twenty-two places from Geneva to Davos and Schaffhausen to Lugano, forward with the distortion itself pinned, and ten of them in reverse, all within 0.01 mm. The other seven are swisstopo's five EUREF points and two refusals off the grid.

## Invariants

- `core/crates/gp-grids/tests/grids.rs` `chenyx06_removes_the_swiss_distortion_and_round_trips`: at six places the answer is the round offset plus the reported distortion exactly, the distortion is between 1 cm and 1.7 m (swisstopo's stated ceiling), the reported magnitude is the hypotenuse of its two parts, the reverse lands back on the LV03 point to 0.1 mm, both directions report the same distortion, and points off the grid are refused.
- `core/crates/gp-geo/tests/chenyx06.rs` `swisstopos_five_euref_points_both_ways`: swisstopo's five published points reproduce within 8.1 mm, the distortion at each is metres rather than the round offset, the grid's offsets are arcseconds rather than degrees or radians, and each round trips within 0.1 mm in one to six iterations.
- `core/crates/gp-geo/src/hgrid.rs` `bilinear_inside_none_outside_and_the_inverse_closes`: the generic reader interpolates bilinearly, refuses points off the grid, and its inverse closes.
- `tools/data/chenyx06-assets.test.mjs`: the committed package has the pinned digest and swisstopo's extent, with the 30-arcsecond spacing exact to 1e-16 degrees and both far edges landing on 11°03′E and 48°04′N.
