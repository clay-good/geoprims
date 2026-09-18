## Purpose

Manages the reference datasets geoprims depends on (magnetic models, geoid grids, datum-shift grids, coordinate reference system registry, terrain, basemap) so that they are versioned, integrity-checked, lazily and privately loaded, cached for offline use, and correctly licensed and attributed.

## ADDED Requirements

### Requirement: Asset registry
Every dataset SHALL be described in an asset registry entry containing: `id`, `version`, `title`, `issuer`, `license` (SPDX identifier or named terms), `attribution` (exact required text), `sourceUrl`, `retrievedAt`, `validFrom`/`validTo` (for time-bounded models), `sha256` per file, `bytes` per file, `tiling` (none or scheme), and `loadPolicy` (`bundled`, `on-demand`, or `offline-pack-only`). Tools SHALL reference assets only by registry `id` and `version`.

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
| `egm2008-2.5` | EGM2008 geoid, 2.5′ grid, tiled | ~75 MB total | on-demand by tile |
| `egm2008-1` | EGM2008 geoid, 1′ grid, tiled | ~470 MB total | offline-pack-only or on-demand by tile |
| `geoid18` | NGS GEOID18 (NAVD 88 hybrid, CONUS/PR/VI) | per-region | on-demand by tile |
| `nadcon5-*` | NGS NADCON5 datum-shift grids per region and datum pair | per-grid | on-demand by tile |
| `crs-registry` | Curated EPSG-derived subset: all CRSs used by any tool (UTM, UPS, SPCS83, SPCS2022-beta, national grids), with definitions and bounds | ≤ 1.5 MB | on-demand |
| `spcs2022-beta` | NGS SPCS2022 zone definitions (beta), versioned by NGS publication date | small | on-demand |
| `ne-110m`, `ne-50m` | Natural Earth coastlines, borders, graticules (public domain) | ≤ 1 MB, ≤ 5 MB | bundled (110m), on-demand (50m) |
| `dem-glo30` | Copernicus DEM GLO-30, re-tiled and self-hosted | global, tiled | on-demand by tile |
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
Tiled assets SHALL use tiles no smaller than 1° × 1° (or an equivalent coarse index), so that fetching a tile never reveals a user's position more precisely than the tile extent. Tiles SHALL be served from the geoprims origin, never from third-party hosts at request time.

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
