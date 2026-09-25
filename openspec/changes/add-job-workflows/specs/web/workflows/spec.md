## Purpose

Runs a practitioner's whole job on one page: the few inputs the job needs, every step computed live by the core, one combined picture, and one export.

## ADDED Requirements

### Requirement: Workflow page
Each workflow SHALL have a page at `/workflows/<slug>/` with, in order: the job in one sentence; a form of at most eight inputs, prefilled with a worked example, with more behind "More options"; the answer (the workflow's summary sentence); the step list; the combined visual; the assumptions; and export. Each step SHALL show its tool's answer sentence and a link that opens that tool prefilled with the step's exact inputs.

#### Scenario: Cross-country at a glance
- **WHEN** a pilot opens `/workflows/vfr-cross-country/`
- **THEN** the page shows a prefilled route and a nav log, with heading, groundspeed, time, and fuel for each leg, plus the fuel total with reserve, all computed before the first interaction

#### Scenario: Open a step in full
- **WHEN** the pilot selects the second leg's wind-triangle step
- **THEN** the wind-triangle tool opens with that leg's course, TAS, and wind already entered, and its answer matches the step's

### Requirement: Live recompute from the reader's inputs
Changing any workflow input SHALL rerun, in the core, every step that depends on it, and only those. Every value on the page SHALL come from a core result. The page SHALL NOT compute, round, or convert a value itself. The permalink SHALL hold the inputs.

#### Scenario: Wind changes the plan
- **WHEN** the pilot raises the winds aloft at the cruise altitude from 20 kt to 35 kt
- **THEN** each leg's heading, groundspeed, time, and fuel update, and so do the fuel total and the reserve check

### Requirement: Shared chain runner
Workflows SHALL be defined as data (`data/workflows.json`) and run by one chain runner shared by the site build, the workflow page, and the MCP server. Given the same inputs, the three SHALL produce byte-identical step results.

#### Scenario: Build-time check
- **WHEN** a workflow's step wiring names an output its source step does not return
- **THEN** the site build fails, naming the workflow, the step, and the output

### Requirement: A failed step stops the chain
If a step fails, the page SHALL show that step's own error against the input that caused it, and each later step SHALL show "Waiting for step N". No later step SHALL run on a guessed or stale value.

#### Scenario: Unreadable METAR
- **WHEN** a pilot pastes a METAR that does not parse into the preflight check
- **THEN** the METAR step shows the decoder's error, and pressure altitude and every later step wait

### Requirement: Assumptions are listed
Every step input the workflow fixes rather than asks for SHALL be listed under "Assumptions", with its value and a link to change it on the tool page.

#### Scenario: Reserve rule
- **WHEN** the cross-country workflow uses the 14 CFR 91.151 day VFR reserve of 30 minutes
- **THEN** "Fuel reserve: 30 min (day VFR, 14 CFR 91.151)" appears under Assumptions

### Requirement: Launch workflows
The site SHALL ship the eleven workflows of the design inventory: vfr-cross-country, preflight-check, mapping-flight, inspection, can-i-fly-now, convert-coordinate, gnss-to-state-plane, lidar-bid, ifr-approach-brief, boundary-retracement, and index-to-area. Each of the eight journey slugs SHALL redirect (301) to its workflow.

#### Scenario: Convert a coordinate
- **WHEN** a reader pastes `40°26'46"N 79°58'56"W` into `/workflows/convert-coordinate/`
- **THEN** the page lists the point as decimal degrees, DMS, DDM, UTM, MGRS, USNG, state plane (Pennsylvania South, NAD83), H3, S2, geohash, and Plus Code, each with a copy button, and pins it on the map

#### Scenario: Mapping flight sorties
- **WHEN** an operator plans a grid needing 38 minutes of flying with 22-minute batteries and a 25% reserve
- **THEN** the plan shows three sorties, each ending at a battery swap point marked on the map, and the flight time per sortie
