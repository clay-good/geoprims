## MODIFIED Requirements

### Requirement: One page anatomy
Every page SHALL have, in order: the site header, a page header built with the `PageHeader` component (breadcrumbs when the page has a parent, one `h1` in sentence case, an optional status badge, and one purpose line in plain words), the page body, and the site footer.

The site header SHALL contain exactly two things: on the left, the site title linking home with a one-line description of the site beside or beneath it; on the right, the light/dark control from `web/visual-theme`. It SHALL NOT be sticky, SHALL be separated from the page by a hairline rule, and SHALL carry no navigation links, no search field, and no other control.

The site footer SHALL carry the utility links (Units, All tools, Agents, Settings, Source), the safety disclaimer, the privacy line, and the version line.

Page types and their bodies:

| Type | Examples | Body |
|---|---|---|
| Home | `/` | Centered product description, one search field, the topic categories with counts and a link to `/tools/`, then recent and pinned tools when the browser has any |
| Catalog | `/tools/` | Filter field, then every operation grouped by category and group |
| Topic | `/aviation/` | Filter field, jump chips to each group, one card grid per group |
| Group | `/aviation/wind/` | Filter field, card grid |
| Tool | `/aviation/wind/runway-components/` | Safety note if operational, values and answer side by side (answer first on phones), diagram, map, "How we got this", "For developers and agents", related tools |
| Info | `/methodology/`, `/sources/`, `/settings/` | Readable text column (at most 46 rem), tables in `.table-scroll` |
| Not found | any unknown URL | What happened, the search field, and a few popular tools |

#### Scenario: New page follows the template
- **WHEN** a contributor adds a page
- **THEN** it uses `Base` and `PageHeader`, has exactly one `h1`, and its body uses the components listed under "Component vocabulary"

#### Scenario: Header stays minimal
- **WHEN** the template test parses any built page's site header
- **THEN** it finds the title, the description, and the theme control, and no other link or control

#### Scenario: Search is reachable without the header
- **WHEN** a visitor on any page presses `/`
- **THEN** the palette opens, as it did when the header carried a search button

## ADDED Requirements

### Requirement: The home page explains the product
The home page SHALL open with a centered description, at most three sentences, that says what the site computes, that every answer shows its formula and sources, and that everything runs on the visitor's device. Below it SHALL be one search field, then the topic categories with their tool counts and a link to the full catalog. The description SHALL name the domains it claims, and those names SHALL match the catalog's domains.

#### Scenario: A first-time visitor
- **WHEN** someone opens `/` for the first time
- **THEN** the first screen shows what the site is for, one search field, and the way to browse, with no personal or historical sections above them

#### Scenario: Claims match the build
- **WHEN** the claims gate compares the home description with the catalog
- **THEN** every domain it names exists in the catalog, and the privacy sentence matches the site's fetch policy
