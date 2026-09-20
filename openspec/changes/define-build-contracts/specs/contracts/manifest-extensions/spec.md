## Purpose

Lists every extension field a tool manifest may carry and defines the small languages and rules behind them (sentence templates, glossary entries, examples, steps), so authors, the core, the web app, and the MCP server all agree.

## ADDED Requirements

### Requirement: Closed set of manifest extensions
The manifest meta-schema SHALL define, and accept only, these extension fields.

**Per field:**
- `x-quantity`, `x-unit`, `x-angle-range`, `x-display-precision`
- `x-step` (`{small, large}` in the field's default unit)
- `x-private` (boolean: never included in reports or permalinks)
- `x-core` (boolean: shown by default, at most 5 per tool, where a list of rows counts once and a coordinate's two fields count as one point)
- `x-help` (one-line help with an example value)
- `x-prefill` (slot definition: accepted quantities, unit hints, keywords, order)
- `x-swappable-with` (field name)
- `x-status` (outputs only: `{kind: "threshold" | "conformance", source}`, where `source` is a sources-ledger id or `user` when the reader enters the limit)

**Per tool:**
- `x-sentence` (template), `x-comparison` (`{kind, text}`: kind is `vs-input`, `vs-rule-of-thumb`, `vs-typical-range`, or `none`; `text` is a sentence template rendered from the same scope, and is present exactly when the kind is not `none`)
- `x-near-margin` (fraction or absolute, default 0.10)
- `x-clock-default` (`allowed` or `forbidden`, default `forbidden`)
- `x-primary-example` (vector id)
- `x-limitation` (`{simplification, instead, governs}`, capped at 80, 240, and 120 characters), `x-glossary-terms` (term ids)
- `x-related` (`[{id, reason}]`), `x-diagram-inline` (boolean)
- `x-high-intent` (boolean, generated endpoints only)

Unknown `x-` fields SHALL fail the build.

#### Scenario: Unknown extension
- **WHEN** a manifest adds `x-color`
- **THEN** the meta-schema validation fails naming the field

#### Scenario: Too many core inputs
- **WHEN** a manifest marks 6 fields `x-core: true`
- **THEN** the build fails (row groups such as W&B stations count as one, and a lat/lon pair counts as one point)

### Requirement: Sentence-template language
Sentence templates SHALL use this language, rendered by the core:
- `{field}` or `{field:unit}` inserts a value, formatted with the field's display precision in the user's unit profile, or in a forced unit.
- `{delta(a,b)}` and `{abs(x)}` are the only arithmetic helpers.
- `{if <field> <op> <value>}…{else}…{/if}` handles conditions (ops `< <= > >= == !=`).
- `{warn CODE}…{/warn}` includes text only when a warning is present.
- `{plural n "one" "many"}` handles plurals.

Numbers SHALL be grouped with the user's number format, and rounding SHALL happen only at render. Rendered sentences SHALL be at most 280 characters and pass the grade-8 readability lint on every worked example.

#### Scenario: Conditional clause
- **WHEN** density altitude is below field elevation
- **THEN** the template's `{if delta < 0}` branch renders "lower than the field" instead of "higher than the field"

#### Scenario: Unit profile
- **WHEN** the unit profile is SI
- **THEN** `{da}` renders in meters with SI grouping

### Requirement: Glossary schema
Glossary entries SHALL have: `id`, `term`, `expansion`, `definition` (≤ 40 words, plain language), `source` (citation id), and `relatedTools`. Every abbreviation used in a tool's title, labels, help, or sentence SHALL resolve to an entry.

#### Scenario: Missing glossary term
- **WHEN** a label uses "RPP" and no glossary entry has term RPP
- **THEN** the build fails naming the tool and term

### Requirement: The primary worked example
Each operation SHALL designate one primary example (`x-primary-example`). It SHALL be the independent, source-traced worked example where one exists, and it SHALL be used everywhere an example appears: the prefilled first load, "Try the example", the "You enter / You get" block, the OG image, the hero card, the MCP default run, and the explainer embed. High-intent generated pages SHALL have their own primary example drawn from a vector of the composed chain. Non-indexable generated endpoints and alias slugs SHALL use the parent's example with the preset applied.

#### Scenario: Example parity
- **WHEN** the example-parity gate compares the page, button, OG image text, and MCP default run for a tool
- **THEN** all show the same inputs and outputs

### Requirement: Decimal separator rule
Numeric parsing SHALL follow the number-format setting:
- **Decimal-point mode** (US default): `.` is the decimal separator. `,` is accepted only as a thousands separator in valid 3-digit groups (`1,250` = 1250; `12,345.6` = 12345.6). Any other comma (`1,25`) is rejected with a hint to switch modes.
- **Decimal-comma mode:** the roles are reversed, with `.` or a space as the thousands separator.

The rule is identical in the web app, the MCP server, and the query parser.

#### Scenario: US default
- **WHEN** `1,250` is entered in decimal-point mode
- **THEN** it is read as 1250

#### Scenario: Ambiguous comma
- **WHEN** `1,25` is entered in decimal-point mode
- **THEN** it is rejected with "Did you mean 1.25? Switch to decimal-comma format in settings"
