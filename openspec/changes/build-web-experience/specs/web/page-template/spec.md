## Purpose

The page template every geoprims page is built from, so the site feels like one calm, self-service utility. Read this first when adding or changing a page: reuse the anatomy, patterns, and components below before inventing new ones. Colors, type, and modes come from `web/visual-theme`; this spec covers structure and behavior.

## ADDED Requirements

### Requirement: One page anatomy
Every page SHALL have, in order: the site header (brand, a search field that opens the palette, and at most five quiet links), a page header built with the `PageHeader` component (breadcrumbs when the page has a parent, one `h1` in sentence case, an optional status badge, and one purpose line in plain words), the page body, and the site footer. Page types and their bodies:

| Type | Examples | Body |
|---|---|---|
| Home | `/` | Question-style search with example chips, pinned and recent tools, popular tools with live answers, topic cards |
| Topic | `/aviation/` | Filter field, jump chips to each group, one card grid per group |
| Group | `/aviation/wind/` | Filter field, card grid |
| Tool | `/aviation/wind/runway-components/` | Safety note if operational, values and answer side by side (answer first on phones), diagram, map, "How we got this", "For developers and agents", related tools |
| Info | `/methodology/`, `/sources/`, `/settings/` | Readable text column (at most 46 rem), tables in `.table-scroll` |
| Not found | any unknown URL | What happened, the search field, and a few popular tools |

#### Scenario: New page follows the template
- **WHEN** a contributor adds a page
- **THEN** it uses `Base` and `PageHeader`, has exactly one `h1`, and its body uses the components listed under "Component vocabulary"

### Requirement: Always a way forward
No page or state SHALL be a dead end. Empty lists, failed filters, errors, unknown URLs, and results outside a tool's domain SHALL say what happened in one plain sentence and offer the next step: a search, the example, a related tool, or clearing a filter.

#### Scenario: Filter with no matches
- **WHEN** a visitor filters a topic page and no tool matches
- **THEN** the page says so and offers to search all tools for the same words

#### Scenario: Unknown URL
- **WHEN** a visitor opens a URL that does not exist
- **THEN** the not-found page explains it, offers search, and lists popular tools

### Requirement: Self-service tool pages
Tool pages SHALL let a visitor finish without help:
- Every input shows an example value on first load, a one-line help text, and the unit a plain number means, in readable units (°C, degrees).
- An input error names the field: the field is marked invalid with the message beneath it, and the answer card links to it.
- Rarely changed inputs sit under "More options", which opens by itself when one has a value.
- Each result row copies its value on click, and the answer card offers Copy, Copy sentence, Share link, and pin.
- Tools with unit-bearing results offer a unit profile switch in the answer card, which applies everywhere, like Settings.
- A tool with an inverse links to it from its header ("Go the other way").

#### Scenario: Field error is located
- **WHEN** a visitor enters latitude 95 in a tool
- **THEN** the latitude field is marked invalid with the message beneath it, and the answer card offers to jump to it

#### Scenario: Copy one result
- **WHEN** a visitor clicks a secondary result row
- **THEN** that row's value is copied and the row confirms it for a moment

### Requirement: Lists are findable
Any list of more than eight tools SHALL have a filter field that narrows it as the visitor types (title, summary, and aliases), shows how many match, and keeps the URL shareable (`?q=`). Topic pages SHALL also offer jump chips to each group.

#### Scenario: Shareable filter
- **WHEN** a visitor filters the aviation page for "wind" and shares the URL
- **THEN** the recipient sees the same filtered list

### Requirement: Component vocabulary
Pages SHALL reach for these before adding new styles: `.card` (surfaces), `.tools` (card grids of links; the whole card is the link), `.facts` (label and value rows), `.crumbs`, `.safety` (operational note), `.notice` (secondary text), `.chips` (small links or actions), `.filter` (list filter), `button.quiet` (every secondary action; at most one accent action per region), `.segmented` (a two- to four-way switch), `.table-scroll` (wide tables), and `details.card` (progressive disclosure). A new pattern SHALL be added to this spec and to `global.css` in the same change.

#### Scenario: No one-off styles
- **WHEN** the style lint scans a page's markup
- **THEN** it finds no inline `style` attribute (the CSP forbids them) and no color outside the tokens

### Requirement: Words
Copy SHALL use sentence case, plain words, and the reader's units: headings name what the reader gets ("Your values", "Popular"), buttons say what happens ("Share link", "Use the example"), and numbers carry units. Jargon is allowed only where the audience uses it (a pilot reads "density altitude"), and each tool's purpose line says what it is for in one sentence.

#### Scenario: Readable units
- **WHEN** a tool's input help or example value names a unit
- **THEN** it reads "°C" or "degrees", never "degC" or "deg"
