## Purpose

Provides the geoprims web application frame: one fast, pre-rendered page per tool, forms generated from tool manifests, live recomputation, shareable permalinks, history, tool chaining, and settings.

## ADDED Requirements

### Requirement: One pre-rendered page per endpoint
Every catalog endpoint SHALL have a stable route `/<domain>/<group>/<operation>` served as pre-rendered static HTML that includes the tool title, summary, input form (functional after hydration), documentation, and references. The page SHALL be readable and its documentation usable with JavaScript disabled.

#### Scenario: Direct route load
- **WHEN** a user opens `/aviation/airspeed/cas-to-tas`
- **THEN** the server returns static HTML for that tool with HTTP 200, and the tool is interactive after the core module loads

#### Scenario: JavaScript disabled
- **WHEN** JavaScript is disabled
- **THEN** the page shows the documentation, formula, worked example, and a notice that computation requires JavaScript and WebAssembly

### Requirement: Load performance budgets
Tool pages SHALL meet the load budgets and use the reference device and network profile defined once in `contracts/reference-profiles`: hard limits of LCP ≤ 2.0 s, interactive (tool computes on input) ≤ 2.5 s cold and ≤ 500 ms warm, INP ≤ 200 ms, and CLS ≤ 0.1, with targets of LCP 1.5 s, INP 100 ms, and CLS 0.05. The initial JavaScript for the shell SHALL be at most 90 KB compressed, excluding Wasm and canvas code, which SHALL load after first paint.

#### Scenario: Budget enforced in CI
- **WHEN** the performance job measures a sampled set of 50 routes on the reference profile
- **THEN** every route meets the hard limits and the shell bundle budget, else CI fails

### Requirement: Schema-driven input forms
Tool input forms SHALL be generated from the manifest's input schema: each field shows its label, unit selector (with the declared default unit), range hint, and help text. Coordinate fields SHALL accept any supported coordinate notation (DD, DMS, DDM, MGRS, UTM, Maidenhead, geohash, Plus Code) and display the parsed interpretation next to the field before computing.

#### Scenario: Parsed interpretation shown
- **WHEN** a user types `40°26'46"N 79°58'56"W` in a coordinate field
- **THEN** the field shows the parsed value `40.446111°, -79.982222°` and the tool computes with it

#### Scenario: Ambiguous coordinate
- **WHEN** a user types `40.4461 -79.9822` with no hemisphere or label
- **THEN** the field interprets it as latitude, longitude and shows a notice "Read as lat, lon" with a one-click swap control

### Requirement: Live recompute
Tools SHALL recompute automatically as inputs change, debounced to at most one computation per animation frame for closed-form tools and per 150 ms for iterative or data-proportional tools. Invalid intermediate input SHALL show a field-level message without clearing the previous valid result, and the previous result SHALL be visibly marked "Result out of date".

#### Scenario: Stale result marking
- **WHEN** a user deletes a digit making an input invalid
- **THEN** the field shows the validation message and the last result remains visible with a "Result out of date" marker until input is valid again

### Requirement: Permalinks in the URL fragment
The current inputs, selected units, and canvas view SHALL be encoded in the URL fragment so that copying the address bar reproduces the calculation. Fragment encoding SHALL be versioned (`#v1:...`) and backward-compatible across releases.

#### Scenario: Permalink round trip
- **WHEN** a user opens a copied permalink in a new browser profile
- **THEN** the same inputs, units, and result appear

#### Scenario: Old fragment version
- **WHEN** a user opens a permalink with an older fragment version whose fields were renamed
- **THEN** the app migrates the fields and computes, or shows which fields could not be migrated

### Requirement: Result panel
The result panel is the answer card defined in `ux/glanceable-results`; together with its expandable details it SHALL show every output with its unit, a unit switcher, a copy button per value and for the whole result (as JSON, as plain text, and as an MCP `geoprims_run` call), the provenance (`meta`) in an expandable section, and any warnings prominently above the values.

#### Scenario: Copy as MCP call
- **WHEN** a user chooses "Copy as agent call"
- **THEN** the clipboard contains a JSON object with the tool id and inputs, directly usable with the MCP `run` tool

### Requirement: Tool chaining
A user SHALL be able to send any output value (or set of outputs) to another tool's compatible input via a "send to" action that lists tools whose inputs accept that quantity type, and the chain SHALL be representable in a single permalink.

#### Scenario: Chain geodesic to wind
- **WHEN** a user sends the initial course from a geodesic inverse to the wind-triangle tool's course input
- **THEN** the wind-triangle tool opens with that course populated and a breadcrumb to the source tool

### Requirement: Recent and pinned tools
The app SHALL remember up to 50 recently used tools and any user-pinned tools in local storage, shown on the home screen and in the command palette, and SHALL allow clearing them.

#### Scenario: Recent list
- **WHEN** a user has used 3 tools
- **THEN** the home screen lists them most recent first

### Requirement: Settings
A settings screen SHALL provide: unit profile, coordinate display format, number format (decimal point or comma), theme mode, reduced motion, audio on/off and volume, canvas default mode, offline pack management, and erase-all-local-data. Settings SHALL apply immediately and persist locally.

#### Scenario: Unit profile applied globally
- **WHEN** a user selects the `aviation` unit profile
- **THEN** every tool opened afterwards defaults distance to NM, speed to kt, and altitude to ft

### Requirement: Safety notice on operational tools
Every tool in the `aviation` and `drone` domains, and every navigation tool, SHALL display a persistent notice that cannot be dismissed "Planning and education aid. Not for primary navigation." with a link to the full disclaimer. The notice SHALL NOT be a modal and SHALL NOT block use.

#### Scenario: Notice visible
- **WHEN** a user opens any aviation tool
- **THEN** the notice is visible in the tool header without interaction

### Requirement: Error resilience
If the Wasm core fails to load (unsupported browser, blocked by policy), the page SHALL state the reason and the minimum browser requirements, and SHALL keep documentation usable.

#### Scenario: Wasm blocked
- **WHEN** WebAssembly is disabled by enterprise policy
- **THEN** the tool area shows "This tool needs WebAssembly, which is disabled in this browser" and the docs remain readable
