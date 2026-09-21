## MODIFIED Requirements

### Requirement: One page anatomy
Every page SHALL have, in order: the site header, a page header built with the `PageHeader` component, the page body, and the site footer.

The site header SHALL contain: on the left, the site mark and title linking home, with a one-line description beside them on wide screens; on the right, a search button that opens the command palette and the light/dark control from `web/visual-theme`. It SHALL be sticky, SHALL be separated from the page by a hairline rule, and SHALL carry no navigation links and no other control.

Page types and their bodies:

| Type | Examples | Body |
|---|---|---|
| Home | `/` | A decorative terrain hero with the headline and one search field that answers as the reader types and opens the best match with the question's values on Enter; example questions; an instrument panel of worked-example answers; the topics as tiles with counts and a link to `/tools/`; then recent and pinned tools when the browser has any |
| Tool | `/aviation/wind/runway-components/` | Safety note if operational, notices, the answer and the values (answer first on phones), diagram, map, "How we got this", related tools |

#### Scenario: Header holds search and the toggle
- **WHEN** the template test parses any built page's site header
- **THEN** it finds the mark and title, the search button, and the theme control, and no other link or control

#### Scenario: Question opens the filled calculator
- **WHEN** a visitor types "crosswind rwy 27 wind 300 at 15" on the home page and presses Enter
- **THEN** the runway wind components tool opens with runway 27, wind 300°, and 15 kt filled in and the answer computed

#### Scenario: Tool page carries no developer material
- **WHEN** the anatomy gate parses every tool page
- **THEN** none prints a tool id block, field-name table, or `geoprims_run` call, and each has exactly one "How we got this" panel
