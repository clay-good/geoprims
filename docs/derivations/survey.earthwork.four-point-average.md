<!-- Derivation note (trust/correctness-program layer A). -->
# Four-point average cut and fill (`survey.earthwork.four-point-average`)

## Method

A site is covered by a square grid with an existing and a proposed elevation at each grid point. The depth at a point is the existing less the proposed elevation: positive is cut, negative is fill. Each cell's volume is its area times the average of its four corner depths. A cell with both cut and fill corners is split between the two in proportion to the depths on each side. The operation shares one calculation path with `survey.earthwork.borrow-pit`, which leads with the corner-weights net.

## Equations

For a cell of area A with corner depths h₁ to h₄, P the sum of its positive depths and N the sum of the magnitudes of its negative depths:

- All cut: volume = A × P / 4. All fill: volume = A × N / 4.
- Transition cell: cut = A × P² / (4(P + N)), fill = A × N² / (4(P + N)).
- Net = total cut − total fill, which equals Σ(weight × depth) × A / 4 with corner weights 1, 2, and 4.

## Symbols and units

Elevations and the cell size share one length unit. A grid in feet of either kind reports cubic feet, and a grid in meters cubic meters; the net is also given in cubic yards. Balance points are distances east and south of the grid's first point.

## Domain

At least two rows of at least two points, every row the same length, a positive cell size, and either one finished grade or a proposed grid of the same shape.

## Approximations

The ground is taken as straight between grid points. The transition-cell split is the usual proportional rule, not an exact integration of a twisted surface. Volumes are bank measure.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/earthwork-and-grade, requirement "Grid (borrow-pit) method" and scenario "Corner weights"; the arithmetic is checked by hand, not taken from a published table
- independent: no
- inputs: existing 10, 11 / 9, 12 against a level 10.5 on a 20 m cell
- outputs: depths −0.5, +0.5, −1.5, +1.5 give P = 2 and N = 2, so cut = 400 × 4 / 16 = 100 m³ and fill = 100 m³, net 0
- tolerance: 1e-9 m³
- verifiedBy: the `named_earthwork_operations_match_their_general_tools` integration test, which also swaps the two surfaces
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `four_point_average`: 19 random grids of 2 to 6 rows and 2 to 7 columns in all three length units, against a level grade or a proposed grid, by a separate Python loop over the cells.
- `core/vectors/survey.earthwork.four-point-average.jsonl`: those vectors, the page's example, and three refusal cases.
- The integration suite requires the same cut, fill, and balance points as `survey.earthwork.borrow-pit`, and a net within 1e-6 of its corner-weights net.

## Invariants

- Net = cut − fill.
- Swapping the existing and proposed surfaces swaps cut and fill.
- Raising every proposed elevation by the same amount cannot increase the cut.
