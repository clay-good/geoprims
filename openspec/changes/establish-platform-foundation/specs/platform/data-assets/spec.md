## Purpose

Manages the reference datasets geoprims depends on (magnetic models, geoid grids, datum-shift grids, coordinate reference system registry, terrain, basemap) so that they are versioned, integrity-checked, lazily and privately loaded, cached for offline use, and correctly licensed and attributed.

## ADDED Requirements

### Requirement: Asset registry
Every dataset SHALL be described in an asset registry entry containing: `id`, `version`, `title`, `issuer`, `license` (SPDX identifier or named terms), `attribution` (exact required text), `sourceUrl`, `retrievedAt`, `validFrom`/`validTo` (for time-bounded models), `sha256` per file, `bytes` per file, `tiling` (none or scheme), and `loadPolicy` (`bundled`, `on-demand`, `on-demand-tiled`, or `offline-pack-only`). Gridded datasets SHALL be stored as float32 (or the issuer's native precision) and declare the issuer's interpolation method (e.g. biquadratic for NGS NADCON5 and GEOID18), except the GeographicLib-format EGM grids, which declare their 3 mm quantization. Tools SHALL reference assets only by registry `id` and `version`.

#### Scenario: Unregistered asset rejected
- **WHEN** a tool manifest declares an asset id not present in the registry
- **THEN** the build fails naming the tool and asset id

#### Scenario: Attribution rendered
- **WHEN** a tool that used Copernicus DEM data shows its result
- **THEN** the result panel and any export include the registry's exact attribution text for that dataset

### Requirement: Initial dataset set
The registry SHALL include at least the following at first release, with the load policy shown:

| Asset id | Content | Approx. size | Load policy |
|---|---|---|---|
| `wmm2025` | World Magnetic Model 2025 coefficients (degree 12), valid 2025.0–2030.0 | ~5 KB | bundled |
| `wmmhr2025` | WMM High Resolution 2025 (degree 133 crustal) | ~534 KB | on-demand |
| `igrf14` | IGRF-14 coefficients, 1900.0–2030.0 | ~29 KB | on-demand |
| `egm96-15` | EGM96 geoid, 15′ grid | ~2.1 MB | on-demand |
| `egm2008-5` | EGM2008 geoid, 5′ grid, tiled | ~19 MB total | on-demand-tiled |
| `egm2008-2.5` | EGM2008 geoid, 2.5′ grid, tiled | ~75 MB total | on-demand-tiled |
| `egm2008-1` | EGM2008 geoid, 1′ grid, tiled | ~470 MB total | on-demand-tiled (also offered as offline packs) |
| `geoid18` | NGS GEOID18 (NAVD 88 hybrid, CONUS/PR/VI) | per-region | on-demand-tiled |
| `nadcon5-*` | NGS NADCON5 datum-shift grids per region and datum pair | per-grid | on-demand-tiled |
| `crs-registry` | Curated EPSG-derived subset: all CRSs used by any tool (UTM, UPS, SPCS83, SPCS2022-beta, national grids), with definitions and bounds | ≤ 1.5 MB | on-demand |
| `natrf2022-beta` | NGS NSRS 2022 beta frame transformation parameters (NATRF2022 and related frames), versioned by NGS publication date | small | on-demand |
| `geoid2022-beta` | NGS GEOID2022 / NAPGD2022 beta grids, added only when NGS publishes files | per-region | on-demand-tiled |
| `deformation-zones` | Boundaries of crustal deformation regions where rigid-plate motion is inadequate, derived from the NGS HTDP 3.6 velocity-model regions (public domain) | small | bundled with geodesy module |
| `basemap-osm` | Optional self-hosted vector basemap (Protomaps PMTiles extract of OpenStreetMap, zoom 0–7), ODbL with attribution | ≤ 60 MB | on-demand-tiled |
| `leap-seconds` | IERS leap-second table (Bulletin C), public domain | < 5 KB | bundled with time module |
| `tzdb` | IANA time zone database snapshot, public domain | ~400 KB | bundled with time module |
| `tz-boundaries` | Time zone boundary polygons (timezone-boundary-builder), ODbL with attribution | tens of MB | on-demand |
| `ngs-antinfo` | NGS antenna offsets (ANTINFO), public domain | small | on-demand |
| `plss-cadnsdi` | BLM PLSS CadNSDI township and section polygons, per state, public domain (v1.1) | per state | on-demand-tiled (tile ≥ one state) |
| `spcs2022-beta` | NGS SPCS2022 zone definitions (beta), versioned by NGS publication date | small | on-demand |
| `ne-110m`, `ne-50m` | Natural Earth coastlines, borders, graticules (public domain) | ≤ 1 MB, ≤ 5 MB | bundled (110m), on-demand (50m) |
| `dem-glo30` | Copernicus DEM GLO-30, re-tiled and self-hosted | global, tiled | on-demand-tiled |
| `h3-res0`, `s2-faces` | Base-cell and face geometry for index visualization | small | bundled with indexing module |

#### Scenario: Registry completeness check
- **WHEN** the release build runs
- **THEN** each asset listed above exists in the registry with a license, attribution, digest, and load policy

### Requirement: Integrity verification
Every asset file or tile SHALL be verified against its registry SHA-256 digest (or a per-tile digest listed in a signed tile index) before use. A digest mismatch SHALL produce `ASSET_INTEGRITY`, SHALL discard the cached copy, and SHALL NOT fall back to unverified data.

#### Scenario: Corrupted tile
- **WHEN** a cached geoid tile's bytes do not match its digest
- **THEN** the tool returns `ASSET_INTEGRITY`, the tile is evicted, and a retry re-downloads it

### Requirement: Private tile granularity
Tiled assets SHALL use tiles no smaller than 1° × 1° (or an equivalent coarse index), and SHALL be fetched as whole tiles (a byte range, if used, SHALL cover exactly one whole tile), so that fetching never reveals a user's position more precisely than the tile extent. Tiles SHALL be served from the geoprims origin, never from third-party hosts at request time.

#### Scenario: Tile size floor
- **WHEN** the asset build tiles `egm2008-1`
- **THEN** no tile covers less than 1° × 1°

### Requirement: Time-bounded model validity
Assets with a validity window SHALL be enforced by the tools that use them. Requests for an epoch outside the window SHALL return `OUT_OF_DOMAIN` unless the tool explicitly supports extrapolation, in which case the result SHALL include warning `MODEL_EXTRAPOLATED`. After the window closes, the web UI SHALL show a banner on affected tools until an updated model ships.

#### Scenario: WMM after validity
- **WHEN** WMM2025 declination is requested for epoch 2030.5
- **THEN** the tool returns `OUT_OF_DOMAIN` naming the validity window 2025.0–2030.0 and suggesting the successor model if available

### Requirement: Beta and non-official datasets are labeled
Datasets that are not officially adopted by their issuer (for example NGS NATRF2022, NAPGD2022, GEOID2022, and SPCS2022 while in beta) SHALL carry `status: beta` in the registry, and every result that uses them SHALL include warning `NON_OFFICIAL_DATUM` stating that the issuer has not yet adopted the dataset and naming the publication date used.

#### Scenario: SPCS2022 beta warning
- **WHEN** a user converts to an SPCS2022 zone before official adoption
- **THEN** the result includes `NON_OFFICIAL_DATUM` with the NGS publication date of the zone definitions

### Requirement: Offline packs
Users SHALL be able to download named offline packs (for example "Geoid: EGM2008 1′ North America", "Terrain: Colorado", "Magnetic: all models"), see each pack's size before downloading, see total storage used, and delete packs. Downloads SHALL be resumable and verified.

#### Scenario: Pack size shown before download
- **WHEN** a user opens the offline pack manager
- **THEN** each pack lists its compressed download size and installed size

### Requirement: Excluded datasets
The registry SHALL NOT include datasets whose license forbids redistribution or offline use, or whose terms are unclear. Specifically, what3words and the NOAA Enhanced Magnetic Model (EMM) SHALL NOT be included, and the reason SHALL be documented in the registry's exclusions list.

#### Scenario: Exclusion documented
- **WHEN** a contributor proposes adding EMM
- **THEN** the exclusions list shows the documented reason (expired validity and unclear redistribution terms)

### Requirement: Attribution and licenses page
The site SHALL publish a `/licenses` page generated from the registry and the software bill of materials, listing every dataset and dependency with its license and required attribution text.

#### Scenario: Licenses page generated
- **WHEN** a dataset is added to the registry
- **THEN** it appears on `/licenses` in the next build without manual edits

### Requirement: Reference-data files
Small tables of dated standards and regulatory values (fuel reserves, transition altitudes, flight-level tables, drone rules, AASHTO K values, overlap guidance) SHALL live in versioned reference-data files in the repository, not in the asset registry. Each entry SHALL carry a citation, value, unit, effective date, review date, `legalStatus` (`in-force`, `proposed`, `withdrawn`) linked to its sources-ledger row, and authority link, and CI SHALL warn on entries whose review date is more than 12 months old.

#### Scenario: Reference entry lacks a citation
- **WHEN** a reference-data entry has no citation or review date
- **THEN** the build fails naming the entry

### Requirement: Grid edge and no-data rules
Grid interpolation SHALL handle edges explicitly: global grids SHALL wrap in longitude and use pole-aware stencils in the polar rows; regional grids crossing ±180° (e.g. NADCON5 Alaska, Aleutians) SHALL be addressed in a continuous longitude range; any interpolation stencil touching a no-data cell or leaving grid coverage SHALL return `OUT_OF_DOMAIN` rather than a value.

#### Scenario: Stencil touches no-data
- **WHEN** a geoid query's interpolation neighborhood includes a no-data cell
- **THEN** the tool returns `OUT_OF_DOMAIN` naming the grid and coverage gap

#### Scenario: Aleutians across the antimeridian
- **WHEN** a NADCON5 Alaska transformation is requested at longitude 179.5° E
- **THEN** the grid is sampled continuously across ±180° and a result is returned
