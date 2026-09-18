# tools

Build tooling. No npm dependencies.

| Path | What |
|---|---|
| `toolchain.json` | Pinned rustc and wasm-opt versions, and module size budgets |
| `wasm/build.mjs` | Builds every module to `dist/wasm/`, enforces pins, the import lint, and Brotli budgets, and writes SHA-256 digests |
| `wasm/*.test.mjs` | Gate tests against bad fixtures, and module smoke tests |
| `codegen/catalog.mjs` | Builds `dist/catalog/v1.json` from the manifests the built modules report, with vector counts and operation and endpoint counts |
| `vectors/gen_units.py` | Generates the units golden vectors from the published definitions, using exact rational arithmetic |
| `vectors/gen_*.py` | One generator per domain (aviation, navigation, geodesy, drone, survey, time, sun, indexing, H3). `gen_sun.py` and `gen_h3.py` need `pvlib` and `h3` in a scratch virtualenv |
| `vectors/gen_h3_diff.py`, `vectors/gen_tz_diff.py` | Differential fixtures from H3 C (via h3-py) and Python zoneinfo; a small committed fixture runs in CI and a full local run is behind `--ignored` |
| `vectors/supersede.py` | Run after regenerating a published vector file: `supersede.py origin/main FILE "reason"` keeps each published line, marks changed ones `supersededBy`, and moves the new expectation to a fresh id. `supersede.py BASE --check` lists silent edits |
| `codegen/spa_tables.py`, `codegen/tzdb.py` | Regenerate the NREL SPA tables (from pvlib) and the embedded IANA tzdb blob (from tzdata plus a matching `zic -b slim`) |
| `vectors/vectors.test.mjs` | Checks that generated vectors reproduce byte for byte and that no published vector changed without a supersession record |
