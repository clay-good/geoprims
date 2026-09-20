# tools/codegen

Generators that turn tool manifests into published files (design D5).

| Output | Status |
|---|---|
| `dist/catalog/v1.json` (`catalog.mjs`) | Built from the shipped Wasm manifests |
| `dist/catalog/types.d.ts` | Generated input and output types keyed by tool id |
| `dist/catalog/mcp-tools.json` | Generated direct-tool names, descriptions, and schemas |
| `dist/catalog/search.json` | Compact documents for the core search ranker, including prefill fields |
| Docs scaffolds | Not built yet; tool prose still needs a human author |

Run `npm run build` at the repository root to regenerate these files. They are build outputs and are not committed. `tools/codegen/artifacts.test.mjs` checks each output against the pinned sample manifest and checks the full catalog for schema and search parity.
