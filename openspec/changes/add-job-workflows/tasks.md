# Tasks

- [ ] 1 Move the chain runner to `packages/runtime/src/chain.mjs`, with `each` rows and per-step errors → verify: `journeys.test.mjs` passes unchanged against the moved runner, and a new test stops a chain at a failing step with its error
- [ ] 2 `data/workflows.json` schema and lint: inputs map onto real step inputs, `from` points backward to real outputs, at most 8 visible inputs → verify: `tools/trust/workflows.test.mjs` fails on each broken fixture
- [ ] 3 Port the 8 journeys into workflows (vfr-preflight → preflight-check, drone-mapping-day → mapping-flight, the rest by slug); add `/journeys/<slug>/` → `/workflows/<slug>/` redirects → verify: `routes.test.mjs` and `redirects.json` check; the build runs every workflow's prefill through the core
- [ ] 4 Workflow page: form, step list with answer sentences and tool links, assumptions, live recompute, permalink → verify: a browser test edits one input on `vfr-cross-country` and checks every later step's sentence changes, and the permalink restores it
- [ ] 5 Combined visuals of design D4 → verify: `visual-purpose.test.mjs` extended to workflows (each draws from its prefill)
- [ ] 6 Exports: nav log CSV and print view; mission KML; coordinate table copy → verify: export tests byte-compare the prefill's files
- [ ] 7 MCP: `workflow.<slug>` in search, describe, and run → verify: golden surface updated; `mcp/server.test.mjs` runs each workflow and matches the web chain byte for byte
- [ ] 8 Build the new-tool workflows (vfr-cross-country, mapping-flight, can-i-fly-now) after `add-flight-and-drone-planning-tools` lands → verify: tasks 4 to 7 for each
- [ ] 9 Performance: the whole mapping-flight chain within 300 ms on the reference profile → verify: `bench:browser` case
