## Purpose

Defines the layered checks every tool passes before it is called stable, so correctness comes from process rather than hope, and so the site never claims more verification than actually happened.

## ADDED Requirements

### Requirement: Correctness layers
A tool SHALL pass all of these layers before promotion to `stable`:
- **(A) Derivation.** A written derivation or method note in `docs/derivations/<tool>.md` that states the equations, symbols, units, domain, and approximations.
- **(B) Worked examples.** Traced to published sources (see the worked-example requirement).
- **(C) Dimension annotations.** Every input and output carries dimension annotations that the dimension lint checks, including unit-suffixed names.
- **(D) Bounds fuzzing.** Random and extreme inputs within and just outside the declared domain produce either a valid result or a structured error, never NaN, a hang, or a trap.
- **(E) Invariants and round trips.** Per `platform/verification`.
- **(F) Differential tests.** Against reference implementations where they exist.
- **(G) Example parity.** The page example, the "Try the example" button, and the MCP example produce identical results.

#### Scenario: Missing derivation blocks promotion
- **WHEN** a tool with passing vectors lacks `docs/derivations/<tool>.md`
- **THEN** promotion to stable fails naming the missing layer

#### Scenario: Fuzz finds a trap
- **WHEN** the bounds fuzzer finds an input that traps the Wasm module
- **THEN** CI fails with the minimized input, and the tool stays experimental until fixed

### Requirement: Worked examples traced to a source
Each worked example SHALL record:
- `sourcePublisher`, `sourceTitle`, `sourceEdition`, and `sourceLocator` (section, page, table, or example number)
- `verifiedBy` (maintainer or reviewer handle) and `verifiedOn`
- the inputs, and the outputs with a tolerance per field

Tolerances above a per-domain ceiling (e.g. 0.1% for aviation performance, 1 mm for geodesy) SHALL require a written justification. At least one worked example per tool SHALL come from a source independent of geoprims (a standard's own example, an agency tool output, or a textbook problem).

#### Scenario: Independent example required
- **WHEN** a tool's only worked examples were computed by geoprims itself
- **THEN** promotion fails with "needs an independent worked example"

### Requirement: Practitioner review sign-off, disclosed honestly
Each domain SHALL have a review record in `docs/review-signoffs.md` naming the reviewer's relevant qualification (e.g. licensed surveyor, CFI, Part 107 remote pilot, geodesist), the tools reviewed, the date, and the scope. Sign-offs SHALL be renewed at least every 12 months, or when a tool's result changes. Where no sign-off exists, domain pages SHALL state "Not yet independently reviewed by a <practitioner>", and no page SHALL imply review that has not happened.

#### Scenario: Unreviewed domain disclosed
- **WHEN** the survey domain has no sign-off at launch
- **THEN** the survey index and each survey tool's proof panel state that independent practitioner review is pending

### Requirement: Claims honesty gate
A CI gate SHALL check that public claims match the build:
- tool counts on the home page and README
- "verified against" statements, which must match the differential suites actually run
- review statements, which must match the sign-off records
- accuracy statements, which must match the manifests

#### Scenario: Overclaim caught
- **WHEN** the README says "every tool is checked against GeographicLib" but only geodesic and projection tools are
- **THEN** the claims gate fails quoting the sentence

### Requirement: Both surfaces reachable
For every stable tool, CI SHALL check both surfaces:
- **Web:** the tool's own name ranks in the top 5 of the palette search.
- **MCP:** `geoprims_search` finds it in the top 5, `geoprims_describe` advertises every input its example uses, and `geoprims_run` executes the example cleanly.

#### Scenario: Unreachable tool
- **WHEN** a tool's name does not appear in its own search top 5
- **THEN** CI fails, prompting alias or ranking work
