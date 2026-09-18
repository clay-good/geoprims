## Purpose

Makes every geoprims tool understandable at a glance: the answer first, in plain words, framed against something familiar, with only the inputs that matter in view.

## ADDED Requirements

### Requirement: Fixed tool-page anatomy
Every tool page SHALL present, in this order:
1. the tool name and a one-line purpose ("Find how high the airplane 'feels' on a hot day")
2. the answer card
3. core inputs
4. "More options" (collapsed)
5. the canvas
6. "How we got this" (per `trust/proof-display`)
7. related tools, then the report button's footer duplicate

On phones, the answer card SHALL be above the inputs and SHALL remain visible as a sticky bar while inputs are edited.

#### Scenario: Answer first on a phone
- **WHEN** a tool page loads at 390 × 844 CSS px
- **THEN** the answer card's value is visible without scrolling

#### Scenario: Consistent anatomy
- **WHEN** the anatomy gate renders every stable tool
- **THEN** each page contains the sections in the specified order

### Requirement: Answer card
The answer card SHALL show:
- the primary result as a large number with unit, at least 32 px on phones
- a plain-language sentence generated from the tool's sentence template
- a comparison line, where meaningful: a familiar reference, a rule-of-thumb value with its difference, or a typical range with its source
- warnings, as short labeled lines above the number
- a copy button for the value, and one for the sentence with its reference

Secondary outputs SHALL follow in a compact list.

#### Scenario: Density altitude sentence
- **WHEN** density altitude is computed for a 5,000 ft field at 30 °C and 29.80 inHg
- **THEN** the card reads "7,932 ft" and "Density altitude is 7,932 ft, about 2,900 ft higher than the field. Expect a longer takeoff roll and weaker climb.", followed by "Rule of thumb gives 8,093 ft (161 ft high)."

### Requirement: Sentence templates for every tool
Every tool manifest SHALL define an `x-sentence` template in plain US English (reading grade 8 or below, checked by a readability lint) that states the result, its meaning, and any key caveat. The template SHALL use the user's units and display precision. MCP results SHALL include the same rendered sentence as `summary`.

#### Scenario: Template coverage
- **WHEN** the build checks manifests
- **THEN** every stable tool has a sentence template, or the build fails naming it

#### Scenario: MCP summary parity
- **WHEN** an agent runs a tool
- **THEN** `summary` equals the sentence the web page would show for the same inputs and units

### Requirement: Status phrases without false assurance
Where a tool compares a result to a threshold (crosswind limit, Part 107 altitude, ASPRS class, closure standard), the card SHALL show one of three phrases: "Within <limit>", "Near <limit>" (within a declared margin), or "Beyond <limit>". Each SHALL have a distinct icon and text, and the threshold's source SHALL be cited. The words "safe", "unsafe", "legal", and "approved" SHALL NOT appear in any status phrase. Operational tools SHALL keep the planning-aid notice adjacent.

#### Scenario: Crosswind beyond limit
- **WHEN** the crosswind component exceeds the user's entered limit
- **THEN** the card shows "Beyond your 15 kt crosswind limit" with an icon, and no color-only indication

### Requirement: Prefilled examples and clear controls
On first load (no permalink), every tool SHALL be prefilled with its worked example and marked "Example values", so the page shows a real answer immediately. A "Clear" control SHALL empty all inputs. A "Try the example" control SHALL restore the example. Once the user edits any value, the example label SHALL disappear.

#### Scenario: First load shows an answer
- **WHEN** a new visitor opens the GSD tool
- **THEN** the example camera values are filled in, marked "Example values", and the answer card shows 2.74 cm/px

### Requirement: Progressive disclosure with visible defaults
A tool SHALL show at most 5 core inputs by default. Other inputs SHALL live under "More options", and each hidden default SHALL be stated in words next to the answer card ("Assumes dry air. Add dew point"), so no assumption is invisible.

#### Scenario: Hidden default stated
- **WHEN** density altitude runs without dew point
- **THEN** the card shows "Assumes dry air" with a link that opens the dew-point field

### Requirement: Plain vocabulary and glossary
Every abbreviation or term of art (PA, DA, ISA, GSD, MGRS, HAE, CF, RPP, and so on) SHALL be expanded on first use per page and linked to a glossary definition that opens on tap. Definitions SHALL be at most 40 words, cite a source, and live in a shared glossary file also used by explainers and MCP descriptions. Every input SHALL have a one-line help text with an example value.

#### Scenario: Tap to define
- **WHEN** a user taps "HAE"
- **THEN** a popover shows "Height above ellipsoid: the height GPS reports, measured from the WGS 84 ellipsoid rather than sea level", with its source

### Requirement: Glanceable home page
The home page SHALL show:
- one prominent search box that accepts tool names and questions (per `discovery/natural-language-prefill`)
- 8–12 hero tools as large cards, each with a one-line purpose and a live example answer
- "Start a journey" rows (preflight, drone mapping day, survey control, and so on)
- recent tools
- the four promises (free, no ads, no tracking, works offline) in one line

Domain browsing SHALL be one tap away. The home page SHALL render its hero answers without JavaScript.

#### Scenario: Question on home
- **WHEN** a visitor types "crosswind rwy 27 wind 300 at 15" on the home page
- **THEN** the crosswind tool opens prefilled with the answer shown

### Requirement: Usability validation before launch
Before launch, each hero tool SHALL pass:
- a recorded 5-second test with at least 5 target users per domain, in which at least 80% correctly state what the page computes and the current answer
- a task test in which at least 80% complete a representative calculation without help

Results and fixes SHALL be recorded in `docs/usability/`.

#### Scenario: Failed glance test
- **WHEN** fewer than 80% of testers can state what the holding-entry tool shows
- **THEN** the page is revised and retested before launch
