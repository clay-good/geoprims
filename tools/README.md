# tools

Build tooling. No npm dependencies.

| Path | What |
|---|---|
| `toolchain.json` | Pinned rustc and wasm-opt versions, and module size budgets |
| `wasm/build.mjs` | Builds every module to `dist/wasm/`, enforces pins, the import lint, and Brotli budgets, and writes SHA-256 digests |
| `wasm/*.test.mjs` | Gate tests against bad fixtures, and module smoke tests |
| `codegen/catalog.mjs` | Builds `dist/catalog/v1.json` from the manifests the built modules report, with vector counts and operation and endpoint counts |
| `vectors/gen_units.py` | Generates the units golden vectors from the published definitions, using exact rational arithmetic |
| `vectors/vectors.test.mjs` | Checks that generated vectors reproduce byte for byte and that no published vector changed without a supersession record |
