## ADDED Requirements

### Requirement: The catalog page
The app SHALL pre-render a page at `/tools/` that lists every operation in the catalog, grouped by category and then by group, each linking to its tool page. The page SHALL carry the same filter field the topic pages use, SHALL state the total number of operations and how many are stable, SHALL emit an `ItemList` structured-data listing, and SHALL appear in the sitemap. It SHALL be complete without JavaScript.

#### Scenario: Everything is there
- **WHEN** the build renders `/tools/`
- **THEN** the page links to every operation in the catalog, each exactly once

#### Scenario: Works without JavaScript
- **WHEN** `/tools/` is loaded with JavaScript disabled
- **THEN** every category, group, and tool link is present and navigable, and only the filter field is inert

## MODIFIED Requirements

### Requirement: Settings
The settings page SHALL hold the unit profile, number format, and local-data controls (clear recent tools, erase all local data), and SHALL say that settings are saved in this browser only. The display mode SHALL NOT be a setting on this page: it is the header control, per `web/visual-theme`.

#### Scenario: Settings apply immediately
- **WHEN** a visitor changes the unit profile
- **THEN** every tool opened afterwards shows results in that profile, and a unit chosen inside a tool still wins

#### Scenario: No display control here
- **WHEN** the settings page renders
- **THEN** it offers no display-mode select and points to the header control instead
