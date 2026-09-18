## Purpose

Turns written land descriptions (metes-and-bounds deeds and PLSS legal descriptions) into plotted, measured, and checked figures. Every parsed element is shown for the user to confirm, so the tool does the arithmetic while the professional keeps the interpretation.

## ADDED Requirements

### Requirement: Metes-and-bounds parsing with confirmation
The deed tool SHALL parse pasted deed text into an ordered list of calls. Supported forms:
- quadrant bearings (`N 45°30'15" E`, `North 45 degrees 30 minutes East`, `N45-30-15E`, "due north", `N 90° E` = East)
- distances with units (feet, chains and links, rods and poles, varas, meters)
- "thence" separators

Curve calls (radius, arc length, delta, chord bearing and distance, tangent or non-tangent, left or right) SHALL be parsed where complete and flagged `CURVE_CALL_INCOMPLETE` where elements are missing. Monuments, adjoiners, and "along said line" references SHALL be captured as text and flagged, never interpreted.

Each parsed call SHALL be shown in an editable table beside the original text, with the source phrase highlighted. Computation SHALL use the confirmed table.

#### Scenario: Parse and confirm
- **WHEN** a deed with six line calls and one non-tangent curve with chord data is pasted
- **THEN** seven calls appear in the table with their source phrases highlighted, the curve marked non-tangent, and nothing computed until the user confirms or edits

#### Scenario: Monument reference
- **WHEN** a call reads "thence along the centerline of Mill Creek to an iron pin"
- **THEN** the call is flagged `NON_METRIC_CALL` with the text preserved, and the user is asked for a bearing and distance or to leave the figure open

### Requirement: Plot, closure, and area
From confirmed calls, the tool SHALL plot the figure and compute the linear misclosure, its direction, and the precision ratio (per `survey/cogo-and-traverse`). It SHALL compute the area of the figure closed by an implied closing line, with that assumption stated. It SHALL NOT adjust the courses unless the user explicitly chooses an adjustment method, which is then labeled.

#### Scenario: Unclosed deed
- **WHEN** the confirmed calls miss closure by 0.42 ft over 1,850 ft
- **THEN** the result reports misclosure 0.42 ft, precision ≈ 1:4,400, and area "computed with an implied closing line of 0.42 ft", with no automatic adjustment

### Requirement: Legacy land units
A units tool SHALL convert chains, links, rods/poles/perches, furlongs, varas, arpents (length and area), acres, and sections. Every definition SHALL cite its source. Jurisdiction-dependent units SHALL require a jurisdiction:
- the Texas vara (33⅓ in, by statute)
- California and other Spanish-grant varas
- the Louisiana and Missouri arpents

Chain-based units SHALL default to US survey feet for legacy deeds, with the basis stated.

#### Scenario: Vara requires jurisdiction
- **WHEN** a user converts "1,000 varas" without a jurisdiction
- **THEN** the tool asks for the jurisdiction and lists the options with their definitions

#### Scenario: Chains to feet
- **WHEN** 12 chains 34 links is converted
- **THEN** the result is 814.44 US survey feet, with the basis stated (1 chain = 66 ftUS; 1 link = 0.66 ftUS)

### Requirement: PLSS legal descriptions
A PLSS tool SHALL:
- parse descriptions such as `NE¼ SW¼ Sec 12, T3N R4W, 6th PM` (read right to left: the NE quarter of the SW quarter), including half and quarter notations, lots, and principal meridian names
- validate township and range ranges
- compute the nominal aliquot area for a standard 640-acre section, labeled nominal

With the optional per-state `plss-cadnsdi` asset (v1.1, BLM, public domain), it SHALL convert descriptions to polygons and coordinates to descriptions using the actual section geometry, stating the dataset's positional reliability. It SHALL never derive aliquot geometry by quartering an idealized square.

#### Scenario: Aliquot parse
- **WHEN** `NE¼ SW¼ Sec 12, T3N R4W, 6th PM` is parsed
- **THEN** the result identifies the northeast quarter of the southwest quarter of section 12, nominal area 40 acres (labeled nominal), and asks to install the state PLSS data for location

#### Scenario: Meridian required
- **WHEN** a description omits the principal meridian
- **THEN** the tool asks for it, because township numbers repeat across meridians

### Requirement: Basis-of-bearing rotation
A tool SHALL rotate a set of calls or coordinates from one basis of bearing to another, given a line's bearing in both bases, and report the rotation angle.

#### Scenario: Rotate deed to grid
- **WHEN** a deed line recorded as N 10°00'00" E is N 10°02'30" E on grid
- **THEN** all calls rotate by +0°02'30", and the rotation is reported

### Requirement: Professional-use notice
Land-description tools SHALL display: "Parsing and arithmetic aid. Boundary determination requires a licensed surveyor and the governing records."

#### Scenario: Notice present
- **WHEN** any land-description result is shown
- **THEN** the notice is visible with the result
