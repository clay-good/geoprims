## Purpose

Cuts a parcel to a required area, fits a 3D transformation between two control sets, and adjusts a leveling network, each with the working shown and a textbook example reproduced.

## ADDED Requirements

### Requirement: Area partition by a cutoff line
Given a simple closed plane figure and a required area, the partition tool SHALL find the cutoff line for one of three named cases: parallel to a chosen side, through a fixed point on the boundary, or between two chosen sides at a given bearing. It SHALL return both end points (coordinates and station-and-offset on their sides), the line's bearing and length, both resulting areas, and which piece is the cut. The solution SHALL converge to 1e-6 of the linear unit within a declared iteration limit, and the pieces' areas SHALL sum to the figure's area within 1e-9 of the unit squared.

#### Scenario: Parallel cut of a trapezoid
- **WHEN** a trapezoid of the textbook example is cut parallel to its base to take off the example's required area
- **THEN** the distance from the base and the cutoff length match the textbook's values to its printed precision, and the areas sum to the whole

#### Scenario: Required area too large
- **WHEN** the required area exceeds the figure's area
- **THEN** the result is `NO_SOLUTION` naming the figure's total area

#### Scenario: Curved side refused
- **WHEN** the figure includes a curve call
- **THEN** the result is `DEGENERATE_GEOMETRY` stating that partition handles straight sides only

### Requirement: 3D conformal fit from common points
Given at least 3 points with coordinates in two 3D systems, the tool SHALL fit a 7-parameter conformal transformation (scale, three rotations, three translations) by least squares, reporting parameters with standard errors, the rotation convention (position-vector or coordinate-frame, explicit), residuals per point and axis (mm), RMS, and the reference variance. It SHALL warn `NO_REDUNDANCY` below 3 degrees of freedom and `SCALE_SUSPECT` when scale differs from unity by more than 100 ppm. The fitted parameters SHALL be sendable to the generic Helmert tool.

#### Scenario: Textbook data set
- **WHEN** the adjustment textbook's 3D conformal example points are fitted
- **THEN** the parameters and residuals match the book to its printed decimals

#### Scenario: Known parameters recovered
- **WHEN** 10 synthetic points are transformed with known parameters and fitted back
- **THEN** every parameter is recovered within 1e-9 of its value and every residual is below 1e-9 m

### Requirement: Leveling network adjustment
Given height-difference observations between benchmarks, each with a length, setup count, or standard deviation, and at least one fixed benchmark, the tool SHALL adjust the network by weighted least squares and report adjusted elevations with standard errors (m), residuals, standardized residuals, the a-posteriori reference variance, and a chi-square test at 95%, warning `ADJUSTMENT_TEST_FAILED` when it fails. For networks of at most 12 unknowns it SHALL show the observation equations and normal equations. It SHALL accept at most 500 unknown benchmarks.

#### Scenario: Textbook network
- **WHEN** the adjustment textbook's leveling-network example is adjusted with its weights
- **THEN** the adjusted elevations match the book to the millimeter and the residuals to its printed precision

#### Scenario: One loop equals the loop adjustment
- **WHEN** a single closed loop with equal weights is adjusted
- **THEN** the elevations equal the existing level-loop proportional adjustment within 1e-9 m

#### Scenario: No fixed benchmark
- **WHEN** no benchmark is held fixed
- **THEN** the result is `INVALID_INPUT` for `/fixed` explaining that the datum is undefined

### Requirement: Calculation sheets
Each of the three tools SHALL export a calculation sheet carrying the inputs, the equations with the reader's numbers, the result, the method and its citation, and the professional-use notice.

#### Scenario: Sheet exported
- **WHEN** a leveling network result is exported as a calculation sheet
- **THEN** the sheet lists every observation with its weight and residual, the adjusted elevations, and the citation
