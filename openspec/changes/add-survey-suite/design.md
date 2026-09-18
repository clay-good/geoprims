## Context

Motivation is in `proposal.md`. Research: `docs/research/03-aviation-drone-survey-formulas.md` §7 (traverse, adjustments, curvature and refraction, earthwork, grid-ground, and survey-foot status). Key facts:

- **US survey foot.** Deprecated January 1, 2023, but still specified for SPCS83 by statute or FRN in about 40 states. SPCS2022 will use meters and international feet only.
- **The "0.0675 m/km²" curvature-and-refraction coefficient** corresponds to k ≈ 0.14, not 0.13.
- **Elevation factor** requires ellipsoid height. Using orthometric height is a common error of tens of meters in h, which is ppm-level in distance.

## Goals / Non-Goals

**Goals:**
- Audit-ready results: every correction listed, every constant named, and calculation sheets exportable.
- Unit safety: feet variants never mix silently.

**Non-Goals:**
- Full network adjustment software. Data-collector native formats. Legal boundary work.

## Decisions

### SV1. Plane coordinates are first-class
Survey tools operate in plane (northing, easting) coordinates with an explicit context: a grid CRS from geodesy, or local ground coordinates with a combined factor. Northing-first order follows survey convention. The UI labels "N" and "E" explicitly to avoid x/y confusion.

### SV2. Adjustments
- Compass, transit, and Crandall are closed-form.
- The experimental least-squares adjustment uses a sparse normal-equations solver in Rust (Cholesky) with a 200-point cap. Error ellipses come from the covariance matrix.
- Outputs are validated against Ghilani's published textbook examples, which serve as golden vectors.

### SV3. Iterative field solutions
Slope staking and catch points use bracketed root-finding (Brent's method) with 0.01-unit convergence and a declared iteration limit, per numeric-determinism.

### SV4. TIN volumes
Delaunay triangulation comes from `geometry/computational`. Volume is the sum of prism volumes between the TIN and the base plane. Stockpile tools cap input at 100,000 points in the browser.

## Tool inventory (targets)

| Group | Operations | Examples |
|---|---|---|
| `cogo` | 20 | direction-parse, inverse, forward, radial-sideshots, traverse-closure, angular-closure, bowditch, transit, crandall, least-squares-2d (experimental), area-by-coordinates, bearing-bearing, bearing-distance, distance-distance, resection, station-offset, point-from-station-offset, perpendicular-foot, azimuth-bearing, deflection-angle |
| `reduction` | 14 | slope-reduction, two-face-mean, curvature-refraction, edm-atmospheric, elevation-factor, combined-factor, grid-to-ground, ground-to-grid, level-run, level-adjust, stadia, inaccessible-height, distance-offset, angle-offset |
| `earthwork` | 14 | average-end-area, prismoidal, section-area, borrow-pit, four-point-average, shrink-swell, haul-loads, grade-convert, rise-run, slope-staking, stockpile-tin, solid-volumes, angle-of-repose-reference, profile-slope |
| `curves` | 10 | circular-curve, degree-of-curve, curve-stationing, curve-layout, spiral, spiral-curve-spiral, vertical-curve, vertical-curve-turning-point, vertical-curve-elevations, sight-distance-length |
| **Operations** | **58** | |
| Generated endpoints | 10 | common search forms: `cubic-yards-calculator`, `percent-grade-to-degrees`, `slope-ratio-to-percent`, `bearing-to-azimuth`, `station-format`, and similar |
| **Endpoints** | **68** | |

## Risks / Trade-offs

- **[Users rely on outputs for legal work]** → The disclaimer states the outputs are computational aids and a licensed professional is responsible. Calculation sheets make review easy.
- **[Textbook sign conventions differ (index error, deflection direction, station-offset sign)]** → Every convention is stated in the result and selectable where the field is split.
- **[Least squares misused as production-grade]** → It stays experimental, capped, and labeled.

## Migration Plan

Not applicable.

## Open Questions

None that affect the specs.
