# Performance

Every budget in every geoprims spec is measured on one profile, so a number in
one document means the same thing as the same number in another. The profile
lives in [`data/reference-profile.json`](../data/reference-profile.json), which
is what the gates read; this page explains it and records its history.

**Current profile: 1.0.0**, in force since 2026-09-20.

## The profile

| Aspect | Profile 1.0.0 |
|---|---|
| CPU | Chromium on the CI runner at 4× slowdown, a proxy for a mid-tier 2024 Android phone, plus a quarterly manual check on a real mid-tier Android device and a 3-year-old iPhone |
| Network, cold load | 9 Mbps down, 1.5 Mbps up, 150 ms round trip, no cache |
| Warm load | Service worker installed, same network |
| Engines | Chromium, WebKit, and Firefox, current stable; Node.js active LTS for the MCP server and the core |
| Measurement | Median of three runs per route, from Playwright traces and web-vitals, in CI only |

## Viewports

Mobile and layout gates use these, and no others:

| Viewport | Size | Why |
|---|---|---|
| Smallest phone | 320 × 720 | Nothing may overflow horizontally |
| Typical phone | 390 × 844 | The answer must be visible without scrolling |
| Phone landscape | 568 × 320 | The shortest viewport a tool page meets |
| Tablet portrait | 768 × 1024 | |
| Tablet landscape | 1180 × 820 | The electronic flight bag case |
| Desktop | 1440 × 900 | |

Text zoom is checked at 200% at 375 px wide.

## Budgets

| Budget | Hard | Target |
|---|---|---|
| Cold Wasm module instantiation | 150 ms | |
| Largest contentful paint | 2.0 s | 1.5 s |
| Interactive | 2.5 s | |
| Interaction to next paint | 200 ms | 100 ms |
| Cumulative layout shift | 0.1 | 0.05 |
| Shell JavaScript | 90 KB | |
| Self-hosted fonts | 80 KB | |
| Offline precache | 12 MB compressed | |

The pre-rendered answer is the largest contentful paint element, or appears
before it. WebAssembly compiles in the background after first paint, and every
calculation runs in a worker.

## What is measured today

The Playwright suite checks cancellation and cross-host result equality in CI.
The page budgets now have a gate: `apps/web/test/browser/perf.test.mjs` loads
50 sampled routes (the home page, the tool index, every domain, a few groups,
and tools spread across the catalog) three times each, cold, under profile
1.0.0: 4× CPU slowdown, 9 Mbps down, 1.5 Mbps up, 150 ms round trip, no cache,
at the typical-phone viewport. The median of each is held to the hard budgets.
LCP and layout shift come from the browser's performance entries, interactive
is the moment every island has hydrated, INP is the longest event after typing
into a tool's first field (or pressing the mode toggle on other pages), and
shell JavaScript is every script the page loads, gzip-compressed.

The last local run (2026-09-21, on a developer Mac, served over plain HTTP on
localhost, so without TLS) passed every route. Worst medians:

| Budget | Worst of 50 routes | Hard budget |
|---|---|---|
| Largest contentful paint | 564 ms | 2.0 s |
| Interactive | 939 ms | 2.5 s |
| Interaction to next paint | 72 ms | 200 ms |
| Cumulative layout shift | 0.030 | 0.1 |
| JavaScript (tool page, gzip) | 63.8 KB | 90 KB |

Map frames: `apps/web/test/browser/frames.test.mjs` pans a 100,000-vertex
track and a 100,000-vertex polygon at 4× CPU. Locally the p95 frame is
11.1 ms on the flat map and 15.5 ms on the globe, within one 60 Hz frame
(16.7 ms); a path that zig-zags every few pixels along its whole length still
takes about 36 ms, since every one of its pixels has to be drawn.

These are local numbers. The profile names the CI runner, and CI is not running
yet, so no release baseline exists; the same gate runs there once it is.
Chromium only: WebKit and Firefox page metrics are still to come.

The Chromium calculator benchmark applies the profile's 4× CPU setting and
reports each tool's p50 and p95. The following checks run without a browser:

| Check | Where | Today |
|---|---|---|
| Wasm module size, Brotli | `tools/wasm/build.mjs` | Each module against its own budget |
| Offline precache | `apps/web/scripts/pwa.mjs` | Compressed release size against the profile's budget |
| Self-hosted fonts | `apps/web/test/theme.test.mjs` | 52 KB, against the profile's budget |
| Search ranking latency | `tools/search/accuracy.test.mjs` | p95 1.6 ms at 1,000 entries |
| Agent round trip | `tools/mcp/eval.test.mjs` | 406 tokens per task |

The browser suite separately checks that a long H3 calculation stops within
100 ms of an edit and that all 2,705 live golden vectors serialize identically
in Chromium, Firefox, WebKit, and Node. These are functional checks, not
performance measurements against profile 1.0.0.

The Node benchmark is available with `npm run bench:node -- --output /tmp/geoprims-node-bench.json`.
It warms every tool on its primary example, then reports p50 and p95 over 1,000
invocations. Pass a prior report with `--baseline <path>` to show the p95 change
and fail when it rises by more than 20%. Reports from different profile versions
or hosts cannot be compared; new tools have no previous value. There is no published Node release baseline yet;
local results vary with the machine and do not establish the browser budgets.

The browser counterpart is `npm run bench:browser --prefix apps/web -- --output /tmp/geoprims-chromium.json`.
It runs the same Wasm modules on a blank Chromium page with the profile's 4×
CPU setting, 50 warm-up calls, and 1,000 timed calls per tool. The timer is
inside the shared loader around the synchronous Wasm ABI call; it excludes
worker messages, async scheduling, and asset downloads. CDP does not throttle
a dedicated worker. The report also records cold module initialization
after download. CI uploads the table and JSON; a previous-release Chromium
baseline and per-tool budget declarations are still needed for a release gate.
Cold module instantiation already has a 150 ms gate in the Chromium benchmark.
Local 4× runs show some simple calls with p50 near 0.2 ms and p95 above 3 ms;
the ordered timings include irregular 3 ms stalls in about 9% of repeated
identical calls. The 2 ms closed-form limit cannot be claimed as passing from
these local runs. A reference runner or device check must resolve that before
turning on the per-tool gate.

## Changing the profile

A profile change is a version bump, not an edit. Raise `version` in
`data/reference-profile.json`, add a row below, and publish a comparison run on
both the old and the new profile so the baselines can be reset knowingly.

| Version | Date | Change |
|---|---|---|
| 1.0.0 | 2026-09-20 | First profile, from `contracts/reference-profiles` |
