# tools

Build tooling. No npm dependencies.

| Path | What |
|---|---|
| `toolchain.json` | Pinned rustc and wasm-opt versions, and module size budgets |
| `wasm/build.mjs` | Builds every module to `dist/wasm/`, enforces pins, the import lint, and Brotli budgets, and writes SHA-256 digests |
| `wasm/*.test.mjs` | Gate tests against bad fixtures, and module smoke tests |
| `codegen/` | Manifest generators (not built yet) |
