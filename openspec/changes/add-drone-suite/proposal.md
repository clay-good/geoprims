## Why

Drone mapping and inspection teams plan missions with vendor calculators tied to one app or one camera, marketing-funnel GSD pages, and spreadsheets. The frequent errors:

- 35 mm-equivalent focal length used in GSD
- height above takeoff treated as height above terrain
- ideal hover power used without losses, which overstates endurance by 30–50%
- regulations quoted from memory, e.g. treating FAA Part 108 as final when it is still proposed in September 2026

geoprims gives drone operators, surveyors, and UAM engineers vendor-neutral, exact mission math and exportable flight patterns, drawn on the map, with regulatory limits as dated references.

Depends on: `establish-platform-foundation`, `add-geodesy-suite`, `add-navigation-and-geometry` (buffers, routes), `add-aviation-suite` (atmosphere, density).

## What Changes

Adds the `drone` domain: about 42 operations and 42 tool ids (inventory in `design.md`).

- **Photogrammetry:** GSD and its inverse (altitude for a target GSD), image footprint, overlap to trigger distance and interval, flight-line spacing, motion blur and maximum shutter time, oblique GSD, terrain-aware overlap, image count, and the ASPRS Edition 2 accuracy calculator.
- **Mission patterns:** survey grid (lawnmower) over a polygon, crosshatch, corridor, orbit/point of interest, facade scan, geofence generation with buffers, and export to KML, CSV, and GeoJSON waypoints.
- **Endurance and power:** momentum-theory hover power with figure of merit and efficiencies, battery energy and usable capacity, reserves, density-altitude and temperature derating, payload impact, return-to-home energy budget against wind, and C-rate.
- **Operations reference:** FAA Part 107 limits (dated), Remote ID, FAA Part 108 (labeled "Proposed"), and EASA open-category subcategories and class marks (dated). Also kinetic-energy checks, the structure-radius altitude rule, and AGL/MSL conversion for limits.

## Capabilities

### New Capabilities

- `drone/photogrammetry`: Camera and overlap geometry for mapping missions, and mapping accuracy standards.
- `drone/mission-patterns`: Generation of flight patterns and geofences as exportable waypoints.
- `drone/endurance-and-power`: Multirotor power, battery, and endurance estimation.
- `drone/operations-reference`: Dated regulatory limits and compliance calculators.

### Modified Capabilities

None.

## Non-goals

- Controlling drones or uploading missions to flight controllers. Export files are provided; nothing connects to aircraft.
- Airspace authorization (LAANC), live airspace, or TFR data.
- Fixed-wing aerodynamic endurance models beyond a labeled generic estimate.
- Legal advice. Regulatory references are dated summaries with links to the authority.

## Impact

- `core/gp-drone` crate.
- Reference data: dated regulatory summaries (FAA 14 CFR Part 107 and Part 89; the Part 108 NPRM, Federal Register, August 7, 2025; EASA Regulation (EU) 2019/947 and 2019/945 as amended).
- Mission export formats: KML, CSV, and GeoJSON (vendor-neutral waypoint lists).
