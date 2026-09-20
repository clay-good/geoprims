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

The Playwright suite now checks cancellation and cross-host result equality in
CI. The browser performance budgets have no gate yet: the suite does not apply
the profile's 4× CPU slowdown or network settings, or measure paint, interaction,
and layout shift. The following checks run without a browser:

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

## Changing the profile

A profile change is a version bump, not an edit. Raise `version` in
`data/reference-profile.json`, add a row below, and publish a comparison run on
both the old and the new profile so the baselines can be reset knowingly.

| Version | Date | Change |
|---|---|---|
| 1.0.0 | 2026-09-20 | First profile, from `contracts/reference-profiles` |
