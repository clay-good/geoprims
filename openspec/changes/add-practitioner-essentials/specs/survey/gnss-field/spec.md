## Purpose

Supports GNSS field work from planning through reporting: satellite geometry windows, expected accuracy, session length, correct antenna heights, precision standards, and site localization.

## ADDED Requirements

### Requirement: DOP and sky plot from a pasted almanac
Given a pasted GPS almanac (YUMA or SEM), optional other-constellation almanacs, a site, a date and time window, and an elevation mask (default 10°), the tool SHALL compute visible satellite counts and GDOP, PDOP, HDOP, and VDOP through the window. It SHALL draw a sky plot and a DOP chart. It SHALL warn `ALMANAC_OLD` when the almanac is more than 7 days older than the planning date, and SHALL state that terrain and canopy obstruction are not modeled unless a terrain horizon mask is applied from the raster tools.

#### Scenario: Old almanac
- **WHEN** an almanac from 10 days before the planning date is used
- **THEN** results are computed with `ALMANAC_OLD`

#### Scenario: Terrain mask
- **WHEN** the user applies a DEM horizon mask at the site
- **THEN** satellites below the terrain horizon are excluded and the mask is drawn on the sky plot

### Requirement: RTK and PPK error budget
Given the receiver's published horizontal and vertical specifications (a mm + b ppm) and the baseline length, the tool SHALL compute the expected 1σ horizontal and vertical precision (and a stated confidence scaling). It SHALL compare the result to a user target, stating that manufacturer specifications assume ideal conditions.

#### Scenario: 15 km baseline
- **WHEN** the spec is 8 mm + 1 ppm horizontal and the baseline is 15 km
- **THEN** the expected horizontal precision is 23 mm (a + b·d, as manufacturers state it)

### Requirement: OPUS session planning
A tool SHALL recommend the NGS OPUS service and minimum session length for a goal: OPUS-RS for sessions of 15 minutes to 4 hours, and OPUS-S for 2 to 48 hours (NGS guidance, cited and dated; both accept 2 to 4 hours, where the tool explains the trade-off). It SHALL generate the expected RINEX file naming for the day of year.

#### Scenario: One-hour session
- **WHEN** the planned session is 60 minutes
- **THEN** the tool recommends OPUS-RS and states the service's documented requirements

### Requirement: Antenna height reduction
Given a measured slant height to the antenna's measurement mark, the antenna radius to that mark, and the ARP offset (from the optional NGS ANTINFO asset or typed), the tool SHALL compute the vertical height to the antenna reference point (ARP). It SHALL show the geometry.

#### Scenario: Slant to vertical
- **WHEN** slant height = 1.800 m and antenna radius = 0.100 m, with a 0.000 m mark-to-ARP offset
- **THEN** the vertical height ≈ 1.7972 m (√(1.8² − 0.1²)), with the formula shown

### Requirement: ALTA/NSPS relative positional precision
A tool SHALL compute the allowable relative positional precision (0.07 ft or 2 cm, plus 50 ppm of the distance), per the ALTA/NSPS 2026 standards (effective February 23, 2026, cited as dated reference data). It SHALL compare it to the semi-major axis of the 95% relative error ellipse (2.448σ) computed from the least-squares adjustment or user-entered covariance, between adjacent boundary corners.

#### Scenario: Allowable at 1,000 ft
- **WHEN** the distance between adjacent corners is 1,000 ft
- **THEN** the allowable RPP is 0.12 ft

#### Scenario: Wrong measure rejected
- **WHEN** a user enters a traverse misclosure instead of an error ellipse
- **THEN** the tool explains that RPP is evaluated from the relative error ellipse, not misclosure, and asks for the ellipse or adjustment output

### Requirement: Site localization (calibration)
Given at least 2 control pairs (local coordinates and grid or geodetic coordinates), the tool SHALL fit a 2D similarity transform (4 parameters) or, with at least 3 pairs, an affine transform (6 parameters) by least squares. It SHALL report parameters, scale in ppm, rotation, residuals per point, and RMS. It SHALL warn `NO_REDUNDANCY` when the pairs exactly determine the fit, and `SCALE_SUSPECT` when the fitted scale differs from the expected combined factor by more than 100 ppm.

#### Scenario: No redundancy
- **WHEN** a 4-parameter fit uses exactly 2 points
- **THEN** residuals are zero and the result carries `NO_REDUNDANCY`

#### Scenario: Wrong units suspected
- **WHEN** the fitted scale is 0.3048 while the expected combined factor is about 0.99997
- **THEN** the result carries `SCALE_SUSPECT`, suggesting a feet or meters mix-up
