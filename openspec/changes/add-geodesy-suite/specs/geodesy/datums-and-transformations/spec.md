## Purpose

Moves coordinates between datums and reference frames correctly, which means with explicit realizations and epochs and with an honest accuracy statement. This ends the silent "WGS84 = NAD83" meter-level errors common in field and drone work.

## ADDED Requirements

### Requirement: Frames are named precisely
Transformation tools SHALL identify source and target frames by realization, not family name. Supported frames SHALL include at least: WGS 84 (G730, G873, G1150, G1674, G1762, G2139, G2296), ITRF2008, ITRF2014, ITRF2020, IGS20, NAD83(2011), NAD83(CSRS) v8, NAD83(HARN), NAD83(1986), NAD27, ETRS89 (ETRF2000 and ETRF2020 realizations), GDA94, GDA2020, and NATRF2022 (beta, per data-assets). When a user selects "WGS 84" without a realization, the tool SHALL use G2296 and add warning `REALIZATION_ASSUMED`.

#### Scenario: Unqualified WGS 84
- **WHEN** a user transforms from "WGS 84" to NAD83(2011) without choosing a realization
- **THEN** the tool uses WGS 84 (G2296), and the result includes `REALIZATION_ASSUMED`

### Requirement: Epochs are explicit for dynamic frames
Any transformation involving a time-dependent frame (ITRF, IGS, WGS 84 realizations since G730, NATRF2022) SHALL take an observation epoch (decimal year or ISO date) and, where the target is a plate-fixed frame, SHALL state the target reference epoch (e.g. NAD83(2011) epoch 2010.00). The default epoch SHALL be the current date, echoed in the result per the tool contract.

#### Scenario: Epoch echoed
- **WHEN** an ITRF2020 → NAD83(2011) transformation is run without an epoch
- **THEN** the result states the epoch used (today as a decimal year) and the NAD83(2011) reference epoch 2010.00

### Requirement: Time-dependent Helmert transformations
The domain SHALL implement 7-parameter and 14-parameter (time-dependent) Helmert transformations in both the position-vector and coordinate-frame conventions, with the convention explicit in every parameter set, and SHALL ship the published parameter sets for the supported frame pairs (IERS/ITRF, NGS, NGA, and EPSG sources), each with its source citation and stated accuracy. A generic Helmert tool SHALL accept user-supplied parameters with an explicit convention.

#### Scenario: Convention required
- **WHEN** a user supplies custom Helmert rotations without choosing position-vector or coordinate-frame convention
- **THEN** the tool returns `INVALID_INPUT` for `/convention`, explaining that the two conventions differ in rotation sign

#### Scenario: ITRF2020 to NAD83(2011) matches NGS
- **WHEN** the NGS HTDP sample points are transformed from ITRF2020 to NAD83(2011) at their published epochs
- **THEN** results agree with HTDP within 1 mm per component

### Requirement: Plate motion and velocity propagation
The domain SHALL propagate coordinates between epochs within a frame using the ITRF2020 plate motion model (rigid-plate velocities) and SHALL state that deformation zones (e.g. western US plate boundary) are not modeled, adding warning `DEFORMATION_ZONE` when the point lies in a zone flagged by the model. A user-supplied site velocity SHALL override the model.

#### Scenario: Propagation in a deformation zone
- **WHEN** a point near the San Andreas fault is propagated from epoch 2010.0 to 2026.7 in ITRF2020
- **THEN** the result includes `DEFORMATION_ZONE` and recommends a site velocity from NGS

### Requirement: Grid-based datum transformations
The domain SHALL implement NADCON5 grid transformations (NAD27 ↔ NAD83(1986) ↔ NAD83(HARN) ↔ NAD83(FBN) ↔ NAD83(2007) ↔ NAD83(2011) across CONUS, Alaska, Hawaii, Puerto Rico/Virgin Islands, and other published regions), including the chained path where required, with NGS-published accuracy estimates per point. Points outside grid coverage SHALL return `OUT_OF_DOMAIN`.

#### Scenario: NAD27 to NAD83(2011)
- **WHEN** a CONUS NAD27 point is converted to NAD83(2011)
- **THEN** the result matches NGS NCAT output within 1 mm per component, reports the chain of grids used, and reports NADCON5's accuracy estimate for that point

#### Scenario: Outside grid
- **WHEN** a NAD27 point in Mexico beyond NADCON5 coverage is transformed
- **THEN** the result is `OUT_OF_DOMAIN` naming the grid coverage

### Requirement: Transformation accuracy is always stated
Every transformation result SHALL include the combined accuracy of the transformation path (parameter or grid accuracy plus any realization equivalence assumed). Where two frames are treated as coincident (e.g. WGS 84 (G2296) and ITRF2020), the result SHALL state the assumed agreement (a few centimeters) rather than claim exactness.

#### Scenario: Coincidence stated
- **WHEN** WGS 84 (G2296) is converted to ITRF2020
- **THEN** the coordinates are unchanged and `meta.accuracy` states that the frames agree at the few-centimeter level by NGA's alignment

### Requirement: Magnitude explainer for the WGS 84 vs NAD83 difference
A tool SHALL compute the horizontal and vertical displacement between WGS 84 (G2296) and NAD83(2011) at a given point and epoch, and visualize it as an arrow on the canvas, so users can see why "they are the same" is wrong at the meter level.

#### Scenario: Meter-level difference shown
- **WHEN** the displacement at a point in Kansas is computed for epoch 2026.7
- **THEN** the tool reports a horizontal difference on the order of 1–2 m with direction, and the canvas draws the displacement arrow

### Requirement: Legacy datum shifts
The domain SHALL support common legacy datums via published 3- or 7-parameter shifts (e.g. ED50, OSGB36, Tokyo, NAD27 via Molodensky where no grid exists) and SHALL label them with the published accuracy (typically meters) and warning `LOW_ACCURACY_TRANSFORM`.

#### Scenario: Legacy shift warning
- **WHEN** ED50 is transformed to WGS 84 with a 3-parameter shift
- **THEN** the result includes `LOW_ACCURACY_TRANSFORM` and the published accuracy in meters

### Requirement: Beta frames labeled
Transformations to or from NATRF2022 (and other NSRS 2022 frames) SHALL use the NGS beta parameters registered in the data assets and SHALL include `NON_OFFICIAL_DATUM` until the registry marks them official.

#### Scenario: NATRF2022 beta
- **WHEN** a point is transformed to NATRF2022
- **THEN** the result includes `NON_OFFICIAL_DATUM` and the NGS beta publication date
