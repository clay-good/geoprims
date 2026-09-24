<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Contour lines from a grid of elevations (`raster.terrain.contours`)

## Method

A grid of elevations only says what the ground is at the grid points. To draw a contour, the tool assumes the ground changes steadily along each grid line between two neighbouring points, which is the least it can assume, and then looks at one grid square at a time (marching squares, the flat case of Lorensen and Cline's marching cubes).

For each level, a square's corners are either above the level or not. A contour crosses a side of the square wherever one end of that side is above and the other is not, at the point where the steady change along the side reaches the level. The crossings in a square pair up into straight pieces. Usually there are two crossings and one piece. When there are four (two high corners facing each other across the square, a saddle), the grid alone cannot say whether the high corners join through the middle or the low ones do, so the tool follows the convention contourpy and Matplotlib use: the mean of the four corners stands for the middle.

Each piece is pointed so that higher ground is on its left. Because a crossing is computed once per grid line, from the same end each time, neighbouring squares produce exactly the same point, and the pieces join end to start into lines. A line that comes back to its start is a closed loop; one that runs off the grid or into a gap is open. Where a line passes twice through the same point (two contours touching at a grid point that sits exactly on the level), the loop between the two visits is split off, so a closed contour is never folded into another line.

## Equations

- Levels: L = base + k × interval for every whole k with lowest ≤ L ≤ highest, over the elevations that are not no-data.
- Above: a corner is above level L when z > L; a corner exactly at L counts as not above, as in contourpy.
- Crossing on the grid line from a to b: t = (L − z_a) / (z_b − z_a), the point a + t (b − a), with t taken from the north or west end.
- Saddle: when all four sides are crossed, the high corners join through the middle when (z₁ + z₂ + z₃ + z₄) / 4 > L.
- Direction: each piece p → q has the higher corners on its left.
- Length: the sum of the straight pieces, in the cell-size unit.

## Symbols and units

`elevations` is rows of numbers in meters, the north row first and each row west to east; `cell_size` is the ground distance between neighbouring grid points; `interval` and `base` set the levels; `no_data` marks a missing elevation. Outputs are in meters: `contours` gives each line's points in order as distances east and south of the north-west grid point, `levels` gives the lines, closed loops, and length at each elevation, and `line_count`, `level_count`, `length`, `lowest`, and `highest` summarize.

## Domain

A regular grid with square cells, 2 to 300 points a side, every row the same length. Up to 500 levels and 100,000 contour vertices. A square with a no-data corner is skipped, so lines stop at gaps. The grid is not georeferenced: positions are distances from its north-west point.

## Approximations

Between grid points the ground is taken to change linearly along each grid line, so the contours are exact for that surface and for no other; a feature smaller than the grid spacing cannot appear. The saddle rule is a convention, since four corners do not decide which way a saddle joins. Exact-level touches are a convention too (the split rule above).

## Worked example

- sourcePublisher: the contourpy developers (the contouring engine behind Matplotlib)
- sourceTitle: contourpy
- sourceEdition: contourpy 1.3.0, numpy 1.26.4
- sourceLocator: `contour_generator(z, name="serial", corner_mask=False, line_type="Separate").lines(level)`
- independent: yes
- inputs: 250 random grids of 4 to 20 points a side made of smooth hills and pits, 76 with no-data holes, 40% rounded to whole meters with whole-meter intervals so grid points often sit exactly on a level; and the hand-made grids in the golden vectors (a hill, a pit, a saddle, the two-point checkerboard both ways, a plane, and a hill with a hole)
- outputs: for every level, the number of lines, closed loops, vertices, and total length
- tolerance: exact on the counts, 1e-9 relative on lengths
- verifiedBy: `core/crates/gp-raster/tests/contours_parity.rs` over all 250 grids; golden vectors v001 to v022
- verifiedOn: 2026-09-24

contourpy is written separately in C++ and traces lines its own way, with its own data structures; the two share only the conventions above, which contourpy documents and this tool follows on purpose. On every level of every grid they find the same lines. Three differences of presentation are normalized in the reference before comparing, and the generator prints how often each applies: contourpy returns a one-point "line" where a corner sits exactly on the level with nothing lower around it, and the tool draws nothing there; contourpy repeats a point where a line passes exactly through a grid point, and the tool does not; and where lines touch at a grid point exactly on the level, contourpy sometimes restarts a line there and sometimes runs through, depending on where its trace began (22 of 5,328 levels at generation), so its lines are joined end to start and split at revisited points by the tool's own rule.

The fixture was checked for teeth: counting a corner exactly at the level as above fails 517 levels; deciding saddles the other way fails 9; and leaving pieces unoriented fails 8,793.

## Differential tests

- `tools/vectors/gen_contours.py`: the contourpy fixture and the 25 golden vectors (22 from contourpy, 3 errors)
- `core/crates/gp-raster/tests/contours_parity.rs` `contours_match_contourpy`: every level of all 250 grids

## Invariants

- `core/crates/gp-raster/tests/contours.rs` `contour_invariants`: on four bumpy surfaces, every vertex sits on a grid line where the elevation interpolated along that line equals its level to 1e-9; at both ends of every step the higher grid point of the line it crosses is on its left; raising the ground and the base by the same amount changes nothing; doubling the cell size doubles every length and keeps every line; turning the ground upside down keeps every line and its length; and a single hill gives exactly one closed loop per level, each shorter than the one below it.
