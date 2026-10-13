<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Site localization (`survey.gnss.localization`)

## Method

The tool fits coordinates observed in a local site system to coordinates for the same control points in a grid system. A similarity fit has one scale and one rotation, so it preserves angles and shapes. An affine fit allows separate coefficients in each direction, including shear and unequal scale. Both are ordinary least-squares fits over every entered control pair.

The calculation first subtracts the coordinate centroids. Centering makes the fitted coefficients depend on the control network's size, not on the size of its false easting or northing. The shifts are recovered after the coefficients are fit. A compact network at large coordinates therefore gets the same scale, rotation, and residuals as the same network near zero, within floating-point roundoff.

## Equations

For each point with local easting `e` and northing `n`, and grid easting `E` and northing `N`:

- similarity: `E = a e - c n + tE`; `N = c e + a n + tN`
- similarity scale: `s = sqrt(a² + c²)`
- similarity rotation: `theta = atan2(c, a)`
- affine: `E = a e + b n + tE`; `N = c e + d n + tN`
- affine representative scale: `s = sqrt(|ad - bc|)`, the square root of the area scale
- residuals: `dE = Eobserved - Efitted`; `dN = Nobserved - Nfitted`
- horizontal residual: `r = sqrt(dE² + dN²)`
- root-mean-square residual: `RMS = sqrt(sum(r²) / point count)`

The similarity coefficients come from the centered normal equations in closed form. The affine coefficients come from the inverse of the centered 2 by 2 local-coordinate covariance matrix. These are algebraically equivalent to a full least-squares design matrix.

## Symbols and units

`e`, `n`, `E`, `N`, `tE`, `tN`, the residuals, and RMS are lengths. Every entered coordinate is converted independently to meters before fitting, so correctly labeled mixed length units are valid. `a`, `b`, `c`, `d`, and `s` are unitless. `theta` is reported in degrees, counterclockwise for the point-vector rotation from local coordinates to grid coordinates. Scale offset is `(s - 1) × 1,000,000` in parts per million.

## Domain

A similarity fit needs at least 2 distinct local points. An affine fit needs at least 3 local points that do not lie on one line. Up to 500 pairs are accepted. A fit at its minimum point count raises `NO_REDUNDANCY`, because its zero residuals provide no independent check. A supplied expected combined factor must be positive. A fitted scale more than 100 ppm from that expected value raises `SCALE_SUSPECT`; without an expected value, a difference greater than 1% from 1 raises it. A scale near the foot-to-meter ratio calls out a likely unit mix-up.

## Approximations

The least-squares arithmetic is exact apart from binary floating-point rounding. The result is a horizontal site fit, not a datum transformation model, geoid model, or height localization. Residuals reveal disagreement among redundant controls but cannot identify which control is wrong. The affine scale is an area-equivalent summary; inspect all four coefficients when shear or unequal directional scale matters. Extrapolation beyond the control network is not checked and can be unreliable.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers
- sourceTitle: IOGP Guidance Note 7-2: Coordinate Conversions and Transformations
- sourceEdition: Revised September 2019
- sourceLocator: Section 2.4.3.2, Similarity transformation, method code 9621, worked example
- independent: yes
- inputs: four local control points beginning with the source point E = 300,000 m and N = 4,500,000 m; their grid coordinates are generated with the source's shifts, scale, and rotation
- outputs: scale 1.00000155 and a point-vector rotation of -1.56504 arcseconds (-0.000434733 degrees)
- tolerance: 0.001 ppm in scale and 0.001 arcsecond in rotation; the source's first transformed coordinate is reproduced within 0.002 m from its printed parameters
- verifiedBy: golden vector v018 and `core/crates/gp-survey/tests/localize.rs`, run by the core on every build
- verifiedOn: 2026-10-13

IOGP defines the published positive angle as a rotation of the source axes. The tool reports the equivalent rotation of point vectors, so its angle has the opposite sign. Applying the printed parameters to the source's first point gives E = 299,905.060 m and N = 4,499,796.514 m; the source prints E = 299,905.060 m and N = 4,499,796.515 m.

## Differential tests

- `tools/vectors/gen_localize.py`: 24 live vectors, with 19 successful fits compared with NumPy's full design-matrix `lstsq` rather than the core's closed form; the vector file also preserves 3 superseded warning-index records for regression history
- coverage includes similarity and genuine affine fits, exact and noisy controls, rotations near 180 degrees, shear, unequal scale, reflection, metric, feet, mixed units, minimum point counts, a large coordinate origin, suspect scale, and invalid geometry or expected scale
- vector v018 uses IOGP's published method 9621 parameters; NumPy independently recovers the scale, rotation, shifts, and matrix coefficients from the rounded control values

## Invariants

- `core/crates/gp-survey/tests/localize.rs` `similarity_is_invariant_to_point_order_and_a_large_coordinate_origin`: point order and a 1,000,000,000 m coordinate translation do not change scale, rotation, or the residual-centroid property
- `core/crates/gp-survey/tests/localize.rs` `affine_fit_recovers_shear_and_nonuniform_scale`: an exact affine network recovers all four coefficients and has zero RMS within roundoff
- `core/crates/gp-survey/tests/localize.rs` `iogp_method_9621_parameters_are_recovered_with_the_documented_sign`: the IOGP scale, shifts, and point-vector rotation are recovered with the documented sign convention
