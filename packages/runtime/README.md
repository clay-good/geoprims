# packages/runtime

The internal, unpublished Wasm loader shared by the website and the MCP server. Neither surface calls a module any other way.

| File | What |
|---|---|
| `src/module.mjs` | Loads one module and wraps its raw ABI (`invoke`, `invokeBatch`, `manifest`, `version`). A Wasm trap becomes an `INTERNAL` error and the instance is recreated. Runs in browsers and Node. |
| `src/harden.mjs` | Rejects bad input before the core sees it: numbers that overflow, duplicate keys, nesting deeper than 32, strings over 1 MB, oversized payloads |
| `src/node.mjs` | Node host: lazy-loads modules from a directory and routes each tool id to its module |
| `src/worker-host.mjs` | Node worker host for MCP calls. It enforces timeouts and accepts an optional `AbortSignal` and elapsed-time progress callback on `invoke` and `invokeBatch`. An aborted call resolves to `null`; the worker restarts and queued calls continue. |
| `src/assets.mjs` | Verifies dataset digests before providing bytes to a module; Node and browser hosts use the same integrity check |
| `src/runtime.test.mjs` | Runs every golden vector through Wasm in Node, plus hardening, batch, and unknown-id tests |

The site and MCP release integrity gates check every shipped module's SHA-256 digest against the build. `apps/web/test/browser/determinism.test.mjs` compares exact result bytes for every live golden vector across Chromium, Firefox, WebKit, and Node, including asset-backed calculations. The Wasm import lint rejects host `Math` calls before a module can reach any surface.

The browser worker host is in `apps/web/src/lib/compute.worker.js`. The web client and Node worker host can interrupt a running invocation by replacing its worker; the MCP stdio server forwards progress and cancellation. Core-level cooperative cancellation remains open under platform task 4.4.
