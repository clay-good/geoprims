## Purpose

Lets people and agents type a question the way they think it, for example "crosswind rwy 27 wind 300 at 15" or "gsd 13.2mm sensor 8.8mm lens 100 m 5472 px". They get the right tool, already filled in and computed.

## ADDED Requirements

### Requirement: Shared, deterministic query parser
A single query parser, compiled into the core and used by both the web palette and the MCP server, SHALL:
1. normalize the query: case, unit spellings via a synonym table, and stopwords
2. rank tools by weighted fields (name, aliases, keywords, description), with prefix stemming and single-edit typo tolerance
3. extract quantities with units (`5000 ft`, `30C`, `29.80`, `A2980`, `rwy 27`, `300@15`, `13.2mm`, coordinates in any notation)
4. map the quantities to the top tool's inputs using per-tool slot definitions (which input accepts which units, and the order hints)

The same query SHALL yield the same ranking and prefill on both surfaces.

#### Scenario: Density altitude from text
- **WHEN** the query is `density altitude 5000 ft 30C 29.80`
- **THEN** the top tool is density altitude, with elevation 5,000 ft, OAT 30 °C, and altimeter 29.80 inHg filled in, and the palette offers "Open with these values"

#### Scenario: Surface parity
- **WHEN** the same query is sent to MCP `geoprims_search`
- **THEN** the top result and its `prefill` object equal the web palette's

### Requirement: Ambiguity is surfaced, not guessed
When extracted quantities could fill more than one input (for example two temperatures), the parser SHALL fill only unambiguous slots. It SHALL list the ambiguous values with their candidate fields, and the UI SHALL ask the user to place them.

#### Scenario: Two temperatures
- **WHEN** the query is `density altitude 30 20 29.92 5000`
- **THEN** elevation and altimeter are filled, and the UI asks which of 30 and 20 is OAT and which is dew point

### Requirement: Slot definitions per tool
Each tool manifest SHALL define prefill slots: for each input, the accepted quantity types, unit hints, keywords (e.g. `oat`, `temp`, `dew`), and a default order. The build SHALL verify that every slot names a real input.

#### Scenario: Invalid slot
- **WHEN** a slot names a non-existent input
- **THEN** the build fails naming the tool and slot

### Requirement: Measured ranking quality
The repository SHALL keep a query fixture of at least 500 real-world phrasings, drawn from practitioner vocabulary and the Search Console log, each with the expected tool and prefill. CI SHALL report top-1 and top-3 accuracy and prefill accuracy, and SHALL fail on a regression of more than 2 percentage points.

#### Scenario: Ranking regression
- **WHEN** an alias change drops top-3 accuracy from 96% to 93%
- **THEN** CI fails, listing the queries that changed
