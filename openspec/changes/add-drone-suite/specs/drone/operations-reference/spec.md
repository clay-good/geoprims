## Purpose

Gives drone operators dated, cited regulatory limits and simple compliance calculators (altitude, speed, kinetic energy, class and category) without pretending to be legal advice, and labels proposed rules as proposed.

## ADDED Requirements

### Requirement: Regulatory references are dated and cited
Every regulatory value SHALL come from a reference-data file entry containing: jurisdiction, rule citation, value, unit, effective date, "rules as of" review date, status (`in-force` or `proposed`), and a link to the authority. The initial entries SHALL cover:
- FAA 14 CFR Part 107: 400 ft AGL, or within 400 ft of a structure and no higher than 400 ft above its top; 87 kt (100 mph) groundspeed; 3 statute mile visibility; cloud clearance of 500 ft below and 2,000 ft horizontal; VLOS; under 55 lb.
- FAA Part 89 Remote ID: compliance required since March 16, 2024.
- The FAA Part 108 BVLOS NPRM (August 7, 2025), status `proposed`.
- EASA open category (A1, A2, A3 subcategories; C0–C4 class marks, with C5/C6 belonging to the specific category's standard scenarios; 120 m height limit; 25 kg MTOM).

#### Scenario: Part 108 status
- **WHEN** a user views any Part 108 value before a final rule is published
- **THEN** it is labeled "Proposed" with the Federal Register citation, and no compliance check treats it as in force

#### Scenario: Stale review date
- **WHEN** a reference entry's review date is more than 12 months old at build time
- **THEN** the build emits a warning listing the entry for re-verification

### Requirement: Altitude limit calculator
A tool SHALL compute the maximum permitted altitude at a point under Part 107, including the structure exception (a user-entered structure height and distance). It SHALL convert the limit to MSL and HAE via the geodesy height tools, using a DEM ground elevation (with its accuracy stated) or a user-entered one.

#### Scenario: Near a structure
- **WHEN** operating 200 ft horizontally from a 300 ft tower
- **THEN** the maximum altitude is 700 ft AGL (300 + 400), and the result explains the 400 ft radius condition

#### Scenario: MSL conversion
- **WHEN** ground elevation is 5,280 ft MSL
- **THEN** the 400 ft AGL limit is shown as 5,680 ft MSL and as HAE with the geoid used

### Requirement: Speed and kinetic energy checks
Tools SHALL check groundspeed against the 87 kt limit, including wind (airspeed ≠ groundspeed), and SHALL compute impact kinetic energy KE = ½mv² in joules and ft-lb for comparison against category thresholds (e.g. EASA C1 < 80 J at maximum speed; FAA operations-over-people categories), each cited.

#### Scenario: EASA C1 energy
- **WHEN** a 0.9 kg drone at 19 m/s is checked against C1
- **THEN** KE ≈ 162.5 J (±0.1 J), which exceeds 80 J; the result explains that C1 compliance is based on mass under 900 g or impact energy under 80 J per the class definition, and cites the regulation

### Requirement: EASA class and subcategory helper
Given drone mass, class mark (or legacy/no mark), and intended proximity to people, a tool SHALL list the open subcategories available (A1/A2/A3) and their distance rules, citing Regulation (EU) 2019/947 and 2019/945 as amended. It SHALL state that national standard scenarios expired on December 31, 2025, and that STS operations require C5/C6.

#### Scenario: Legacy 2 kg drone
- **WHEN** a legacy (no class mark) 2 kg drone is entered
- **THEN** the only available open subcategory is A3, and the 150 m distance from residential, commercial, industrial, or recreational areas is shown

### Requirement: Not legal advice
Every operations-reference result SHALL display "Summary of rules as of <date>. Not legal advice. Check the current regulation and any waivers, authorizations, or local restrictions." with the authority link.

#### Scenario: Disclaimer present
- **WHEN** any operations-reference tool shows a result
- **THEN** the disclaimer with the review date is visible
