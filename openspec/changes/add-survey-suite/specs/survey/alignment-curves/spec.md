## Purpose

Computes horizontal circular, spiral, and vertical parabolic curve geometry and field layout data for roads, rail, pipelines, and site work.

## ADDED Requirements

### Requirement: Horizontal circular curve elements
Given any two independent elements (radius R, deflection angle Δ, tangent T, length L, chord C, external E, middle ordinate M, or degree of curve D), the tool SHALL compute all others. It SHALL support both the arc definition (D = 5,729.578 / R for 100 ft arcs) and the chord definition (sin(D/2) = 50 / R) of the degree of curve, labeled, with metric equivalents.

#### Scenario: R = 500 ft, Δ = 30°
- **WHEN** R = 500 ft and Δ = 30°00'00"
- **THEN** T ≈ 133.975 ft, L ≈ 261.799 ft, C ≈ 258.819 ft, E ≈ 17.638 ft, M ≈ 17.037 ft, D(arc) ≈ 11.4592° (11°27'33"), D(chord) ≈ 11.4783°

#### Scenario: Inconsistent inputs
- **WHEN** a user gives R, T, and Δ that are mutually inconsistent
- **THEN** the tool returns `INVALID_INPUT` asking for exactly two independent elements

### Requirement: Stationing and layout
Given the PI station and curve elements, the tool SHALL compute PC and PT stations and SHALL produce a layout table at a chosen interval (full and sub-chords). The table SHALL give deflection angles from the PC, chord lengths, and coordinates (given PI coordinates and the back-tangent azimuth). Stations SHALL be formatted in the chosen convention (`12+34.56` for feet, `1+234.567` for meters).

#### Scenario: Layout table
- **WHEN** layout at 50 ft stations is requested
- **THEN** each row gives station, deflection angle, chord, and coordinates, and the final deflection equals Δ/2

### Requirement: Spiral (clothoid) transitions
The tool SHALL compute spiral elements (spiral length Ls, spiral angle θs, X, Y, p, k, long and short tangents, total tangent for spiral-curve-spiral), stations TS/SC/CS/ST, and layout deflections. It SHALL follow the stated series (with the number of series terms) accurate to 0.001 of the unit.

#### Scenario: Spiral-curve-spiral stations
- **WHEN** Ls = 200 ft, R = 1,000 ft, Δ = 40°
- **THEN** TS, SC, CS, ST stations are computed and the circular arc length equals (Δ − 2θs) × R in radians

### Requirement: Vertical parabolic curves
Given incoming and outgoing grades g1 and g2, curve length L (or a K value), and PVI station and elevation, the tool SHALL compute:
- PVC and PVT stations and elevations
- elevations at any station or interval
- the high or low point (station and elevation), when one lies within the curve
- K = L / |g2 − g1| (with grades in percent)
- rate of change of grade

It SHALL support unequal-tangent curves.

#### Scenario: Crest curve high point
- **WHEN** g1 = +2%, g2 = −3%, L = 600 ft, PVI at 10+00 elevation 100.00 ft
- **THEN** PVC is at 7+00 with elevation 94.00 ft, the high point is 240 ft past the PVC (station 9+40) at elevation 96.40 ft, and K = 120

#### Scenario: No turning point
- **WHEN** both grades have the same sign
- **THEN** the result states that no high or low point exists within the curve

### Requirement: Sight distance references
The tool SHALL compute the minimum curve length for a stopping sight distance with user-entered eye and object heights, using the standard crest and sag formulas. Design K values from AASHTO SHALL be shown as dated reference data only.

#### Scenario: Crest SSD
- **WHEN** SSD = 400 ft with eye height 3.5 ft and object height 2.0 ft on the crest curve above
- **THEN** the minimum curve length is computed using the S < L or S > L case as appropriate, and the case is stated

### Requirement: Alignment visualization
Curve tools SHALL draw the plan view (tangents, PI, PC, PT, spirals, chords) and the profile view (grades, vertical curve, high/low point) with stations labeled.

#### Scenario: Plan and profile
- **WHEN** a horizontal and vertical curve are computed together
- **THEN** the canvas shows the plan and profile views linked by station
