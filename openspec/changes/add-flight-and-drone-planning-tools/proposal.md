## Why

The catalog computes every piece of a flight plan but not the plan itself. A pilot can solve one wind triangle, but there is no nav log that does every leg at once with the fuel. A drone operator can get a grid and a flight time, but nothing says how many batteries the job takes, where to swap them, or whether the far corner is out of sight. These are the steps the `add-job-workflows` workflows need, and the ones people now do on paper or in a spreadsheet.

Depends on: `add-aviation-suite`, `add-drone-suite`, `add-navigation-and-geometry`. Feeds: `add-job-workflows`.

## What Changes

Seven tools. Each composes existing core functions: no formula is written twice. Each has a visual that serves its job (`align-visuals-with-jobs`).

| Tool | Question it answers | Visual |
|---|---|---|
| `aviation.flight-plan.nav-log` | For each leg: what heading do I fly, how fast over the ground, how long, and how much fuel? | Route on the map, each leg labeled with magnetic heading, groundspeed, and time |
| `aviation.performance.climb-plan` | How long, how far, and how much fuel to reach cruise, and where is top of climb? | Side profile from the runway to top of climb |
| `aviation.performance.etp-pnr` | Where on this leg is it as quick to go on as to turn back, and until when can I still turn back? | A line from departure to destination with the ETP and PNR marked |
| `drone.mission.sorties` | How many batteries does this mission take, and where does each flight end? | Map: the path colored by sortie, with home and the swap points |
| `drone.ops.wind-limit` | Is the wind at my flying height within what my drone can handle? | Bar of wind at height against the drone's limit, gust marked |
| `drone.ops.vlos-check` | Can I see the drone from where I stand, over the whole mission? | Map: the pilot, a ring at visual range, and the waypoints beyond it marked |
| `drone.photogrammetry.gcp-plan` | How many ground control points and checkpoints, and where? | Map: the area with suggested GCP and checkpoint spots |

## Non-goals

- Weight-and-balance or takeoff performance from a type's own charts. `aviation.loading.table-interpolate` already takes the reader's POH table.
- Airspace, obstacle, terrain, or NOTAM checks: no operational data.
- Terrain-following sorties (deferred with the terrain-following export, per `docs/research/07-practitioner-gap-analysis.md` section C).
- Recommending a GCP layout as sufficient for a given accuracy. The count follows ASPRS Edition 2. The layout is a stated rule of thumb, labeled as one.

## Impact

Seven new tools, all starting as experimental and promoted with the promotion recipe. The sources ledger gains the ETP and PNR source and the wind-profile source (design D3, D5). `data/hubs.json` adds a "Plan a flight" task and a "Plan a drone mission" task.
