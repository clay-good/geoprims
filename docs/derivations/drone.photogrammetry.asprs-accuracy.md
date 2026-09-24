<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ASPRS accuracy, Edition 2 (`drone.photogrammetry.asprs-accuracy`)

## Method

A mapping product is checked by comparing it with checkpoints: places surveyed independently, more accurately than the product. The difference at each checkpoint is an error, and the ASPRS standard (Edition 2) states accuracy as the root mean square of those errors, the RMSE, and nothing else. The 95% figures of earlier standards are gone.

The checkpoints are not perfect either, and Edition 2 lets them be only twice as accurate as the product, so their own error is no longer small enough to ignore. The tool adds it in quadrature: the product's accuracy is the square root of the fit's RMSE squared plus the checkpoints' RMSE squared (section 7.11). Horizontally this is done per axis, since the standard assumes the checkpoints' easting and northing errors are equal.

Given the checkpoint errors themselves, the tool computes the RMSEs and applies the rest of the standard. Heights on open ground (bare soil, short grass, pavement) give the non-vegetated vertical accuracy (NVA), which must meet its class. Heights in vegetation give the vegetated vertical accuracy (VVA), which is only reported. Any error over three times the target RMSE in a component is a blunder to investigate, and a mean error of 25% of the target or more suggests a systematic error (section 7.2). An accuracy statement needs at least 30 checkpoints in each test (7.13) and checkpoints at least twice as accurate as the product (7.12).

## Equations

- RMSE of n errors e: √(Σe² / n), per component.
- RMSE_H = √(RMSE_x² + RMSE_y²).
- Product accuracy, per component: √(RMSE_fit² + RMSE_checkpoint²); so RMSE_V = √(RMSE_V1² + RMSE_V2²), and RMSE_H = √((RMSE_x² + c²) + (RMSE_y² + c²)) = √(RMSE_H1² + RMSE_H2²) with RMSE_H2 = √2 c for a per-axis checkpoint RMSE c.
- NVA meets a class X when RMSE_V (open ground) ≤ X; the horizontal class X when RMSE_H ≤ X.
- Per-axis horizontal target: X / √2.
- Blunder: |e| > 3 × the component's target RMSE (vegetated heights are not tested).
- Mean error flag: |mean| ≥ 0.25 × the component's target RMSE (heights on open ground only).
- Checkpoint accuracy flag: c > (smallest per-component target) / 2.

## Symbols and units

Lengths are in centimeters by default and accept any length unit. `rmse_x`, `rmse_y`, `rmse_z` are the fit to checkpoints; `checkpoint_rmse` is the checkpoint survey's RMSE per axis; `checkpoints` is their number; `errors` lists each checkpoint's easting, northing, and height error (product minus checkpoint) and its ground cover; `target_horizontal` and `target_vertical` are the accuracy classes. Outputs: `horizontal` (RMSE_H) and `vertical` (RMSE_V, the NVA when errors are listed) with checkpoint error included, `vva`, the class verdicts, the mean errors, the blunders with their limits, and `checkpoint_status`.

## Domain

Any number of fit RMSEs, or up to 10,000 checkpoint errors, but not both at once, since the RMSEs are computed from the errors. A checkpoint may carry horizontal errors, a height error, or both. Classes must be positive and the checkpoint RMSE not negative.

## Approximations

None in the arithmetic, which is the standard's. Two readings are made where the text is not explicit, and both are stated in the tool's limitations: the checkpoint RMSE is per axis, as the standard assumes when it combines easting and northing; and the blunder and mean-error tests for easting and northing use the horizontal class divided by √2, since the standard states horizontal classes as RMSE_H but tests blunders and bias per component.

## Worked example

- sourcePublisher: American Society for Photogrammetry and Remote Sensing
- sourceTitle: ASPRS Positional Accuracy Standards for Digital Geospatial Data, Edition 2
- sourceEdition: Edition 2, Version 1.0 (February 2023)
- sourceLocator: Section 7.11.4, Table 7.4, Computing Vertical Product Accuracy
- independent: yes
- inputs: the fit to checkpoints RMSE_V1 from 1.00 to 10.00 cm in steps of 0.50 cm, with survey checkpoint accuracy RMSE_V2 = 2.0 cm
- outputs: the vertical product accuracy RMSE_V for each row, printed to 0.01 cm (2.24 cm for the first row)
- tolerance: 0.005 cm, the table's printed precision
- verifiedBy: golden vectors v008 to v026, all 19 rows
- verifiedOn: 2026-09-24

The table is the standard's own worked example, printed in the standard, so it checks the tool against the authority rather than against a second reading of it. Every row agrees to the printed hundredth. The first row is also the spec's "checkpoint error included" scenario and the tool's primary example.

The standard was read in Version 1.0 from the copy the Missouri surveyors' association distributes, because ASPRS's own links to Version 2.0 (approved in 2024) now end at a member sign-in. Whether Version 2.0 changed any of the sections used here has not been checked; the sources ledger records which version was read.

## Differential tests

- `tools/vectors/gen_asprs.py`: Table 7.4 as printed, and checkpoint lists whose statistics are recomputed in Python from the standard's definitions (a passing set, vegetated checkpoints for VVA, a height and an easting blunder, a biased set, coarse checkpoints, a class not met, and two input errors)
- `core/crates/gp-drone/tests/drone.rs` `asprs_scenarios`: the checkpoint-error and too-few-checkpoints scenarios

## Invariants

- `core/crates/gp-drone/tests/asprs.rs` `asprs_invariants`: on six sets of 40 checkpoints, a list of errors gives the same accuracy as its own RMSEs typed in; product accuracy is never better than the fit or the checkpoints and equals the fit with perfect checkpoints; the order of the checkpoints does not matter and flipping every error's sign changes only the signs of the means; a class is met exactly when the accuracy is at or under it; vegetated heights change the VVA but not the NVA and are never called blunders; a height error of exactly three times the target is not a blunder and one just past it is, named by its row; and 20 checkpoints raise INSUFFICIENT_CHECKPOINTS.
