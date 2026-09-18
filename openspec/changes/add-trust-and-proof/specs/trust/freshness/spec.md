## Purpose

Keeps geoprims current as standards, models, and regulations change. A tool can never quietly cite a superseded edition, run on an expired model, or quote a rule that has since changed.

## ADDED Requirements

### Requirement: Sources ledger
The repository SHALL maintain a sources ledger with one row per tracked standard, model, dataset, or regulation. Each row SHALL contain:
- `id`, `name`, `issuer`
- `currentEdition`, `currentReleaseDate`
- `cycleMonths` (typical revision interval, or null)
- `nextExpected` (date or null)
- `validFrom` / `validTo` (for time-bounded models)
- `lastVerified` (date a maintainer confirmed the current edition at the issuer)
- `verificationNote`, `freeAccessUrl`
- `matchTerms` (strings that identify citations of this source)
- `editionStatus` (`current`, `disclosed-lag`, `acknowledged-stale`)
- `legalStatus` for regulations (`in-force`, `proposed`, `withdrawn`), null otherwise

The initial ledger SHALL cover at least: WMM, WMMHR, IGRF, EGM96, EGM2008, GEOID18, the NSRS 2022 products (beta), NADCON5, SPCS83/SPCS2022, EPSG dataset version, ICAO Doc 7488, US Standard Atmosphere 1976, ICAO Doc 8168, AIM, 14 CFR Parts 61/91/107/89 and the Part 108 NPRM, EASA 2019/947 and 2019/945, ASPRS Positional Accuracy Standards, ALTA/NSPS standards, USGS Lidar Base Specification, H3, S2, Open Location Code, A5, Copernicus DEM, and the IERS leap-second bulletin.

#### Scenario: Ledger completeness
- **WHEN** a citation names a source whose `matchTerms` are not in the ledger
- **THEN** the build fails asking for a ledger row

### Requirement: Superseded editions fail the build
The build SHALL fail when any citation names an edition of a tracked source other than `currentEdition`, unless the ledger row has `editionStatus` `disclosed-lag` and the tool page shows a disclosure naming the newer edition and the reason for lagging.

#### Scenario: New edition published
- **WHEN** a maintainer updates the ASPRS row to a new edition and a tool still cites the previous one
- **THEN** the build fails listing the tool, until the tool is updated or a lag disclosure is added

### Requirement: Overdue verification fails the build
When `nextExpected` has passed and `lastVerified` is earlier than `nextExpected`, the build SHALL fail until a maintainer checks the issuer and re-stamps `lastVerified` (with a note, whether or not a new edition appeared). A `lastVerified` older than 365 days SHALL produce a warning for every row.

#### Scenario: Overdue check
- **WHEN** IGRF's next expected release date passes without re-verification
- **THEN** CI fails with "IGRF: next edition was expected on <date>; verify and re-stamp"

### Requirement: Model validity enforcement
Time-bounded models SHALL be tracked by `validTo`. The build SHALL warn 12 months before expiry and fail at expiry unless a successor model is registered. Tool pages using a model within 6 months of expiry SHALL show a notice.

#### Scenario: WMM expiry approaching
- **WHEN** the build date is on or after 2029-01-01 and no WMM2030 is registered
- **THEN** CI warns, and the declination pages show "WMM2025 is valid through 2029-12-31; the successor is expected in December 2029"

### Requirement: Verified-on dates come from the ledger, never the clock
Displayed "last verified" dates SHALL come from the ledger or worked-example fixtures, never from the build time. Provenance stamps SHALL move only forward: a change that makes a `lastVerified` or data stamp older than on the base branch SHALL fail CI.

#### Scenario: Stale revert blocked
- **WHEN** a data-refresh branch would reset GEOID18's `lastVerified` to an earlier date
- **THEN** the monotonic-stamp check fails

### Requirement: Free-access link probe
A scheduled monthly job SHALL request every `freeAccessUrl` and citation URL. It SHALL open an issue listing broken or redirected links, without failing normal builds.

#### Scenario: Broken link
- **WHEN** an agency moves a PDF
- **THEN** the monthly probe opens an issue naming the citation, tools, and HTTP status

### Requirement: Regulatory values carry review dates
Every regulatory reference entry (per `platform/data-assets` reference-data files) SHALL be linked to a ledger row, and its page display SHALL read "Rules as of <lastVerified>". Proposed rules SHALL show "Proposed" until the ledger's `legalStatus` changes to `in-force`.

#### Scenario: Part 108 finalized
- **WHEN** the FAA publishes a Part 108 final rule and the ledger row's `legalStatus` is updated to `in-force`
- **THEN** the "Proposed" labels disappear in the next build, and the tools citing it are listed for review
