## Why

geoprims.com promises hundreds of spatial and aerospace tools that humans and AI agents can trust without an account, a server, or an ad. That promise only holds if every tool is built on one contract: the same inputs give the same outputs in every browser and in Node, units are explicit, accuracy is stated, and nothing the user types leaves the device. Without this foundation, 800 tools become 800 slightly different calculators, which is exactly what the market already has.

This change is phase 1. Every other change in `openspec/changes/` depends on it.

## What Changes

- Define the **tool contract**: the manifest every tool publishes (id, inputs, outputs, units, accuracy, references, test vectors, visualization hints, stability). The manifest is the single source of truth for the web UI, MCP server, docs, and search.
- Define the **compute core**: a WebAssembly core split by domain, with a stable call interface, a structured error model, and hard performance budgets.
- Define **numeric determinism**: bit-identical results across Chromium, WebKit, Gecko, and the Node.js runtime of the MCP server; explicit handling of NaN, infinities, signed zero, and angle wrapping.
- Define the **units and quantities** system: exact conversion constants (international foot, nautical mile, US survey foot kept only for legacy data), tagged inputs, canonical SI internals.
- Define **data assets**: how large models (geoid grids, magnetic coefficients, EPSG registry, terrain) are versioned, integrity-checked, lazy-loaded, cached offline, and licensed.
- Define **verification**: golden vectors from authoritative sources, cross-implementation checks, property-based tests, and published accuracy claims.
- Define **privacy and security**: no accounts, no cookies, no third-party requests carrying user input, strict Content Security Policy, reproducible and signed releases.
- Define the **tool catalog**: the taxonomy, id scheme, counting rule (operations vs endpoints), and lifecycle (experimental → stable → deprecated).

## Capabilities

### New Capabilities

- `platform/tool-contract`: The manifest schema and behavioral contract every tool satisfies.
- `platform/compute-core`: The WebAssembly compute runtime, call interface, error model, and performance budgets.
- `platform/numeric-determinism`: Floating-point, angle, and rounding rules that make results reproducible everywhere.
- `platform/units-and-quantities`: Unit system, exact constants, parsing of unit-tagged values, and output formatting.
- `platform/data-assets`: Versioned, integrity-checked, lazily loaded reference datasets and their licensing.
- `platform/verification`: Golden vectors, reference cross-checks, property tests, and accuracy disclosure.
- `platform/privacy-and-security`: No-account, no-tracking, local-only processing guarantees and supply-chain integrity.
- `platform/tool-catalog`: Taxonomy, identifiers, counting rules, and lifecycle for the full tool inventory.

### Modified Capabilities

None. This is a greenfield repository.

## Non-goals

- No user accounts, sync, saved workspaces in the cloud, or paid tiers.
- No server-side computation, proxies, or APIs hosted by geoprims.com.
- No live operational data feeds (weather, NOTAMs, ADS-B, airspace) in v1. Tools take these values as inputs.
- No certification for operational navigation use (DO-178C, DO-200B). Tools are engineering and planning aids.

## Impact

- Creates the repository layout, build pipeline, and release process that every later change uses.
- Establishes the Rust → WebAssembly toolchain and the manifest code generator.
- Introduces the reference-data pipeline (geoid, magnetic, EPSG, terrain tiles) and its hosting on the same static origin.
- Sets performance and privacy gates in CI that block releases.
