# packages/runtime

The internal, unpublished Wasm loader shared by the website and the MCP server. Neither surface calls a module any other way.

| File | What |
|---|---|
| `src/module.mjs` | Loads one module and wraps its raw ABI (`invoke`, `invokeBatch`, `manifest`, `version`). A Wasm trap becomes an `INTERNAL` error and the instance is recreated. Runs in browsers and Node. |
| `src/harden.mjs` | Rejects bad input before the core sees it: numbers that overflow, duplicate keys, nesting deeper than 32, strings over 1 MB, oversized payloads |
| `src/node.mjs` | Node host: lazy-loads modules from a directory and routes each tool id to its module |
| `src/runtime.test.mjs` | Runs every golden vector through Wasm in Node, plus hardening, batch, and unknown-id tests |

Not built yet: the browser worker host and asset providers (platform tasks 4.2 and 4.5).
