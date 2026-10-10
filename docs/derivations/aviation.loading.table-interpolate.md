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

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite, aviation/fuel-and-loading spec, "POH/AFM table interpolation"
- sourceEdition: 2026
- sourceLocator: Scenario "Bilinear interpolation" (table at 0/2,000/4,000 ft and 0/10/20/30 °C, queried at 3,000 ft and 25 °C); the tool's example adds a −10% headwind correction
- independent: no
- inputs: the tool's 12-cell table, pressure altitude 3,000, temperature 25, correction 10 kt headwind −10%
- outputs: cells (2000, 20) 1,325, (2000, 30) 1,420, (4000, 20) 1,540, (4000, 30) 1,655, each weight 0.25; table value 1,485 (computed); with −10%, 1,336.5 (computed)
- tolerance: 1e-9
- verifiedBy: golden vector v001 (table value 1,485; the −10% step is checked inside v002's chain of three corrections)
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_table.py`: a separate Python multilinear interpolation over the bracketing cells at seven two-variable queries, a one-variable and a three-variable table, a chain of percent, multiply, and add corrections, and four refusals (within 1e-9)
- `core/vectors/aviation.loading.table-interpolate.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `a_table_is_never_extrapolated_and_the_refusal_names_the_axis`: a query beyond the table returns OUT_OF_DOMAIN naming the pressure-altitude axis and its 0 to 4000 range, and a query inside uses four cells whose weights sum to 1 within 1e-12
