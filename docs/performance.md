# Performance

Every budget in every geoprims spec is measured on one profile, so a number in
one document means the same thing as the same number in another. The profile
lives in [`data/reference-profile.json`](../data/reference-profile.json), which
is what the gates read; this page explains it and records its history.

**Current profile: 1.1.0**, in force since 2026-09-23.

## The profile

| Aspect | Profile 1.1.0 |
|---|---|
| CPU | Chromium on the CI runner, throttled to Lighthouse BenchmarkIndex 800 (the top of Lighthouse's mid-tier mobile range), a proxy for a mid-tier 2024 Android phone, plus a quarterly manual check on a real mid-tier Android device and a 3-year-old iPhone |
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
| Closed-form tool, p95 | 2 ms | |
| Iterative tool, p95 | 10 ms | |
| Data-proportional tool, p95 | Per tool in `data/performance-budgets.json` | |
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
1.1.0: CPU throttled to BenchmarkIndex 800, 9 Mbps down, 1.5 Mbps up, 150 ms round trip, no cache,
at the typical-phone viewport. The median of each is held to the hard budgets.
LCP and layout shift come from the browser's performance entries, interactive
is the moment every island has hydrated, INP is the longest event after typing
into a tool's first field (or pressing the mode toggle on other pages), and
shell JavaScript is every script the page loads, gzip-compressed.

The last local run (2026-10-12, on a developer Mac, served over plain HTTP on
localhost, so without TLS) passed every route. Worst medians:

| Budget | Worst of 50 routes | Hard budget |
|---|---|---|
| Largest contentful paint | 584 ms | 2.0 s |
| Interactive | 1,060 ms | 2.5 s |
| Interaction to next paint | 72 ms | 200 ms |
| Cumulative layout shift | 0.000 | 0.1 |
| JavaScript (tool page, gzip) | 87.9 KB | 90 KB |

Export serializers load only when a reader requests a download. A geographic
page paints its complete Canvas2D map first, then loads the optional luma.gl
WebGPU or WebGL2 compositor after the first map interaction or after the
3-second startup window. Before that split, sampled map pages loaded about
229 KB and failed the shell budget; the same pages now load 79.2 KB initially.

Map frames: `apps/web/test/browser/frames.test.mjs` pans a 100,000-vertex
track and a 100,000-vertex polygon at 4× CPU. Locally the p95 frame is
11.1 ms on the flat map and 15.5 ms on the globe, within one 60 Hz frame
(16.7 ms); a path that zig-zags every few pixels along its whole length still
takes about 36 ms, since every one of its pixels has to be drawn.

These are local page-load numbers. Chromium only: WebKit and Firefox page
metrics are still to come.

The Chromium calculator benchmark calibrates the CPU slowdown to the profile's
BenchmarkIndex 800 target and
reports each tool's p50 and p95. The following checks run without a browser:

| Check | Where | Today |
|---|---|---|
| Wasm module size, Brotli | `tools/wasm/build.mjs` | Each module against its own budget |
| Offline precache | `apps/web/scripts/pwa.mjs` | Compressed release size against the profile's budget |
| Self-hosted fonts | `apps/web/test/theme.test.mjs` | 52 KB, against the profile's budget |
| Search ranking latency | `tools/search/accuracy.test.mjs` | p95 1.6 ms at 1,000 entries |
| Agent round trip | `tools/mcp/eval.test.mjs` | 406 tokens per task |

The browser suite separately checks that a long H3 calculation stops within
100 ms of an edit and that all live golden vectors serialize identically
in Chromium, Firefox, WebKit, and Node. These are functional checks, not
performance measurements against profile 1.0.0.

The Node benchmark is available with `npm run bench:node -- --output /tmp/geoprims-node-bench.json`.
It warms every tool on its primary example, then reports p50 and p95 over 1,000
invocations. Pass a prior report with `--baseline <path>` to show the p95 change
and fail when it rises by more than 20%. Reports from different profile versions
or hosts cannot be compared; new tools have no previous value. There is no published Node release baseline yet;
local results vary with the machine and do not establish the browser budgets.

The browser counterpart is `npm run bench:browser --prefix apps/web -- --baseline ../../data/performance-baselines/chromium-1.1.0.json --output /tmp/geoprims-chromium.json`.
It runs the same Wasm modules on a blank Chromium page with CPU slowdown
calibrated to the profile target, 50 warm-up calls, and 1,000 timed calls per
tool. The timer is
inside the shared loader around the synchronous Wasm ABI call; it excludes
worker messages, async scheduling, and asset downloads. CDP does not throttle
a dedicated worker. The report also records cold module initialization
after download. It then runs every workflow's whole chain through the
shared chain runner, 5 warm-up and 50 timed runs each, and fails when the
mapping-flight chain's p95 is over 300 ms (add-job-workflows); on 2026-10-07
it was 65 ms for all nine steps. `--workflows-only` skips the per-tool pass.
CI compares every tool with the checked-in artifact from run
38482406532 at commit `90d3d55e`. That run measured all 390 tools and then
failed the very comparison this baseline replaces, which is why the table and
JSON are uploaded on every run, including a failing one. A p95
increase over 20% and 0.5 ms fails, so timer granularity cannot turn a 0.1 ms
call into a false regression. Both are measured after taking off the median
rise across the whole run, never less than zero, when at least 20 tools are
compared. Runners differ twofold in speed, and the calibrated throttle is not
linear: run 38490441839, a README-only commit on a runner throttled 5.96×
against the baseline's 3.24×, read every one of the 390 tools 0.1 to 0.7 ms
later (median 0.4 ms), and `indexing.s2.cell-info` crossed the 0.5 ms floor by
a rounding error. A rise every tool shares is the host; one tool cannot move
the median, so a tool that slows down still stands out from it, and the failing
line now names it. A slowdown of the whole core would hide in the median, which
is what the absolute budgets below are for. The same run enforces `data/performance-budgets.json`:
2 ms by default for closed-form tools, 10 ms for iterative tools, and an
explicit ceiling and scaling input for every list or data-proportional tool.
It also holds cold module instantiation to 150 ms. The 2026-10-13 baseline has
390 tools; all pass their execution budgets.

Refresh the baseline when a tool legitimately starts doing more work, not to
quiet a regression whose cause is unknown. The 2026-10-10 baseline was replaced
because four tools had grown past it for reasons their changelog entries state:
`time.sun.position` computes a day-night boundary on every call since the solar
terminator landed, `indexing.h3.polygon-to-cells` gained fill modes, and
`drone.mission.survey-grid` and `time.sun.aviation-nights` followed their own
feature work. Take the JSON from the run's `chromium-benchmark` artifact, add
`sourceCommit` and `sourceRun`, and check it in: it has to come from a CI run,
because a laptop reads these times differently even throttled to the same
target.

## Changing the profile

A profile change is a version bump, not an edit. Raise `version` in
`data/reference-profile.json`, add a row below, and publish a comparison run on
both the old and the new profile so the baselines can be reset knowingly.

| Version | Date | Change |
|---|---|---|
| 1.0.0 | 2026-09-20 | First profile, from `contracts/reference-profiles` |
| 1.1.0 | 2026-09-23 | CPU throttle calibrated to the host's Lighthouse BenchmarkIndex (target 800) instead of a fixed 4× |

## Profile 1.1.0 (2026-09-23): the CPU throttle is calibrated to the host

Profile 1.0.0 throttled every host by a fixed 4×. The first CI runs after the
repository went public showed what that hid: the development Mac scores a
Lighthouse BenchmarkIndex of about 4,150, so 4× emulated a phone of about
1,010 (Lighthouse's high-end mobile range), while the slower CI runner at 4×
emulated a much slower phone. The same page passed its budgets on one machine
and missed them by about 20% on the other. Lighthouse's own throttling guide
says the multiplier should follow the host's BenchmarkIndex.

Every timing gate now measures the host's BenchmarkIndex once per run (its
`computeBenchmarkIndex`, median of three, unthrottled) and throttles by
host ÷ 800, held to 1–10, so every machine emulates the same device. On the
development Mac that is 5.2×, stricter than before. The rate and the host's
index are printed with each result (`apps/web/scripts/cpu.mjs`).

