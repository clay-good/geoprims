<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Leveling network adjustment (`survey.reduction.level-network`)

## Method

Weighted least squares on observation equations, one equation per leveling run.

- Each unknown benchmark is an unknown. Fixed benchmarks move to the right-hand side.
- The normal equations are solved by Cholesky factorization.
- Standard errors come from the inverse of the normal matrix, scaled by the reference variance. With a standard deviation on every run and no redundancy, the scale is the a-priori variance of 1.
- With a standard deviation on every run, a two-tailed chi-square test at 95% checks the weighted residual sum.

## Equations

- H_to − H_from = Δh + v, with weight w = 1/σ², else 1/length (km), else 1/setups, else 1.
- N = AᵀPA, t = AᵀPl, and N x = t, solved by Cholesky.
- v = A x − l, and σ0² = vᵀPv / r, where r = runs − unknown benchmarks.
- Standard error of a benchmark: √(σ0² Q_xx), where Q_xx = N⁻¹.
- Standardized residual: v / (σ0 √Q_vv), where Q_vv = 1/w − a Q_xx aᵀ.
- Test: χ²(0.025, r) ≤ vᵀPv ≤ χ²(0.975, r).

## Symbols and units

- H: elevation, entered in any length unit and reported in m.
- Δh: observed rise, to minus from.
- σ: standard deviation of a run.
- Length: in km; setups is a count.
- Results: elevations to 0.1 mm in display, with standard errors and residuals in mm. The reference variance has no unit when standard deviations are given.

## Domain

- At least one fixed benchmark, and at most 500 unknown benchmarks and 2,000 runs.
- Every unknown benchmark must connect to a fixed one through the runs, or the result is `NO_SOLUTION`.
- A net where every benchmark is fixed is refused with `INVALID_INPUT`.

## Approximations

The height differences are adjusted as given: no rod, curvature and refraction, or orthometric corrections. Weights by length or setups are relative only, so the test needs a standard deviation on every run.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: One run between two fixed benchmarks equals the proportional adjustment
- sourceEdition: 2026-10-20
- sourceLocator: BM A 250.000 m to BM B 247.975 m through TP 1 and TP 2, lengths 120, 95, 150 m. The least-squares elevations equal those of `survey.reduction.level-adjust`, which spreads the misclosure in proportion to distance.
- independent: no (the leveling-network examples in Ghilani, Adjustment Computations, are not open and are not yet reproduced)
- inputs: the three runs, weighted by length
- outputs: the same turning-point elevations as the level-adjust tool
- tolerance: 6e-5 m (the level-adjust display precision)
- verifiedBy: `level_network_invariants`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_level_network.py`: NumPy's `lstsq` on the weighted design matrix, separate from the core's Cholesky. It covers 18 random networks with loops, weighted by length, setups, or standard deviation, with one or two fixed benchmarks. Elevations agree to the displayed 0.1 mm, and the reference variance within 1e-6 relative.
- `core/vectors/survey.reduction.level-network.jsonl`: those vectors plus the disconnected-network and all-fixed refusals, run through the core on every build.

## Invariants

- `core/crates/gp-survey/tests/levelnet.rs` `level_network_invariants`:
  - a single run weighted by length equals the proportional level-adjust result;
  - an open run has 0 degrees of freedom;
  - the chi-square test passes on clean data and fails with `ADJUSTMENT_TEST_FAILED` on a 10 cm blunder.
