## Why

People don't come to geoprims for one formula. They come to get a job done: plan tomorrow's cross-country, fly a mapping job, or turn a coordinate from a client into the form their software wants. Today the site answers one step at a time. The eight journeys at `/journeys/<slug>/` show how the steps chain, but they are fixed worked examples. To use your own numbers, you click through five tool pages and carry answers between them by hand. That is where time goes and where mistakes creep in, like a true course entered as magnetic or a height in feet read as meters.

A workflow page does the carrying. You enter only what the job needs. Every step runs live in the same core, and one picture shows the whole plan.

Depends on: `plan-launch-and-value-proof` (L3 journeys), `build-web-experience` (`web/app-shell` tool chaining, `web/map-canvas`), `add-local-mcp-server`, and `align-visuals-with-jobs`. The new flight and drone tools come from `add-flight-and-drone-planning-tools`.

## What Changes

- **Workflow pages** at `/workflows/<slug>/`. Each is one form with the few inputs the job needs, prefilled with a worked example. Below it is a step list: each step shows its answer sentence and links to its full tool page, prefilled. One combined visual shows the plan, and there is one export. Everything recomputes as you type.
- **Launch set: eleven workflows.** Below are eight jobs someone has this week. The three other journeys (IFR approach brief, boundary retracement, index to area) move over as they are.

| Workflow | For | You enter | You get |
|---|---|---|---|
| Plan a VFR cross-country | Pilots | Waypoints, cruise altitude, TAS, winds aloft, fuel burn, departure time | Nav log per leg (TC, WCA, TH, MH, GS, ETE, fuel), fuel with reserve, top of climb and descent, sunset and night, route on the map |
| Preflight check | Pilots | METAR, field elevation, runway, aircraft weights | Pressure and density altitude, runway wind, best runway, weight and balance, sunset |
| Plan a mapping flight | Drone operators | Area on the map, camera, target GSD, overlap, battery, pilot position | Height (with the Part 107 check), grid, photo count, dataset size, sorties and battery swaps, sun window, VLOS check, KML |
| Plan an inspection | Drone operators | Structure location and height, standoff, camera | Orbit or facade path, GSD on the structure, flight time, gimbal angles, KML |
| Can I fly now? | Drone operators | Location, time, wind (METAR or typed), drone limits, nearest structure | Wind at flight height against the drone's limit, civil twilight, Part 107 ceiling, groundspeed check |
| Convert a coordinate | Everyone | Any coordinate, pasted | Every format at once (DD, DMS, DDM, UTM, MGRS, USNG, state plane, H3, S2, geohash, Plus Code), each with a copy button, plus a pin on the map |
| GNSS point to state plane | Surveyors | Lat, lon, ellipsoid height, epoch | NAD83(2011), state plane coordinates, orthometric height, combined factor, ground distance |
| Lidar bid | Operators and surveyors | Area, sensor, accuracy class | Height and speed, strips, flight time and batteries, data size, checkpoint count |

- **Journeys become workflows.** Each of the eight journey slugs redirects (301) to its workflow. Its worked example becomes the workflow's prefill, and the build still runs every chain through the core.
- **Agents get the same workflows.** `geoprims_search` finds them, `geoprims_describe` explains them, and `geoprims_run` runs `workflow.<slug>` with the same inputs, returning every step's result. The web and MCP use one shared chain runner.
- **No new math.** A workflow only connects tools. Every number comes from a core tool, and a step that fails stops the chain with that step's own error.

## Non-goals

- Live data: METARs, winds aloft, NOTAMs, TFRs, airspace, and terrain are not fetched. The reader pastes or types them, as the tools already expect.
- Saving plans to an account, or syncing them. The permalink holds the inputs.
- Approving a flight. Workflows help plan and check. The legal notices of each tool carry through to the workflow.
- Custom workflows built by readers (a later change could allow one from a permalink).

## Impact

A new route family (`contracts/routes-and-urls`), a new data file `data/workflows.json` that replaces `data/journeys.json`, and a chain runner moved from `apps/web/src/lib/journeys.mjs` to `packages/runtime`. The MCP golden surface gains the workflow ids. Journey pages remain reachable through their redirects.
