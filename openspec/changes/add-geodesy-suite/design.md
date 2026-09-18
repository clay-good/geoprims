## Context

Motivation is in `proposal.md`. Research: `docs/research/01-geodesy-and-reference-data.md`. State of the field (September 2026):

- **Karney (GeographicLib) is the accuracy reference** for geodesics, Transverse Mercator (TM), Universal Polar Stereographic (UPS), MGRS, and geoid evaluation. The Rust port covers geodesics; TM, UPS, MGRS, Rhumb, and the geoid evaluator must be ported from C++ (MIT).
- **PROJ 9.x is the reference** for other projections and EPSG transformations. It does not ship to browsers (foundation D4).
- **US datum modernization is not yet official.** NATRF2022, NAPGD2022, GEOID2022, and SPCS2022 are still beta, with adoption expected late 2026 to early 2027. SPCS2022 zone definitions are still being registered with EPSG.
- **Magnetic models.** WMM2025 (valid 2025–2030), WMMHR2025 (degree 133), and IGRF-14 are current. EMM is excluded.

## Goals / Non-Goals

**Goals:**
- Reference-grade accuracy (nanometer-level for projections and geodesics, millimeter-level agreement with NGS tools) with every assumption surfaced as a warning.
- A single parsing layer that every other domain reuses.

**Non-Goals:**
- Full EPSG coverage. Tidal datums (VDatum). Crustal-field modeling beyond WMMHR.

## Decisions

### G1. Port GeographicLib classes rather than re-derive
TransverseMercator (6th-order series), TransverseMercatorExact, PolarStereographic, UTMUPS, MGRS, Geoid (PGM grid reader with cubic interpolation), MagneticModel/SphericalHarmonic, Rhumb, and DMS are ported line-by-line from GeographicLib C++ 2.x into `gp-geodesy`. Each keeps its upstream test cases and adds a differential test against the C++ build.

Alternative: write from textbook formulas (Snyder, the Redfearn series). Rejected: the accuracy is worse (the Redfearn series degrades to millimeters far from the central meridian) and there is no oracle parity.

### G2. EPSG-parameterized projection engine
Other projections are implemented from EPSG Guidance Note 7-2 formulas with EPSG parameter names. The curated `crs-registry` asset stores CRS definitions in a compact JSON form (PROJJSON subset). Registry scope:

- All UTM (WGS 84 and NAD83) and UPS CRSs
- All SPCS83 zones (NAD83, NAD83(HARN), NAD83(2011)) in each legal unit
- SPCS2022 beta zones
- National grids with stable, published parameters (British National Grid via its 7-parameter approximation, labeled low-accuracy; Swiss LV95; Dutch RD; Australian MGA2020; Canadian UTM NAD83(CSRS))
- Web Mercator

### G3. Explicit frames and epochs everywhere
Coordinates in tool I/O carry an optional `frame` (e.g. `NAD83(2011)`) and `epoch`. Transformation-aware tools refuse to guess silently: they assume the documented default and warn (`REALIZATION_ASSUMED`, `FRAME_MISMATCH`). This is the main correctness feature of the suite.

### G4. Datum transformation paths
- **Helmert parameter sets:** IERS (ITRF↔ITRF), NGS (ITRF2020/IGS20↔NAD83(2011), the same values HTDP 3.6 uses), NGA (WGS 84 realizations), and EPSG for legacy datums.
- **Grids:** NADCON5 for NAD27/NAD83 realizations, packaged from NGS sources into ≥ 1° tiles.
- **Plate motion:** the ITRF2020 plate motion model (rigid plates) plus user site velocity. The HTDP crustal-deformation velocity grid is out of scope for v1 and is flagged by `DEFORMATION_ZONE`.
- **Path selection:** a shortest path over a frame graph weighted by stated accuracy. Every step is reported in the result.

### G5. Geoid grids: GeographicLib PGM format, tiled
PGM grids (16-bit, 3 mm quantization) are re-tiled into 1° × 1° or coarser tiles, each carrying its digest. Cubic interpolation needs a 4 × 4 neighborhood, so tiles carry a 2-cell overlap margin, which keeps edge evaluation within a single tile fetch. GEOID18 is converted from NGS binary grids to the same tile format.

### G6. Magnetic engine
A spherical-harmonic evaluator (Schmidt semi-normalized, geodetic → geocentric conversion, secular variation) is shared by WMM, WMMHR (degree 133), and IGRF. Isogonic overlays are computed on the device by evaluating a coarse grid (e.g. 1°) and contouring with marching squares in a worker.

### G7. MGRS strictness
Encode by truncation; decode to the square's south-west corner and center; allow band-letter tolerance within 5 nm; reject nonexistent 100 km squares. This follows GeographicLib, which is stricter and better documented than the npm `mgrs` package.

## Tool inventory (targets)

| Group | Operations | Examples |
|---|---|---|
| `parse` | 8 | parse-auto, parse-dms, parse-ddm, format-dd/dms/ddm, angle-arithmetic, bearing-difference |
| `ellipsoid` | 5 | parameters, radii-of-curvature, auxiliary-latitude, meridian-arc, degree-lengths |
| `frames` | 8 | geodetic↔ecef (2), ecef↔enu (2), enu↔ned, aer↔enu (2), local-rotation-matrix |
| `datum` | 7 | helmert-7, helmert-14, frame-transform (path), nadcon5, plate-motion, wgs84-vs-nad83, legacy-shift |
| `utm`/`ups`/`spcs`/`crs` + methods | 26 | utm f/i/zone, ups f/i, spcs83 f/i, spcs2022 f/i, spc-zone-lookup, tm f/i, lcc f/i, albers f/i, hotine f/i, web-mercator f/i, azimuthal-equidistant f/i, crs-search, crs-transform, convergence, scale-factor |
| `grid-ref` | 12 | mgrs f/i, usng f/i, mgrs-precision, grid-zone-lookup, maidenhead f/i, gars f/i, georef f/i |
| `height` | 5 | geoid-undulation, h-to-H, H-to-h, height-reference-convert, geoid-compare |
| `magnetic` | 7 | field-elements, true-to-magnetic, magnetic-to-true, grivation, model-compare, uncertainty-zone, isogonic-map |
| **Operations** | **78** | |
| Generated conversion pairs | 84 | allow-listed pairs among DD, DMS, DDM, UTM, UPS, MGRS, USNG, Maidenhead, GARS, GEOREF, ECEF, Web Mercator, SPCS83 (e.g. `dms-to-mgrs`, `mgrs-to-dd`, `utm-to-mgrs`, `dd-to-maidenhead`, `ecef-to-dms`) |
| **Endpoints** | **162** | |

## Risks / Trade-offs

- **[SPCS2022 definitions change before adoption]** → Versioned `spcs2022-beta` asset, with results carrying the NGS publication date. Refreshing is a data change.
- **[HTDP deformation model not included; western US propagation error at the cm–dm level]** → `DEFORMATION_ZONE` warning plus site-velocity override. A future change can add the velocity grid.
- **[Geoid tile fetches reveal coarse location]** → 1° tiles minimum, same-origin, and offline packs (privacy spec).
- **[Porting errors in GeographicLib classes]** → Differential tests against C++ on 10,000+ points per class, plus the upstream test suites.
- **[Users treat EGM2008 heights as NAVD 88]** → Height results always name the vertical datum the model realizes. GEOID18 is suggested inside US coverage.

## Migration Plan

Not applicable. When NGS formally adopts NSRS 2022, update the registry status (beta → official). The `NON_OFFICIAL_DATUM` warnings then stop without code changes.

## Open Questions

- Final SPCS2022 zone count and EPSG codes: data-only.
