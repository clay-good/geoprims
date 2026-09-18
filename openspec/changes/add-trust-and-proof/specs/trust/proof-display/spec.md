## Purpose

Shows users, at a glance and on demand, why a geoprims answer can be trusted: the formula with their numbers, the matching worked example, the sources, what the tool simplifies, and when it was last verified.

## ADDED Requirements

### Requirement: "How we got this" panel on every tool
Below every result, a collapsible panel titled "How we got this" SHALL show:
1. **Show your work.** The formula in standard notation, the same formula with the user's values substituted (with units), and every intermediate value.
2. **Worked example.** "You enter / You get", matching the independent worked example.
3. **Sources.** Citations with locators and free-access links.
4. **Assumptions.** Constants and defaults, each with its source.
5. **Limitations.** What the tool does not model.
6. **Status.** "Last verified <date> against <source>", reviewer status, tool and core versions, and links to test vectors and the verification report.

The panel SHALL be closed by default on phones and open by default on desktop widths of at least 1,024 px. It SHALL print expanded.

#### Scenario: Substituted formula
- **WHEN** a user computes density altitude
- **THEN** the panel shows the density-altitude formula, the same formula with their pressure and temperature substituted, and intermediate σ and PA values

#### Scenario: Printed proof
- **WHEN** the page is printed
- **THEN** the panel content prints in full even if collapsed on screen

### Requirement: Limitation banners for simplified tools
Tools that simplify a governing method (for example dry-air density altitude, or the spherical haversine distance) SHALL show a short banner above the result. The banner states the simplification (≤ 80 characters), what to use instead (≤ 240 characters), and who governs (≤ 120 characters). The same text SHALL appear in MCP results.

#### Scenario: Banner text shared
- **WHEN** the MCP runs a simplified tool
- **THEN** `meta.limitations` contains the same banner text shown on the web page

### Requirement: Context bands for sanity checking
Where a physically typical range exists (e.g. hover power per kg, GSD for mapping, density altitude above a field), results SHALL show a context phrase such as "7,930 ft (about 2,900 ft above the field)" or "2.7 cm/px (typical mapping range 1–5 cm/px)". The band's source SHALL be cited, and the band SHALL never read as a safety verdict.

#### Scenario: Context band
- **WHEN** a GSD of 12 cm/px is computed
- **THEN** the result reads "12 cm/px: coarser than typical mapping work (1–5 cm/px)", citing the basis

### Requirement: Site-wide trust pages
The site SHALL publish:
- `/sources`: every ledger source with edition, status, last verified, and the tools citing it.
- `/methodology`: the correctness layers, numeric determinism, verification process, review status, and how to report a problem.
- `/verification/<version>`: per `platform/verification`, plus per-tool worked-example sources.
- `/changelog`: result-change entries labeled.
- `/known-issues`: per `feedback/triage-and-corrections`.

Every tool page's proof panel SHALL link to the relevant entries.

#### Scenario: Sources page
- **WHEN** a user opens `/sources` and selects ICAO Doc 7488
- **THEN** they see its edition, last-verified date, and every tool that cites it

### Requirement: Test vectors are downloadable
Each tool page SHALL link to its golden vectors as a JSON Lines file, with the sources for each vector. The MCP resource `geoprims://tool/{id}` SHALL expose the same vectors.

#### Scenario: Vector download
- **WHEN** a developer clicks "Test vectors" on the UTM forward page
- **THEN** a `.jsonl` file downloads with inputs, expected outputs, tolerances, and sources
