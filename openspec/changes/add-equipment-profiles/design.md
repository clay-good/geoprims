## Context

`apps/web/src/lib/aircraft.mjs` already keeps, per tool, the inputs that belong to the airplane rather than the flight, stores them under one local-storage key, and moves them as `geoprims-aircraft` JSON files. It is tested and private. This change widens the same pattern to the other equipment practitioners own, and makes the field mapping data so a new kind needs no code.

## Goals / Non-Goals

**Goals:**
- Enter equipment once; use it on every tool and workflow that takes it.
- Keep the privacy and determinism rules exactly as they are.

**Non-Goals:**
- Any server-side storage. Any equipment database maintained by geoprims.

## Decisions

### EP1. Kinds and field maps live in `data/profiles.json`
Each kind lists, per tool id, the input fields a profile keeps:

| Kind | Tools (fields kept) |
|---|---|
| `aircraft` | the five tools and fields in `aircraft.mjs` today, unchanged |
| `drone` | `drone.sensors.vlos` (characteristic dimension, airframe type), `drone.ops.wind-limit` (wind rating, max airspeed), `drone.power.endurance`, `drone.power.rth-budget`, `drone.mission.sorties` (battery energy or usable flight time, hover and cruise power or calibrated FM·η, cruise and transit speed), `drone.ops.kinetic-energy` (mass), `drone.ops.speed-check` (max groundspeed) |
| `camera` | every `drone.photogrammetry.*` tool's sensor width and height, image width and height, focal length, minimum interval, and `drone.sensors.thermal-footprint` (IFOV or pixel pitch) |
| `lidar` | `drone.sensors.lidar-plan` (pulse rate, field of view, returns), `drone.sensors.dataset-size` (point record size) |
| `gnss-receiver` | `survey.gnss.rtk-budget` (a + b ppm horizontal and vertical), `survey.gnss.antenna-height` (antenna radius, mark-to-ARP offset or ANTINFO antenna id) |
| `radio` | `drone.links.link-budget` (transmit power, antenna gains, cable loss, receiver sensitivity, frequency) |

The build checks that every listed field exists on the tool and is not marked `x-private`. A field kept by a profile is never a job value (no location, date, wind, or load).

- *Alternative: a profile holds free-form values by quantity (one "focal length" shared by every tool).* Rejected. Tools name the same quantity differently, and a profile that is "the tool's own JSON" is readable and importable without a mapping layer. Tool-field maps are what works today.

### EP1a. Profiles live on the tools, not a settings page (decided 2026-10-19)
`simplify-launch-chrome` removed `/settings/`, and the owner chose to keep it removed: each tool's panel lists, saves, loads, exports, imports, and deletes its kind's profiles, and the browser's own "clear site data" erases them. A field saved for one tool fills the same field on the kind's other tools (the tool's own value first), so a camera saved on the GSD tool fills the trigger tool; `geoprims_run` applies the same rule.

### EP2. One file format, one storage key
Format `geoprims-profile`, version 1: `{format, version, kind, name, tools: {<toolId>: {<field>: value}}}`. The aircraft format (`geoprims-aircraft` v1) imports as `kind: "aircraft"`; export writes the new format. Storage is one key, `gp-profiles`, holding every profile by kind and name; the old `gp-aircraft` key migrates on first read and is then removed. Limits: 1 MB per file, 60-character names, 50 profiles per kind.

### EP3. Filling a tool: profile values are inputs, nothing more
Loading a profile sets the tool's input fields. The result is computed from those inputs like any others, so permalinks, reports, "copy as agent call", and MCP parity are unchanged. The page shows a chip "From profile: Mavic 3E" beside the answer until an equipment field is edited, then "Edited from Mavic 3E". The profile name is display state only: it never enters the fragment, the report payload, or any request. The chip is for the reader.

### EP4. Workflows declare profile-fillable inputs
A workflow input may carry `"profile": {"kind": "camera", "field": "focal_length"}`. The page then offers one "Use my <kind>" control per kind the workflow names, listing saved profiles of that kind. Filling writes the profile's values into the workflow inputs (visible or advanced), and the assumptions list reads "Camera: from profile Zenmuse P1" while the values stand. The chain runner is untouched: it receives inputs as before.

### EP5. MCP: inline, stateless
`geoprims_run` gains an optional `profile` argument whose value is a profile object in the file format. For a tool, the server takes `profile.tools[id]` and fills any input the caller did not pass. For `workflow.<slug>`, the server fills workflow inputs declared for that kind. The server validates the object against the format, ignores tools and fields the kind does not keep, and reports which inputs the profile filled in `meta.profileFilled` (field names only). It reads no files and remembers nothing between calls.

- *Alternative: `--profiles=<dir>` on the server.* Rejected. It adds filesystem reads and per-machine state to a server whose whole value is being stateless and reproducible from the call alone.

### EP6. Shipped examples
The site ships the example equipment the tools' worked examples already use (the 1-inch 20 MP camera, the small multirotor, the published RTK receiver spec) as read-only example profiles, labeled "Example" and never counted as the reader's. They give the manager something to show before the reader saves anything.

## Risks / Trade-offs

- **[A stale profile silently feeds a plan]** → The chip names the profile on every result; a profile shows its saved date; the assumptions list on workflows names it.
- **[Profile fields drift from manifests]** → The build gate checks every field in `data/profiles.json` against the catalog.
- **[Private windows and full storage]** → As today: the profile lasts for the page, and the panel says so.

## Migration Plan

On first load, `gp-aircraft` entries become `gp-profiles` entries of kind `aircraft`; old `.aircraft.json` files import indefinitely. No tool, vector, or result changes.

## Open Questions

None.
