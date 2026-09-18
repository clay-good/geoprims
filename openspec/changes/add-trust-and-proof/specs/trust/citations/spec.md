## Purpose

Gives every geoprims tool a complete, specific, verifiable account of where its math and constants come from, so any answer can be traced to a primary source.

## ADDED Requirements

### Requirement: Citation record per tool
Every tool SHALL have a citation record containing:
- `formula`: a plain-language statement of the method
- `sources`: one or more entries, each with publisher or author, title, edition or version, year, section/page/equation number, a stable URL or DOI, and `freeAccess` (a public link, or a statement of where it can be read free, or "paid only")
- `governs`: who is authoritative for this quantity (e.g. "ICAO Doc 7488 defines the standard atmosphere; your aircraft's POH/AFM governs performance")
- `assumptions`: every numeric constant or default the tool uses that the user does not supply, each with value, unit, and its own source

A tool without a complete record SHALL fail the citation-coverage gate.

#### Scenario: Coverage gate
- **WHEN** a tool has a citation record with a source missing its edition or section locator
- **THEN** the build fails naming the tool and the missing field

#### Scenario: Assumption sourced
- **WHEN** the density-altitude tool uses R = 287.05287 J/(kg·K)
- **THEN** its assumptions list includes that value with the ICAO Doc 7488/3 citation

### Requirement: Primary sources preferred and ranked
Sources SHALL be ranked: (1) the governing standard or regulation, (2) the issuing agency's publication or reference implementation, (3) peer-reviewed literature, (4) authoritative textbooks, (5) vendor guidance, labeled as such. Secondary sources (encyclopedias, blogs, forums) SHALL NOT be the sole citation for any formula or constant.

#### Scenario: Vendor guidance labeled
- **WHEN** the overlap preset cites Pix4D guidance
- **THEN** the citation is labeled "vendor guidance" and appears after any standard

### Requirement: No reproduction of copyrighted tables
Tools SHALL NOT reproduce copyrighted tables or figures (for example AASHTO design K-value tables, ASTM tables, paid ICAO manual tables beyond defining constants). Where a practitioner normally reads a value from such a table, the tool SHALL take the value as an input and cite the table by title, edition, and table number, so the user can look it up.

#### Scenario: Sight-distance design value
- **WHEN** a user opens the vertical-curve sight-distance tool
- **THEN** the design K value is an input field citing "AASHTO Green Book, 7th ed., Table 3-34 (look up your design speed)", with no table reproduced

### Requirement: Bare links are safe and specific
Citation links SHALL use `rel="noopener noreferrer"`, SHALL point to the most specific stable page (the document, not a home page), and SHALL be checked monthly by the free-access probe (per `trust/freshness`).

#### Scenario: Specific link
- **WHEN** the WMM citation renders
- **THEN** it links to the NCEI WMM product page and the WMM2025 technical report, not to noaa.gov

### Requirement: Citations travel with results
Citations SHALL be included in: the tool page proof panel; the printed page; the "Copy answer with reference" text; the calculation-sheet export; MCP `geoprims_describe` at `schema` detail and above; and each MCP `geoprims_run` result's `meta.references` (short form: publisher, title, edition, locator).

#### Scenario: Copy answer with reference
- **WHEN** a surveyor copies a combined-factor result with reference
- **THEN** the clipboard contains the result with units, the inputs, the method, the citations with locators, the tool and core versions, the date, and the not-for-legal-determination notice

### Requirement: Inverse source map
The build SHALL produce an inverse map from each cited source to the tools that cite it. It is used by the `/sources` page and by edition rollovers, so that a new edition of a standard lists every affected tool.

#### Scenario: Edition rollover impact
- **WHEN** the ledger records a new edition of the ASPRS Positional Accuracy Standards
- **THEN** the rollover report lists every tool citing the previous edition
