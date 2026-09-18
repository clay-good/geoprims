## Purpose

Defines the manifest and behavioral contract that every geoprims tool publishes, so that the web UI, MCP server, documentation, and search index are all generated from one authoritative description and cannot drift apart.

## ADDED Requirements

### Requirement: Every tool publishes a manifest
Every tool SHALL publish a machine-readable manifest containing at minimum: `id`, `version`, `title`, `summary`, `domain`, `group`, `aliases`, `inputs`, `outputs`, `errors`, `accuracy`, `references`, `examples`, `vectors`, `assets`, `visualization`, `related`, `stability`, and `since`. A tool without a complete manifest SHALL NOT be included in a release build.

#### Scenario: Manifest is complete
- **WHEN** the build compiles the catalog
- **THEN** every tool's manifest validates against the manifest meta-schema and the build fails listing each tool and each missing field otherwise

#### Scenario: Manifest is the single source for all surfaces
- **WHEN** a tool's input description changes in the manifest
- **THEN** the web form label, MCP input schema description, and docs page all reflect the change in the same build with no other edit

### Requirement: Tool identifiers are stable and namespaced
A tool `id` SHALL match the pattern `^[a-z0-9]+(-[a-z0-9]+)*(\.[a-z0-9]+(-[a-z0-9]+)*){2}$` (exactly `<domain>.<group>.<operation>`) (for example `geodesy.utm.forward`, `aviation.airspeed.cas-to-tas`). Once a tool reaches `stable`, its `id` SHALL NOT be reused for a different operation and SHALL NOT be removed without passing through `deprecated` for at least one minor release, with a redirect to its replacement.

#### Scenario: Invalid id rejected
- **WHEN** a manifest declares id `Geodesy.UTM_Forward`
- **THEN** the build fails with an error naming the id and the required pattern

#### Scenario: Deprecated id resolves to replacement
- **WHEN** a user or agent invokes a deprecated tool id
- **THEN** the tool still executes, the response includes a `deprecation` notice naming the replacement id and removal version, and the web route redirects with an HTTP-equivalent client redirect to the replacement page

### Requirement: Inputs and outputs are typed with JSON Schema 2020-12 plus unit annotations
Tool `inputs` and `outputs` SHALL be expressed as JSON Schema draft 2020-12 objects. Every numeric field that represents a physical quantity SHALL carry an `x-quantity` annotation (e.g. `length`, `distance`, `angle`, `speed`, `pressure`, `temperature`, `mass`, `energy`, `power`, `time`, `area`, `volume`, `dimensionless`) and an `x-unit` annotation naming the canonical unit of the value as returned. Every angular field SHALL declare its range convention (`x-angle-range`: `[-180,180)`, `[0,360)`, `[-90,90]`, or `unbounded`).

#### Scenario: Quantity annotation required
- **WHEN** a manifest declares numeric input `height` without `x-quantity`
- **THEN** the build fails naming the tool and field

#### Scenario: Azimuth range is declared and honored
- **WHEN** a tool declares output `azimuth` with `x-angle-range: [0,360)` and the computed value is -0.0 or 360.0 degrees
- **THEN** the returned value is exactly `0`

### Requirement: Tool invocation is a pure function of declared inputs and declared assets
A tool invocation SHALL depend only on its validated inputs, the versions of the data assets it declares in `assets`, and the core version. Tools SHALL NOT read the clock, locale, time zone, random sources, or network, except that a tool whose manifest sets `x-clock-default: allowed` MAY accept a date/epoch as an explicit input with the host-supplied current date as its default, echoed back in the result. Tools with `x-clock-default: forbidden` (the default; e.g. weather decoding, night currency) SHALL require the date or time as an explicit input.

#### Scenario: Implicit date is echoed
- **WHEN** a magnetic declination tool is called without an `epoch` input
- **THEN** the tool uses the documented default (the current UTC date supplied by the host at call time), and the result includes `epoch` with the exact decimal year used

#### Scenario: Same inputs produce same outputs
- **WHEN** the same tool is invoked twice with identical inputs, identical asset versions, and identical core version
- **THEN** the two results are byte-identical when serialized

### Requirement: Results carry provenance
Every result SHALL be an envelope: `{"ok": true, "result": {…}, "summary": "…", "meta": {…}}` on success (`summary` is the rendered `x-sentence`), or `{"ok": false, "error": {"code", "message", "field"?, "hint"?}}` on failure. Every quantity output SHALL be an object `{"value": <number>, "unit": "<symbol>"}`. Callers pass per-call options in the reserved input key `options` (`profile`, `outputUnits`, `numberFormat`). Every successful result SHALL include a `meta` object containing: `tool` (id), `toolVersion`, `coreVersion`, `assets` (id and version of each dataset used), `model` (named algorithm or model, e.g. `Karney 2013 geodesic`, `WMM2025`), `accuracy` (a statement or numeric bound for this call), and `warnings` (a possibly empty array of structured warnings).

#### Scenario: Provenance present
- **WHEN** any tool returns a result
- **THEN** `meta.tool`, `meta.toolVersion`, `meta.coreVersion`, `meta.model`, and `meta.accuracy` are present and non-empty

#### Scenario: Warning for degraded accuracy
- **WHEN** a geoid height is requested at a location where the loaded grid resolution yields a stated interpolation error larger than 0.1 m
- **THEN** `meta.warnings` contains an entry with code `ACCURACY_DEGRADED` and a human-readable message

### Requirement: Structured error model
A failed invocation SHALL return a structured error with `code` (from a closed, documented enumeration), `message` (plain-language, actionable), `field` (JSON Pointer to the offending input when applicable), and `hint` (optional corrective suggestion). The enumeration SHALL include at least: `INVALID_INPUT`, `OUT_OF_DOMAIN`, `UNIT_MISMATCH`, `DID_NOT_CONVERGE`, `DEGENERATE_GEOMETRY`, `ASSET_UNAVAILABLE`, `ASSET_INTEGRITY`, `LIMIT_EXCEEDED`, `NO_SOLUTION` (the problem is well-formed but has no answer, e.g. wind stronger than airspeed), `UNSUPPORTED` (unknown or unavailable tool), and `INTERNAL`. Tools SHALL NOT return `NaN`, `Infinity`, or partial results in place of an error.

#### Scenario: Out-of-domain input
- **WHEN** a UTM forward conversion is requested for latitude 85° N
- **THEN** the tool returns error `OUT_OF_DOMAIN` with `field` `/lat`, a message stating UTM covers 80° S to 84° N, and a hint pointing to `geodesy.ups.forward`

#### Scenario: No silent NaN
- **WHEN** any computation internally produces NaN
- **THEN** the tool returns error `INTERNAL` or a more specific code, never a result containing NaN

### Requirement: Each tool declares its visualization
Every manifest SHALL declare a `visualization` descriptor selecting one or more canvas layer kinds (for example `point`, `line-geodesic`, `line-rhumb`, `polygon`, `bbox`, `cell-set`, `vector-diagram`, `profile-chart`, `gauge`, `table-only`) and a mapping from output fields to layer inputs. `table-only` SHALL be permitted only for tools with no spatial or vector meaning.

#### Scenario: Visualization mapping validated
- **WHEN** a manifest maps layer input `path` to output field `route` that does not exist in its output schema
- **THEN** the build fails naming the tool and the missing field

### Requirement: Each tool cites references and states accuracy
Every manifest SHALL cite at least one authoritative reference (standard, peer-reviewed paper, or agency publication) with title, author or issuing body, year, and a stable URL or DOI where available, and SHALL state accuracy as either a numeric bound with unit or a qualified statement (for example "exact to double precision", "±0.5° RMS globally, model uncertainty per WMM2025 technical report").

#### Scenario: Missing reference blocks release
- **WHEN** a tool has no reference with an issuing body and year
- **THEN** the release build fails naming the tool

### Requirement: Batch invocation
Every tool whose inputs are all scalar or small-structured SHALL support batch invocation over an array of input records, returning an array of per-record results or errors in the same order. A single failing record SHALL NOT fail the whole batch.

#### Scenario: Mixed batch
- **WHEN** a batch of 3 records is submitted where record 2 has latitude 120°
- **THEN** records 1 and 3 return results, record 2 returns `INVALID_INPUT` with `field` `/lat`, and the output array has length 3 in input order

### Requirement: Declared limits
Every tool SHALL declare numeric limits on input size (for example maximum polygon vertices, maximum H3 cells produced, maximum batch rows) and SHALL return `LIMIT_EXCEEDED` with the limit value before allocating memory proportional to an over-limit request.

#### Scenario: Polyfill limit
- **WHEN** a polygon-to-H3 request would produce an estimated 50,000,000 cells and the declared limit is 5,000,000
- **THEN** the tool returns `LIMIT_EXCEEDED` citing the estimate and the limit, within 50 ms, without attempting the fill

### Requirement: Warning code registry
Every warning code SHALL be defined once in a warning registry with its meaning, severity (`info`, `caution`, `accuracy`), and the tools that may emit it. The build SHALL fail if a tool emits or documents a warning code that is not registered, and the registry SHALL be published on the site.

#### Scenario: Unregistered warning
- **WHEN** a tool emits `SOME_NEW_WARNING` that is not in the registry
- **THEN** the build fails naming the tool and code

### Requirement: Epoch and date conventions
Dates SHALL be accepted as ISO 8601 (UTC) or decimal years. Decimal year SHALL be defined as `year + (dayOfYear − 1 + secondsOfDay/86400) / daysInYear` in UTC, with `daysInYear` 366 in leap years. When a tool defaults its epoch to "now", the echoed epoch SHALL be pinned into permalinks, exported calculation sheets, and "copy as agent call" output, so that re-running them later reproduces the original result.

#### Scenario: Decimal year
- **WHEN** the date 2026-07-02T12:00:00Z is converted
- **THEN** the decimal year is 2026 + 182.5/365 ≈ 2026.5

#### Scenario: Pinned default epoch
- **WHEN** a user copies a permalink to a declination computed with the default epoch
- **THEN** the permalink contains the explicit epoch used, and opening it a year later gives the same result
