# tools

Build tooling. No npm dependencies.

| Path | What |
|---|---|
| `toolchain.json` | Pinned rustc and wasm-opt versions, and module size budgets |
| `wasm/build.mjs` | Builds every module to `dist/wasm/`, enforces pins, the import lint, and Brotli budgets, and writes SHA-256 digests |
| `wasm/*.test.mjs` | Gate tests against bad fixtures, and module smoke tests |
| `wasm/integrity.test.mjs` | The recorded digests match the modules on disk, and the website and the MCP release ship those exact bytes |
| `wasm/reproducible.mjs` | `npm run verify:reproducible`: builds twice from a cleared target directory and fails if any digest moves |
| `trust/profile.test.mjs` | The reference profile and docs/performance.md agree about the version, the viewports, and every budget |
| `diff/runner.mjs` | `npm run diff`: 10,000 seeded random and edge-biased cases per family (geodesic inverse and direct, the exact method on strongly flattened ellipsoids, rhumb inverse and direct, UTM, UPS, MGRS, local ENU, EGM96 geoid height, segment intersection, polygon area) through the shipped Wasm and GeographicLib's command-line tools, compared within declared tolerances, with failing cases written to `dist/diff/`. Families whose reference is not installed are skipped. `diff/runner.test.mjs` runs 300 cases each and checks that a perturbed tool fails |
| `perf/bench.mjs` | Benchmarks every tool's primary example in Node after warm-up, reports p50/p95 from 1,000 calls, and compares with a supplied same-profile release baseline |
| `trust/related.mjs` | The related-tools gate: lists resolve, give a recognised reason, agree about inverses, and stay within six; the count short of three is a ratchet |
| `trust/status.mjs` | The status-phrase gate: every `x-status` output must read as Within/Near/Beyond or Meets/Does not meet, and may never say safe, unsafe, legal, or approved |
| `mcp/eval.mjs` | The agent evaluation: 637 tasks from the worked examples, measuring tool selection, answers end to end, and tokens per task, with a 3-point regression gate |
| `repo/suites.test.mjs` | Holds the test scripts to every test file in the tree, and CI to those scripts, so a directory of tests cannot sit unrun |
| `codegen/catalog.mjs` | Builds `dist/catalog/v1.json` from the manifests the built modules report, with vector counts and operation and endpoint counts |
| `vectors/gen_units.py` | Generates the units golden vectors from the published definitions, using exact rational arithmetic |
| `vectors/gen_*.py` | One generator per domain (aviation, navigation, geodesy, drone, survey, time, sun, indexing, H3). `gen_sun.py` and `gen_h3.py` need `pvlib` and `h3` in a scratch virtualenv |
| `codegen/spcs83.py`, `vectors/gen_spcs_diff.py` | Generate the SPCS83 zone table from the EPSG dataset and the PROJ differential fixture and vectors (need `pyproj`) |
| `vectors/gen_magnetic.py` | Geomagnetism vectors: WMM2025 from the NCEI test values, IGRF-14 from `ppigrf` (scratch virtualenv) at coefficient epochs |
| `vectors/gen_h3_diff.py`, `vectors/gen_tz_diff.py` | Differential fixtures from H3 C (via h3-py) and Python zoneinfo; a small committed fixture runs in CI and a full local run is behind `--ignored` |
| `vectors/supersede.py` | Run after regenerating a published vector file: `supersede.py origin/main FILE "reason"` keeps each published line, marks changed ones `supersededBy`, and moves the new expectation to a fresh id. `supersede.py BASE --check` lists silent edits |
| `codegen/spa_tables.py`, `codegen/tzdb.py` | Regenerate the NREL SPA tables (from pvlib) and the embedded IANA tzdb blob (from tzdata plus a matching `zic -b slim`) |
| `vectors/vectors.test.mjs` | Checks that generated vectors reproduce byte for byte and that no published vector changed without a supersession record |
