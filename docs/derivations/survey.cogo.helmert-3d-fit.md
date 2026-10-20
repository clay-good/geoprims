<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# 3D conformal (Helmert) fit (`survey.cogo.helmert-3d-fit`)

## Method

Least squares for the seven parameters of the linearized Helmert (Bursa-Wolf) transformation: three translations, three small rotations, and a scale change. It uses the model of IOGP Guidance Note 7-2 §4.2.3, which `geodesy.datum.helmert` applies, so the fitted parameters can be sent to that tool unchanged.

- Coordinates are first reduced to their centroids, so the normal equations stay well conditioned at ECEF magnitudes.
- Gauss-Newton starts from zero and runs to a 1e-15 change.
- The translations and their covariance are then carried back to the uncentered origin.
- The rotation convention, position-vector or coordinate-frame, is chosen and reported.

## Equations

- V_T = T + (1 + dS) · (I + Ω) · V_S, where Ω is the skew matrix of the small rotations (rx, ry, rz). Its signs are reversed for the coordinate-frame convention.
- Residuals: v = V_T(observed) − V_T(fitted), per point and axis.
- σ0² = vᵀv / (3n − 7), and standard errors are √(σ0² N⁻¹) on the diagonal.
- RMS = √(vᵀv / 3n).

## Symbols and units

- X, Y, Z: in m, in both systems.
- Translations: in m.
- Rotations: in arc-seconds.
- Scale: in ppm.
- Residuals and RMS: in mm.

## Domain

- At least 3 common points; at most 500.
- Rotations must be small, as in datum transformations: well under a degree.
- `NO_REDUNDANCY` warns below 3 degrees of freedom.
- `SCALE_SUSPECT` warns when the scale differs from one by more than 100 ppm.
- Points that are all on one line leave the rotations undefined and are refused.

## Approximations

The rotation matrix is the small-angle form, as in the Helmert tool, so the fit and the transform agree exactly. All points weigh equally.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: Five common points 2 km apart, a sub-meter shift
- sourceEdition: 2026-10-20
- sourceLocator: The tool's primary example. Points transformed by known parameters are fitted back to those parameters.
- independent: no (the 3D conformal example in Ghilani, Adjustment Computations, is not open and is not yet reproduced)
- inputs: five ECEF points and their transformed coordinates
- outputs: the generating parameters, with residuals at the noise level
- tolerance: 1e-5 m, arc-second, and ppm at ECEF magnitudes; 1e-9 in local coordinates
- verifiedBy: `helmert_3d_fit_invariants` and the golden vectors
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_helmert_fit.py`: target points made in Python from source points with known parameters by the same linearized model, at ECEF and local magnitudes, with both rotation conventions. The fit must recover the parameters that made them.
- `core/vectors/survey.cogo.helmert-3d-fit.jsonl`: 26 vectors from it, run through the core on every build.

## Invariants

- `core/crates/gp-survey/tests/helmert.rs` `helmert_3d_fit_invariants`:
  - 10 synthetic local points made with known parameters, in both conventions, come back within 1e-9, with residuals below 1e-9 m.
  - The test's transform is written separately from the core, following the IOGP form that `geodesy.datum.helmert` applies.
- Precision: at ECEF magnitudes with points 2 km apart, double precision fixes the rotations only to about 1e-7 arc-seconds, so translations come back to micrometers.
