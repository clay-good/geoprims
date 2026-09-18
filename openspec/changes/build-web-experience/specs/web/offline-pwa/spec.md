## Purpose

Makes geoprims installable and fully usable offline, in the field, on the flight line, or in a remote survey site, with predictable updates and user control over storage.

## ADDED Requirements

### Requirement: Installable PWA
The site SHALL provide a valid web app manifest (name, icons including maskable, theme color per mode, `display: standalone`, start URL, shortcuts to the command palette and recent tools) and a service worker so that supporting browsers offer installation.

#### Scenario: Install criteria
- **WHEN** the PWA audit runs
- **THEN** the site passes installability checks in Chromium and is addable to the iOS home screen with correct icon and name

### Requirement: Offline core after first visit
After the first complete visit, the app shell, the catalog, all docs pages, the search index, and all Wasm modules SHALL be cached so that every tool not requiring an un-downloaded data asset works offline. The shell precache SHALL be at most 12 MB.

#### Scenario: Airplane mode
- **WHEN** a user who has visited once enables airplane mode and opens the density-altitude tool
- **THEN** the page loads and computes normally

#### Scenario: Missing asset offline
- **WHEN** an offline user requests an EGM2008 geoid height for a tile not downloaded
- **THEN** the tool states that the required data tile is not available offline, names the offline pack that contains it, and offers the EGM96 result with its lower accuracy stated if EGM96 is cached

### Requirement: Storage durability
The app SHALL request persistent storage when the user installs the PWA or downloads an offline pack, SHALL show current usage and quota, and SHALL warn users on browsers that evict storage after inactivity (e.g. Safari tabs after 7 days without a visit) that installing to the home screen keeps offline data.

#### Scenario: Safari tab warning
- **WHEN** a Safari (non-installed) user downloads an offline pack
- **THEN** a notice explains that Safari may remove offline data after 7 days of not visiting unless the app is added to the home screen

### Requirement: Predictable updates
A new release SHALL be fetched in the background and activated only on the next navigation or when the user accepts an "Update available" prompt. An update SHALL never change results mid-session without notice. The previous release's Wasm modules SHALL remain available until the new version activates.

#### Scenario: Update prompt
- **WHEN** a new version is available while the user has results on screen
- **THEN** a non-blocking prompt offers "Reload to update", and results on screen remain unchanged until the user reloads

### Requirement: Version transparency
The footer and settings SHALL show the app version, core version, and each loaded asset version, with a link to the changelog and the verification report for that version.

#### Scenario: Version shown
- **WHEN** a user opens settings
- **THEN** app, core, and asset versions are displayed

### Requirement: Cache hygiene
Old caches SHALL be deleted after a new version activates, except user-downloaded offline packs, which SHALL persist across versions unless their asset version is superseded, in which case the user SHALL be offered an update.

#### Scenario: Superseded pack
- **WHEN** a new release ships `wmm2030` superseding `wmm2025` in a user's magnetic pack
- **THEN** the pack manager marks the pack "update available" and does not delete the old pack until the user updates or removes it
