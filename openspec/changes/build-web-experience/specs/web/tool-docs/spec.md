## Purpose

Gives every geoprims endpoint a pre-rendered documentation page that teaches the math, cites its sources, states its accuracy and limits, and is discoverable by search engines and readable without JavaScript.

## ADDED Requirements

### Requirement: Standard documentation sections
Every tool page SHALL include these documentation sections, placed per the canonical anatomy in `contracts/page-chrome`: summary (one sentence), when to use it, inputs and outputs table (name, unit, range, meaning), formula or algorithm description with rendered math, the primary worked example (matching a golden vector), edge cases and limitations, accuracy statement, references with links, related tools, and the tool's changelog.

#### Scenario: Worked example matches vector
- **WHEN** the docs build renders a tool's worked example
- **THEN** the example's numbers are generated from a golden vector at build time and the build fails if they differ from the tool's output

### Requirement: Math rendered at build time
Formulas SHALL be rendered to static MathML (with an accessible text fallback) at build time, with no runtime math-rendering library required.

#### Scenario: Formula without JavaScript
- **WHEN** a tool page is loaded with JavaScript disabled
- **THEN** the formula is visible and readable by screen readers

### Requirement: Search-engine metadata
Titles, descriptions, canonical URLs, Open Graph images, structured data, and sitemaps SHALL follow `discovery/search-pages`, which is authoritative (titles from the shared head module, JSON-LD limited to the allowlist, OG images showing the tool name, domain, and example answer, sitemaps listing only indexable pages).

#### Scenario: Unique titles
- **WHEN** the SEO lint checks all pages
- **THEN** no two indexable pages share a title or canonical URL (non-indexable presets deliberately canonicalize to their parent)

### Requirement: Domain and group index pages
Each domain and group SHALL have an index page listing its tools with one-line summaries, grouped by task ("I want to…"), and linking to learning guides.

#### Scenario: Group index
- **WHEN** a user opens `/aviation/airspeed`
- **THEN** the page lists every airspeed tool with its summary and a short guide to IAS → CAS → EAS → TAS → Mach

### Requirement: Learning guides
The site SHALL publish the practitioner journeys listed in `plan-launch-and-value-proof` (design L3) as learning guides that chain tools end to end with prefilled steps. That list is the single source; home-page journey rows and hub links SHALL draw from it.

#### Scenario: Guide chain works
- **WHEN** a user follows "Drone mapping day" and clicks each step
- **THEN** each tool opens pre-filled with the previous step's outputs

### Requirement: Disclaimers in docs
Aviation, drone, and navigation tool docs SHALL include the operational disclaimer and SHALL date any regulatory values ("rules as of <date>", with the authority's link). Proposed rules SHALL be labeled "Proposed".

#### Scenario: Part 108 labeled
- **WHEN** any page references FAA Part 108 before a final rule is published
- **THEN** it is labeled "Proposed" with the Federal Register citation
