<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Performance table lookup (`aviation.loading.table-interpolate`)

## Method

The user enters a POH or AFM table of one, two, or three variables as a full grid of cells. For each variable the tool finds the two listed values that bracket the query and the fraction of the way between them. The answer is the sum over the 2, 4, or 8 surrounding cells of each cell's value times the product of its per-variable weights (linear, bilinear, or trilinear interpolation). A query outside any axis is refused, never extrapolated. Corrections are then applied in the order entered, each as its own labeled step. With no table the tool refuses rather than give a generic estimate.

## Equations

- For axis k with bracketing values x_lo ≤ q_k ≤ x_hi: t_k = (q_k − x_lo) / (x_hi − x_lo)
- Each corner c takes x_hi or x_lo on each axis; weight w_c = Π_k (t_k if upper, else 1 − t_k)
- Table value = Σ_c w_c · value(c)  (the weights sum to 1)
- Corrections in order: percent p gives v × (1 + p/100); add a gives v + a; multiply m gives v × m

## Symbols and units

Variables and values are plain numbers in whatever units the table uses (the tool does not convert them); q_k the lookup point; t_k the fraction along axis k.

## Domain

A table of 2 to 2,000 rows; every row gives the same variables; each axis has at least two distinct values; every combination appears exactly once. Each lookup value must lie within its axis's listed range, or the tool returns OUT_OF_DOMAIN naming the axis and its range. Up to 20 corrections.

## Approximations

Straight-line reading between rows. Real performance between table rows is not linear, so the answer is only as good as the table and that assumption. No aircraft data is built in.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C
- sourceLocator: Chapter 11, Figure 11-21, "Interpolating charts"
- independent: yes
- inputs: takeoff ground roll 1,115 ft at 2,000 ft pressure altitude and 1,230 ft at 3,000 ft, both at 20 °C; look up 2,500 ft
- outputs: (1,115 + 1,230) / 2 = 1,172.5 ft, published as 1,173 ft
- tolerance: 0.5 ft, the source's whole-foot rounding
- verifiedBy: golden vector v014
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_table.py`: a separate Python multilinear interpolation over the bracketing cells at eleven two-variable queries, three one-variable queries, two three-variable queries, a chain of percent, multiply, and add corrections, and four refusals (within 1e-9), plus the FAA worked example at its printed precision
- `core/vectors/aviation.loading.table-interpolate.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `table_interpolation_preserves_affine_grids_and_correction_order`: a query beyond the table returns OUT_OF_DOMAIN naming the pressure-altitude axis and its 0 to 4000 range; a bilinear affine grid is reproduced exactly, its four weights sum to 1, changing every cell by the same amount changes the answer by that amount, and percent, multiply, and add corrections run in the stated order
