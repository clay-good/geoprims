## Purpose

Gives the approach-planning references that pilots look up from regulations and orders: RVR to visibility, approach category, and the circling protected radius, all dated and cited.

## ADDED Requirements

### Requirement: RVR and visibility conversion
The tool SHALL convert RVR to the equivalent visibility by the 14 CFR 91.175(h)(2) table (1,600 ft = ¼ SM; 2,400 = ½; 3,200 = ⅝; 4,000 = ¾; 4,500 = ⅞; 5,000 = 1; 6,000 = 1¼), stating that it applies when RVR minimums are prescribed but RVR is not reported and not to Category II or III, and SHALL convert reported meteorological visibility to RVR/CMV by the EASA AMC10 CAT.OP.MPA.110 Table 8 factors (high-intensity lights ×1.5 day, ×2.0 night; other lights ×1.0 day, ×1.5 night; no lights ×1.0 day, not applicable at night) with its exclusions. Values SHALL come from `data/regulations.json` with "rules as of" shown. As an option, the tool SHALL also convert by the Terminal Procedures Publication legend's Comparable Values of RVR and Visibility table, dated by its d-TPP cycle, which adds rows to the regulation's and says to use the next higher listed RVR between rows; a row the regulation does not list SHALL carry the caution `RVR_NOT_IN_REGULATION`, because the legend reserves the regulation's rows for takeoff and landing minima.

#### Scenario: FAA table
- **WHEN** RVR 2,400 ft is converted
- **THEN** the visibility is ½ SM and the applicability note is shown

#### Scenario: Chart legend table
- **WHEN** RVR 4,800 ft is converted with the Terminal Procedures Publication legend's table
- **THEN** the next higher listed RVR, 5,000 ft, is used and the visibility is 1 SM, as the legend's own example gives

#### Scenario: A legend row the regulation lacks
- **WHEN** RVR 2,800 ft is converted with the legend's table
- **THEN** the 3,000 ft row gives 5/8 SM with a caution that the regulation does not list that row, so it is not a takeoff or landing minimum

#### Scenario: EASA night, no lights
- **WHEN** meteorological visibility is converted at night with no approach lights
- **THEN** the result states that the conversion is not applicable

### Requirement: Approach category
The tool SHALL assign the aircraft approach category from V_REF, or 1.3 V_S0 at maximum landing weight when V_REF is not specified, per 14 CFR 97.3: A below 91 kt, B 91 to below 121, C 121 to below 141, D 141 to below 166, E 166 and above, and SHALL note that a higher-speed approach requires the higher category's minimums.

#### Scenario: Boundary
- **WHEN** V_REF is 121 kt
- **THEN** the category is C, and 120.9 kt is B

### Requirement: Circling radius
The tool SHALL compute the circling approach radius by FAA Order 8260.3G §2-7-1 (superseding 8260.3F, with the same section): true airspeed by formula 2-7-1 from the category's indicated speed (Table 2-7-1: A 90, B 120, C 140, D 165, E 200 KIAS) at the circling altitude and airport elevation (height above airport at least 1,000 ft), radius by formula 2-7-2 with the category's bank (25°, 25°, 20°, 20°, 22°) and S term (0.4, 0.4, 0.5, 0.6, 0.7 NM), minimum 1.30 NM. It SHALL show the fixed radii charted before Order 8260.3 Change 21 (1.3, 1.5, 1.7, 2.3, 4.5 NM, Instrument Procedures Handbook Figure 4-2) as dated legacy values and SHALL NOT present the banded legend table until it is read from the Terminal Procedures Publication legend and its assumptions recorded.

#### Scenario: Category C
- **WHEN** category C is circled at 1,000 ft MSL at a sea-level airport
- **THEN** the radius is computed by formulas 2-7-1 and 2-7-2 with the Table 2-7-1 inputs shown, and the result is at least 1.30 NM

#### Scenario: Minimum radius
- **WHEN** category A at a sea-level airport gives a formula result below 1.30 NM
- **THEN** the radius reported is 1.30 NM with the minimum rule cited
