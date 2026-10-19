## Context

The survey module has plane COGO on robust arithmetic, area by coordinates, Bowditch and transit adjustments, a 2D similarity and affine localization with residuals, a loop-level adjustment, and an experimental 2D least-squares network with a 200-point cap and Cholesky solver. The three tools here reuse that machinery.

## Goals / Non-Goals

**Goals:**
- Textbook results, reproduced to the textbook's printed precision, with the working shown.
- Conventions stated: which side of the cutoff line, which rotation convention, which weights.

**Non-Goals:**
- Curved-boundary partitions. Robust estimation or blunder detection beyond standardized residuals.

## Decisions

### SP1. Area partition as three named cases
Given a closed plane figure (confirmed coordinates, as the deed and traverse tools produce) and a required area A:
1. **Line parallel to a chosen side** at distance d: solve for d where the area between the side and the line equals A. The cut area is a function of d; the tool brackets and solves it with Brent's method to 1e-6 units (per the numeric-determinism contract: declared tolerance and iteration limit), reporting the quadratic coefficients for the trapezoid case so the reader can check by hand.
2. **Line through a fixed point** on the boundary (a corner or a point at a station along a side) rotating until the cut area equals A: the free end's position along the boundary is solved by bracketing.
3. **Line between two chosen sides** with a given bearing: as case 1 but with the bearing fixed.
Each result gives both end points of the cutoff line as coordinates and as station-and-offset on their sides, the line's bearing and length, both resulting areas, and which side of the line is the "cut" piece. The figure must be simple (no self-intersection) and polygonal; a curve call is refused with `DEGENERATE_GEOMETRY` and the reason.

### SP2. 3D conformal fit from control
Given at least 3 common points with coordinates in both systems (ECEF or local 3D), the tool fits the 7 parameters (scale, three rotations, three translations) by least squares: a linearized Bursa-Wolf model iterated from the 2D-similarity initial values, reporting parameters with standard errors, residuals per point and axis, RMS, and the reference variance. The rotation convention is explicit in the result and selectable (position-vector or coordinate-frame, as the geodesy Helmert tools define them), and the fitted parameters can be sent to `geodesy.datum.helmert` to transform other points. Warnings: `NO_REDUNDANCY` at exactly 3 points (21 observations, hmm: 9 observations, 7 unknowns leaves 2 degrees of freedom; the warning fires when degrees of freedom are below 3), `SCALE_SUSPECT` when the scale differs from unity by more than 100 ppm.

### SP3. Leveling network by weighted least squares
Observations are height differences between two benchmarks with a length (weight 1/length) or setup count (weight 1/n), or an explicit standard deviation. Fixed benchmarks hold the datum; at least one is required. The tool builds the observation equations, solves the normal equations by Cholesky, and reports adjusted elevations with standard errors, residuals, standardized residuals, the a-posteriori reference variance, and the chi-square test at 95% (`ADJUSTMENT_TEST_FAILED` when it fails, as the 2D tool does). The result shows the design matrix and normal equations for networks of up to 12 unknowns, so the textbook example is checkable line by line. Cap: 500 unknown benchmarks.

### SP4. Verification
- Partition: Ghilani & Wolf's worked partitioning examples as independent vectors; invariants: the two pieces' areas sum to the whole (1e-9), and the cutoff end points lie on the boundary (1e-9).
- Helmert fit: Ghilani's 3D conformal example to its printed decimals; invariants: transforming the control with the fitted parameters reproduces the residuals; a synthetic data set generated with known parameters is recovered to 1e-9 of the parameter.
- Leveling: Ghilani's leveling-network examples to their printed millimeters; invariant: with all weights equal and one loop, the result equals the existing proportional loop adjustment.

## Tool inventory

| Tool | Endpoints | Reference | Independent vector |
|---|---|---|---|
| area-partition | 1 | Ghilani & Wolf ch. 12 | the chapter's partition examples |
| helmert-3d-fit | 1 | Ghilani, Adjustment Computations (3D conformal); IOGP 7-2 §4.3 | Ghilani's example data set |
| level-network | 1 | Ghilani, Adjustment Computations (leveling) | Ghilani's leveling example |

Total: 3 operations, 3 endpoints. The survey rollup moves from 72 to 75 operations.

## Risks / Trade-offs

- **[The textbook's printed precision is coarse]** → Vectors carry the printed tolerance; the native and Wasm builds are compared before pinning, per the project's practice for iterative tools.
- **[Partition cases with no solution (A larger than the figure)]** → `NO_SOLUTION` naming the figure's total area.

## Migration Plan

Not applicable. New tools start experimental.

## Open Questions

- The exact section numbers of the 16th-edition partitioning material are confirmed at the source in task 1 (the ledger row records chapter 12's subject as unchanged from the 15th).
