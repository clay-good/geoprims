# packages/runtime

The internal, unpublished Wasm loader shared by the website and the MCP server. Neither surface calls a module any other way.

| File | What |
|---|---|
| `src/module.mjs` | Loads one module and wraps its raw ABI (`invoke`, `invokeBatch`, `manifest`, `version`). A Wasm trap becomes an `INTERNAL` error and the instance is recreated. Runs in browsers and Node. |
| `src/harden.mjs` | Rejects bad input before the core sees it: numbers that overflow, duplicate keys, nesting deeper than 32, strings over 1 MB, oversized payloads |
| `src/node.mjs` | Node host: lazy-loads modules from a directory and routes each tool id to its module |
| `src/worker-host.mjs` | Node worker host for MCP calls. It enforces timeouts and accepts an optional `AbortSignal` and elapsed-time progress callback on `invoke` and `invokeBatch`. An aborted call resolves to `null`; the worker restarts and queued calls continue. |
| `src/assets.mjs` | Verifies dataset digests and signed tile indexes before providing bytes to a module; Node and browser hosts use the same integrity and pinned-key checks. The browser host evicts corrupt cached bytes and stores a verified replacement for offline use; the Node host reads packaged files without a cache |
| `src/runtime.test.mjs` | Runs every golden vector through Wasm in Node, plus hardening, batch, and unknown-id tests |

The site and MCP release integrity gates check every shipped module's SHA-256 digest against the build. `apps/web/test/browser/determinism.test.mjs` compares exact result bytes for every live golden vector across Chromium, Firefox, WebKit, and Node, including asset-backed calculations. The Wasm import lint rejects host `Math` calls before a module can reach any surface.

The browser worker host is in `apps/web/src/lib/compute.worker.js`. The web client and Node worker host interrupt a running invocation by terminating and replacing its worker; the MCP stdio server forwards progress and cancellation. Both hosts test this boundary with a real long H3 core call and require the canceled call to settle within 100 ms without a partial result.

Closing the Node worker host cancels running and queued calls with `null`, clears their timers and abort listeners, and waits for worker termination. Later calls return `null`; repeated `close()` calls share the same termination promise. `src/shutdown.test.mjs` verifies shutdown without requiring built Wasm modules.
