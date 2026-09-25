## Context

Journeys already chain tools through `{"from": [step, "output"]}` wiring and run at build time (`apps/web/src/lib/journeys.mjs`). "Send to" (`lib/chain.mjs`) carries one value to one tool. Workflows keep the wiring and run it live, with the reader's inputs, on one page.

## Decisions

### D1. A workflow is data, not code
`data/workflows.json` holds, for each workflow: its slug, title, audience, the job in one sentence, its own inputs (each mapped onto one or more step inputs), its steps with `from` wiring (optionally `each` to run a step once per row of an earlier list), its combined visual, and its export.
- *Alternative: a Svelte page per workflow.* Rejected. Eight hand-built pages drift apart, and the MCP server could not run them.
- *Alternative: let the reader chain any tools.* Deferred. It is powerful but not dead simple, and the fixed workflows cover the common jobs.

### D2. One chain runner, shared
The runner moves to `packages/runtime/src/chain.mjs` and takes an `invoke(id, input)` function. The site build, the workflow page (in the compute worker), and the MCP server all call it, so all three give byte-identical step results.
- *Alternative: the web and MCP each keep their own runner.* Rejected. It breaks "never implement math twice" in spirit, because the wiring decides which number feeds which tool.

### D3. Inputs the job needs, and no more
A workflow shows at most eight inputs before "More options". Every other step input is fixed by the workflow (with a stated default) or carried from an earlier step. Each step's full tool page stays one click away, prefilled, for the reader who wants every option.

### D4. One combined visual per workflow
Each workflow names one visual that answers its main question:

| Workflow | Visual |
|---|---|
| VFR cross-country | Route on the map: legs labeled with MH, GS, and ETE, top-of-climb and top-of-descent marks |
| Preflight check | Runway wind diagram, with density altitude beside it |
| Mapping flight | Map: area, grid colored by sortie, photo points, home, battery swap points, VLOS ring |
| Inspection | Map: orbit or facade path around the structure, plus a side view of the standoff |
| Can I fly now? | A checklist of go and caution items; no map unless a location is entered |
| Convert a coordinate | Map pin, with the UTM zone and MGRS square outlined |
| GNSS to state plane | Height stack diagram (ellipsoid, geoid, orthometric) |
| Lidar bid | Map: area and strips |
| IFR approach brief, boundary retracement, index to area | The visual of the journey's key step (hold entry, deed plot, H3 cells) |

A step's own visual stays on its tool page. The workflow draws only the combined one (`web/map-canvas`, "Every visual serves the job").

### D5. A failed step
The chain stops at the failed step. That step shows the tool's own error and field, and later steps show "Waiting for step N". Nothing is guessed.

### D6. MCP surface
Workflows are catalog entries of kind `workflow` with id `workflow.<slug>`. They are searchable and describable, and `geoprims_run` runs them. The result lists each step (tool id, input, result) plus the workflow's summary. The default meta-tool surface (`agent/mcp-server`) is unchanged: no new top-level tools.

## Workflow inventory

| Workflow | Steps (tools) | New tools it needs |
|---|---|---|
| vfr-cross-country | route.legs, wind.aloft-interpolate, flight-plan.nav-log, performance.climb-plan, loading.fuel-plan, performance.top-of-descent, sun.events | nav-log, climb-plan |
| preflight-check | weather.metar-decode, altimetry.pressure-altitude, altimetry.density-altitude, wind.best-runway, wind.runway-components, loading.weight-balance, sun.events | none |
| mapping-flight | altitude-for-gsd, ops.part107-altitude, photogrammetry.trigger, motion-blur, mission.survey-grid, sensors.dataset-size, mission.sorties, ops.vlos-check, sun.mapping-window, mission.export | sorties, vlos-check |
| inspection | mission.orbit or mission.facade, photogrammetry.gsd, power.endurance, mission.export | none |
| can-i-fly-now | weather.metar-decode, ops.wind-limit, sun.events, ops.part107-altitude, ops.speed-check | wind-limit |
| convert-coordinate | parse.coordinates, parse.format (DMS, DDM), utm.forward, grid-ref.mgrs-forward, grid-ref.usng-forward, spcs.zone-lookup, spcs.spcs83-forward, convert.cross-index | none |
| gnss-to-state-plane | datum.nad83, spcs.zone-lookup, spcs.spcs83-forward, height.convert, reduction.combined-factor | none |
| lidar-bid | area.polygon, sensors.lidar-plan, mission.survey-grid, power.endurance, sensors.dataset-size, photogrammetry.asprs-accuracy | none |
| ifr-approach-brief | The journey's steps, unchanged | none |
| boundary-retracement | The journey's steps, unchanged | none |
| index-to-area | The journey's steps, unchanged | none |

Journey redirects: vfr-preflight → preflight-check, drone-mapping-day → mapping-flight, and the other six to the workflow of the same slug.

Endpoints: 11 workflow pages, 8 journey redirects, 11 MCP workflow ids.

## Risks

- **A long chain is slow on a phone.** The mapping flight runs 10 tools, with the survey grid the heaviest. Steps run in the compute worker, each step only when its inputs change, and the newest input wins (`invokeLatest`). Budget: 300 ms for the whole chain on the reference profile.
- **A workflow that hides options gives a wrong default.** Every fixed input is listed under "Assumptions" on the page, with a link to change it on the tool page.
