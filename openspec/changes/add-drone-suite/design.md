## Context

Motivation is in `proposal.md`. Research: `docs/research/03-aviation-drone-survey-formulas.md` §6 (GSD, overlap, blur, ASPRS Edition 2, hover power, Part 107, Remote ID, Part 108 status, EASA classes). Key facts as of September 18, 2026:

- **FAA Part 108 (BVLOS) is not final.** The NPRM was published August 7, 2025, and the comment period reopened in early 2026. A report says the rule is at OIRA.
- **EASA.** National standard scenarios expired on December 31, 2025; STS operations now need C5/C6 drones.
- **ASPRS Edition 2 v2 (2024)** uses RMSE only, requires at least 30 checkpoints, and requires checkpoints at least 2× better than the product.

## Goals / Non-Goals

**Goals:**
- Vendor-neutral mission math with honest endurance, meaning losses always applied.
- Regulatory values that cannot silently go stale: dated, reviewed, and labeled by status.

**Non-Goals:**
- A camera database maintained by us. Users enter sensor specs, and a small set of labeled examples is provided. Upkeep of vendor specs is out of scope.
- Flight-controller integrations.

## Decisions

### DR1. Physics first, heuristics labeled
- **Hover power:** momentum theory × figure of merit × efficiency.
- **Heuristics:** cruise-power multiplier, temperature derating, and Peukert are user-adjustable, labeled `HEURISTIC_*`, and off or neutral by default where evidence is weak.

Alternative: vendor-published endurance figures. Rejected because they are measured under ideal conditions and not transferable.

### DR2. Pattern generation in planar local coordinates, verified geodesically
Grids are generated in a local azimuthal-equidistant or UTM plane chosen for the polygon, then converted to geographic coordinates. Line spacing is verified by sampling geodesic distances. At survey scales (< 50 km) the planar error is far below GPS accuracy, and the verification step proves it per mission.

### DR3. Heights are typed
Every waypoint height carries its reference (AGL, takeoff-relative, MSL, HAE), and conversions reuse geodesy. This removes the most common drone-mapping altitude error (height above takeoff vs above terrain).

### DR4. Regulatory reference data file
Regulatory values live in `reference/regulatory.json`, with jurisdiction, citation, value, unit, effective date, review date, status, and link. A CI check warns on entries older than 12 months. Tools read values from this file only.

## Tool inventory (targets)

| Group | Operations | Examples |
|---|---|---|
| `photogrammetry` | 14 | gsd, altitude-for-gsd, footprint, trigger-distance, trigger-interval, line-spacing, motion-blur, max-shutter, oblique-gsd, terrain-overlap, image-count, asprs-accuracy, checkpoint-requirements, crop-factor |
| `mission` | 10 | survey-grid, crosshatch, corridor, orbit, facade-scan, geofence, geofence-check, terrain-following, pattern-stats, pattern-export |
| `power` | 11 | battery-energy, usable-energy, c-rate, disk-area, hover-power, endurance, range, payload-impact, max-payload, rth-budget, temperature-derating |
| `ops` | 7 | part107-altitude, structure-altitude, speed-check, kinetic-energy, easa-subcategory, remote-id-reference, part108-proposed-reference |
| **Operations** | **42** | |
| Generated endpoints | 10 | common search forms (e.g. `gsd-calculator`, `flight-time-calculator`, `mah-to-wh`, `overlap-calculator`) composed from operations |
| **Endpoints** | **52** | |

## Risks / Trade-offs

- **[Regulations change (Part 108 finalization likely within months)]** → Data-only update path, review-date CI warnings, and `proposed` status gating.
- **[Endurance estimates still differ from reality]** → Conservative defaults, visible assumptions, and a "calibrate from a test flight" option: the user enters a measured hover time and the tool back-solves the effective FM·η.
- **[Terrain-following depends on DEM quality]** → The DEM name and accuracy are shown per mission, and the user can add a margin.

## Migration Plan

Not applicable. When FAA Part 108 is final, update the reference data status from `proposed` to `in-force` and add its values.

## Open Questions

None that affect the specs.
