## Why

Most of what a practitioner types into geoprims belongs to their equipment, not to today's job. A drone's wind rating, battery energy, and characteristic dimension; a camera's sensor size, pixels, focal length, and minimum interval; a lidar sensor's pulse rate and field of view; a GNSS receiver's RTK specification and antenna offsets. Today only aircraft data can be saved (`aviation/fuel-and-loading`, "Aircraft profiles saved locally"), and only on five aviation tools. A drone operator planning a mapping flight re-enters the same camera on four tools and again on the workflow page. Every re-entry is a chance to type 8.8 mm as 24 mm.

Equipment profiles extend the aircraft pattern that already works: named sets of equipment values, saved in the browser, exported as a readable JSON file, never uploaded. A profile fills every tool and workflow that takes its values, in one tap.

Depends on: `add-aviation-suite` (aircraft profiles), `add-drone-suite`, `add-practitioner-essentials` (sensors, GNSS), `add-job-workflows`, `add-local-mcp-server`.

## What Changes

- **Profile kinds as data.** `data/profiles.json` names each kind (aircraft, drone, camera, lidar sensor, GNSS receiver, radio link) and, per kind, the tool fields a profile keeps. The existing aircraft mapping moves into this file unchanged.
- **A profile panel on every tool that takes profile values:** save this tool's equipment inputs under a name, load a profile, export, import, delete. There is no separate manager page: `simplify-launch-chrome` removed `/settings/`, and the panels cover the job. Existing `.aircraft.json` files keep importing.
- **Workflows take profiles.** A workflow declares which of its inputs come from which kind, so "Plan a mapping flight" offers "Use my drone" and "Use my camera", and the assumptions list names the profile.
- **Agents pass profiles inline.** `geoprims_run` and `workflow.<slug>` accept a `profile` object in the same JSON as the exported file; its values fill any input not given explicitly. The server reads no files and keeps no state.
- **Privacy unchanged.** Profile names and files never enter permalinks, reports, or requests. Permalinks carry the values a profile filled, so a link still reproduces the calculation on another device.

## Capabilities

### New Capabilities

- `web/equipment-profiles`: Profile kinds, the per-tool panel, storage, import and export, and how profiles fill tools.

### Modified Capabilities

- `web/workflows`: Workflow inputs can be filled from a profile of a declared kind (added requirement).
- `agent/mcp-server`: `geoprims_run` and workflow runs accept an inline profile (added requirement).

## Non-goals

- Cloud sync or accounts. A profile moves between devices as a file.
- A vendor equipment database. The site ships a few labeled example profiles (the same examples the tools already use); users enter their own equipment.
- Profiles for places (home fields, sites) or jobs. Those are permalinks.

## Impact

- `apps/web/src/lib/aircraft.mjs` generalizes to `profiles.mjs`; `AircraftPanel.svelte` becomes the profile panel. The `gp-aircraft` storage key migrates on first load.
- `data/workflows.json` inputs gain an optional `profile` field.
- The MCP golden surface changes for the `profile` argument of `geoprims_run`.
- No core change, no new tool ids, no result changes.
