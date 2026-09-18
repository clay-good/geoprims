## Purpose

Defines the one canonical layout of every tool page (what appears, in what order, and how notices stack) so the answer is always visible at a glance, even on the smallest phone.

## ADDED Requirements

### Requirement: Canonical tool-page anatomy
Every tool page SHALL render these regions in order. This list is authoritative over `web/tool-docs`, `ux/glanceable-results`, and `discovery/search-pages`, which each contribute content to named regions.

1. **Header bar:**
   - left: logo and home link
   - middle: search button
   - right: theme toggle (sun/moon), "Report a problem", and the overflow menu (settings, units, audio when enabled, shortcuts)
2. **Title block:** H1 tool name, one-line purpose, and, for aviation, drone, and navigation tools, the one-line planning-aid notice.
3. **Notices:** at most two visible (see stacking).
4. **Answer card:** value, sentence, comparison, status phrase, and copy actions. This is the result panel. Its "Details" expander holds the secondary outputs, provenance, copy JSON, and copy as agent call.
5. **Inline diagram:** only for diagram-meaning tools.
6. **Core inputs:** with the "Example values" chip, Clear, and Try the example.
7. **More options:** collapsed, with worded defaults shown in the answer card.
8. **Canvas.**
9. **"How we got this":** show your work, worked example ("You enter / You get"), sources, assumptions, limitations, and status.
10. **About this tool:** when to use it, inputs and outputs table, edge cases, and accuracy. This carries the documentation sections and SEO prose.
11. **Related tools**, each with a reason.
12. **For developers and agents:** tool id, field names, and an example call.
13. **Footer:** GitHub "Wrong answer" link, changelog for this tool, last verified, version, and site links.

#### Scenario: Anatomy gate
- **WHEN** the anatomy gate parses every stable tool page
- **THEN** regions appear in this order, with exactly one report button (in the header)

### Requirement: Header at small widths
At 320 px width, the header SHALL show only the logo, the search button, "Report a problem" (icon plus the visually hidden label "Report a problem"), and the overflow menu, all with targets of at least 48 px. The theme toggle SHALL move into the overflow menu below 360 px. The header SHALL NOT wrap to two lines.

#### Scenario: 320 px header
- **WHEN** a tool page renders at 320 px
- **THEN** the header is one line, holds the four controls, and the report button's accessible name is "Report a problem"

### Requirement: Notice stacking and priority
Notices above the answer card SHALL be ranked:
1. confirmed known issue for this tool
2. out-of-domain, expired model, or non-official datum affecting the current result
3. regulation shown as "Proposed", or "Rules as of" older than 12 months
4. experimental tool
5. result changed in the last 90 days
6. limitation banner (simplified method)
7. model nearing expiry

At most the two highest-priority notices SHALL be shown in full. Any others SHALL collapse into a single line "N more notes", which expands on tap. Result-specific warnings SHALL appear inside the answer card, not as page notices. The answer card's value SHALL remain visible without scrolling at 390 × 844 with every notice present.

#### Scenario: Many notices
- **WHEN** a tool has a known issue, is experimental, has a recent result change, and uses a simplified method
- **THEN** the known-issue and experimental notices show in full, "2 more notes" collapses the others, and the answer value is visible at 390 × 844 without scrolling

### Requirement: Plain wording for chrome
Chrome text SHALL use sentence case and plain words ("Result out of date. Fix the highlighted field.", "Experimental: not yet fully verified"). No chrome element SHALL use all caps for emphasis.

#### Scenario: No all-caps
- **WHEN** the copy lint scans chrome strings
- **THEN** none consists of all-capital words (units and acronyms excepted)
