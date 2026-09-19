## Purpose

Lets a user reach any of the roughly 830 geoprims tools, or act on a value they paste, in a few keystrokes through a fuzzy command palette and a consistent keyboard model.

## ADDED Requirements

### Requirement: Palette opens instantly
Pressing `/` (when focus is not in a text field) or `Ctrl+K` / `Cmd+K` (anywhere) SHALL open the command palette with focus in its search field within 50 ms on the reference device. `Esc` SHALL close it and restore prior focus.

#### Scenario: Slash opens palette
- **WHEN** a user presses `/` on any page with no text field focused
- **THEN** the palette opens with the cursor in the search field

#### Scenario: Slash in a text field
- **WHEN** a user presses `/` while typing in an input
- **THEN** a `/` character is typed and the palette does not open

### Requirement: Fast fuzzy search over the catalog
The palette SHALL search tool titles, ids, aliases, keywords, and group names with typo-tolerant fuzzy matching, returning ranked results within 16 ms per keystroke for a catalog of 1,000 endpoints on the reference device. Ranking SHALL weight exact alias matches, prefix matches, recency, and pinned status.

#### Scenario: Typo tolerance
- **WHEN** a user types `densty alt`
- **THEN** `aviation.altimetry.density-altitude` is the first result

#### Scenario: Abbreviation
- **WHEN** a user types `tas`
- **THEN** the top results are true-airspeed tools

#### Scenario: Latency
- **WHEN** the palette benchmark types 20 queries of 1–20 characters against 1,000 entries
- **THEN** the p95 time from keystroke to rendered results is at most 16 ms

### Requirement: Paste-to-detect
When the palette query parses as a recognizable value (a coordinate in any supported notation, an H3 index, an S2 token, a geohash, a quadkey, an XYZ tile `z/x/y`, a Maidenhead locator, an MGRS string, a METAR-style altimeter `A2992`/`Q1013`), the palette SHALL show a "Detected: <type>" section with the value's interpretation and the most relevant tools pre-filled with it.

#### Scenario: H3 index detected
- **WHEN** a user pastes `8928308280fffff`
- **THEN** the palette shows "Detected: H3 cell, resolution 9" and offers "Cell boundary", "Cell to lat/lon", "k-ring neighbors" with the index pre-filled

#### Scenario: Ambiguous detection
- **WHEN** a user pastes `9q8yy` (valid geohash and possibly other encodings)
- **THEN** the palette lists every plausible interpretation, geohash first, with its decoded location

### Requirement: Keyboard-only operation
Every action in the app SHALL be reachable by keyboard. The palette SHALL support arrow keys and `Ctrl+N`/`Ctrl+P` to move, `Enter` to open, `Ctrl+Enter`/`Cmd+Enter` to open in a new tab, and `Tab` to pin. A `?` key SHALL show a shortcut reference sheet.

#### Scenario: Shortcut sheet
- **WHEN** a user presses `?` outside a text field
- **THEN** an overlay lists all global and page shortcuts

### Requirement: Global shortcuts
The app SHALL provide at least these shortcuts outside text fields: `/` palette, `g h` home, `m` toggle audio mute, `c` cycle canvas mode (2D map, 3D globe, vector), `u` cycle unit profile, `y` copy result JSON, `l` copy permalink, `s` swap the two primary inputs where the tool declares them swappable, `p` play or pause an animated scene, `[` and `]` previous/next tool in group. Shortcuts SHALL be listed in the shortcut sheet, SHALL NOT override browser or screen-reader reserved keys, and single-character shortcuts SHALL be switchable off or remappable in settings (WCAG 2.1.4). `m` (mute) SHALL act only when audio has been enabled.

#### Scenario: Mute toggle
- **WHEN** audio is on and the user presses `m`
- **THEN** audio mutes immediately and a visual toast confirms "Audio muted"

### Requirement: Actions in the palette
The palette SHALL also expose app actions (change theme, change unit profile, toggle audio, erase local data, download offline pack, open settings), prefixed with `>` in the query to restrict results to actions.

#### Scenario: Action mode
- **WHEN** a user types `>theme`
- **THEN** only theme-related actions are listed

### Requirement: Accessible palette
The palette SHALL implement the WAI-ARIA combobox pattern with a listbox of results, announce the result count to screen readers, and keep a visible focus indicator.

#### Scenario: Screen reader announcement
- **WHEN** a screen-reader user types a query
- **THEN** the number of results is announced via a polite live region
