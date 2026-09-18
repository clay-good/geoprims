## Purpose

Estimates earthwork volumes and expresses grades and slopes correctly, so site engineers and surveyors can quantify cut, fill, and stockpiles and stake slopes in the field.

## ADDED Requirements

### Requirement: Cross-section volumes
Given cross-section areas at stations, the tool SHALL compute volumes by average end area V = L·(A1 + A2)/2 and by the prismoidal formula V = L/6·(A1 + 4·Am + A2). The prismoidal formula SHALL require the measured middle area Am and SHALL reject Am computed as the average of the end areas (which reduces it to average end area), unless the user explicitly selects the prismoidal correction method instead. Cut and fill SHALL be tracked separately, with volumes in m³, ft³, and yd³ (ft³ / 27).

#### Scenario: Average end area
- **WHEN** A1 = 120 ft², A2 = 180 ft², L = 100 ft
- **THEN** V = 15,000 ft³ ≈ 555.56 yd³

#### Scenario: Prismoidal with measured middle area
- **WHEN** Am = 148 ft² is measured at the midpoint
- **THEN** V ≈ 14,866.7 ft³, and the difference from average end area is reported

#### Scenario: Middle area not measured
- **WHEN** a user enters Am equal to the mean of A1 and A2
- **THEN** the result warns `PRISMOIDAL_MIDDLE_AREA_AVERAGED`, explaining that the prismoidal formula then equals average end area

### Requirement: Cross-section area from points
The tool SHALL compute cut and fill areas from a cross-section defined by existing-ground and design-template points (offset, elevation), including daylight (catch) points where the template meets the ground.

#### Scenario: Mixed section
- **WHEN** a section is partly in cut and partly in fill
- **THEN** cut area and fill area are computed separately, and the grade point is located

### Requirement: Grid (borrow-pit) method
Given a grid of existing and proposed elevations (a regular grid with cell size), the tool SHALL compute cut and fill volumes by the borrow-pit method (corner weights 1, 2, 3, 4) and by the four-point average method, and SHALL report the balance line (zero-cut/fill contour) on the canvas.

#### Scenario: Corner weights
- **WHEN** a 3 × 3-node grid is processed
- **THEN** corner nodes use weight 1, edge nodes weight 2, and the interior node weight 4, and the volume equals Σ(weight × depth) × cell area / 4

### Requirement: Shrink and swell
Tools SHALL convert between bank, loose, and compacted volumes with user-entered swell and shrink factors (with labeled typical values by soil type as reference only), and compute net import/export.

#### Scenario: Swell to truck loads
- **WHEN** 1,000 bank yd³ with 25% swell is hauled in 12 yd³ trucks
- **THEN** loose volume is 1,250 yd³, requiring 105 truck loads (rounded up)

### Requirement: Grade and slope expressions
Tools SHALL convert between percent grade, ratio (H:V and V:H, labeled explicitly), degrees, and per mille, and compute rise, run, or slope length from any two. Ambiguous ratio strings (e.g. `3:1`) SHALL require the user to confirm H:V vs V:H.

#### Scenario: Ambiguous ratio
- **WHEN** a user enters `3:1` without choosing a convention
- **THEN** the tool asks whether it means 3H:1V (≈ 18.43°, 33.3%) or 3V:1H (≈ 71.57°, 300%), showing both

### Requirement: Slope staking
Given a design template (subgrade width, side slopes for cut and fill) and existing ground (a profile across the centerline or rod readings), the tool SHALL compute catch point offsets and cut/fill at the catch point by iteration, converging to 0.01 of the unit. It SHALL report the stake notation (e.g. `C 4.2 / 28.6 L`).

#### Scenario: Catch point convergence
- **WHEN** existing ground slopes across the template
- **THEN** the catch point offset and cut/fill converge within 0.01 ft (or m) and are reported in stake notation

### Requirement: Stockpile and simple solid volumes
Tools SHALL compute stockpile volume from a base polygon and surface points (TIN between base and surface), and simple solids (cone, frustum, prism) from user-entered dimensions or side-slope angles. No material angle-of-repose table SHALL be provided, because published values vary widely and are not design values; the user enters the angle and its source.

#### Scenario: TIN stockpile
- **WHEN** a base polygon and 500 surface points are provided
- **THEN** the volume above the base plane is computed from a Delaunay TIN, and the surface is shown in 3D

### Requirement: Profile slope analysis
Given an elevation profile (user points or a DEM profile from raster tools), the tool SHALL compute segment grades, maximum and average grade, cumulative climb and descent, and flag segments exceeding a user threshold.

#### Scenario: Grade threshold
- **WHEN** the maximum grade threshold is 8% and a segment is 11%
- **THEN** the segment is flagged and highlighted on the profile chart
