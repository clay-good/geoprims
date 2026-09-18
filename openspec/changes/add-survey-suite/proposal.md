## Why

Field surveyors and civil engineers do coordinate geometry (COGO), traverse closure, instrument reductions, earthwork volumes, and curve layout daily. The tools for it are expensive desktop suites, data-collector software locked to one vendor, or spreadsheets passed between crews. Classic errors recur:

- mixing US survey feet and international feet
- using orthometric height in the elevation factor
- pairing the 0.0675 curvature-and-refraction coefficient with the wrong refraction constant
- taking the prismoidal middle area as the average of the end areas

geoprims gives surveyors free, offline, exact tools that produce calculation sheets suitable for field notes and project files.

Depends on: `establish-platform-foundation`, `add-geodesy-suite` (State Plane, scale factors, heights), `add-navigation-and-geometry` (areas, geometry).

## What Changes

Adds the `survey` domain: about 57 operations and 67 endpoints (inventory in `design.md`).

- **COGO and traverse:** inverse and forward (bearing/azimuth and distance), quadrant bearing parsing, traverse closure (angular and linear misclosure, precision ratio), compass (Bowditch), transit, and Crandall adjustments, a small least-squares adjustment (experimental), area by coordinates, intersections (bearing-bearing, bearing-distance, distance-distance), three-point resection, and offsets.
- **Instrument reductions:** slope to horizontal and vertical from zenith or vertical angle, HI/HR elevation, curvature and refraction, EDM atmospheric (ppm) correction, sea-level/elevation factor, grid-ground combined factor, trigonometric leveling, differential level-loop closure and adjustment, stadia, and total-station offset shots.
- **Earthwork and grade:** average end area, prismoidal, borrow-pit (grid) volumes, cut/fill from cross sections, shrink/swell, grade in %/ratio/degrees, slope staking (catch points), angle of repose reference, stockpile volumes, and profile slope analysis.
- **Alignment curves:** horizontal circular curves (all elements, arc and chord definitions of the degree of curve, stationing, deflection-angle layout), spiral curves, and parabolic vertical curves (high/low point, station elevations, K values).

## Capabilities

### New Capabilities

- `survey/cogo-and-traverse`: Coordinate geometry, traverses, adjustments, intersections, and resection.
- `survey/instrument-reductions`: Reducing field observations to horizontal, vertical, and grid quantities.
- `survey/earthwork-and-grade`: Volumes, grades, and slope staking.
- `survey/alignment-curves`: Horizontal, spiral, and vertical curve geometry and layout.

### Modified Capabilities

None.

## Non-goals

- Legal boundary determination, deed interpretation, or plat preparation.
- Full network least-squares adjustment software (e.g. STAR*NET-class). Only a small experimental adjustment is included.
- Instrument data-collector file formats beyond CSV/JSON import in v1.
- Structural or geotechnical design (no angle-of-repose or other material tables are provided).

## Impact

- `core/gp-survey` crate.
- References: standard surveying texts (Ghilani & Wolf, *Elementary Surveying*), NGS publications for scale factors, NIST/NGS for survey-foot policy, and AASHTO for vertical curve design values, cited as user-entered inputs, for vertical curve K-value references (reference only).
