# assets

The asset registry and the on-demand reference data (platform/data-assets).

| Path | What |
|---|---|
| `registry.json` | Every dataset: id, version, license, attribution, source, and per-file SHA-256 and size. Tools name assets by id and version only |
| `data/<id>/<version>/<file>` | On-demand files, served at `/assets/<id>/<version>/<file>` on the web and read from disk by the MCP server. A bundled file may instead name its shipped path with `bundledIn` |

A file with `"pack": false` is a pinned build input rather than part of an optional offline download. It remains digest-checked in the registry, but the pack does not download a second copy after the derived runtime file.

How loading works: the core never reads files. A tool that needs data returns `ASSET_UNAVAILABLE` with `{id, version, key}`; the host (`packages/runtime/src/assets.mjs`) finds the file in the registry, checks its SHA-256, supplies it with `gp_asset_put`, and retries. For tiled data, the host verifies the index file's registry digest, Ed25519 signature, pinned key id and public key before trusting the requested tile's digest. A digest, signature, or key mismatch is `ASSET_INTEGRITY` and the bytes are never used.

Bundled datasets (WMM2025, IGRF-14, tzdb, leap seconds) are compiled into their modules and listed with `bundledIn`.

The registry currently contains 11 verified datasets: EGM96-15, WMM2025, WMMHR2025, IGRF-14, NADCON5, the curated CRS registry, the SPCS2022 beta definitions, IERS leap seconds, IANA tzdb, and the Natural Earth 110m and 50m base maps. Each file's recorded byte count and SHA-256 digest is checked against the repository in the runtime test suite. The remaining first-release datasets stay tracked in `openspec/changes/establish-platform-foundation/tasks.md`; they are added only after their real files and build pipelines exist.

`tools/data/tz-boundaries.py` verifies timezone-boundary-builder 2026e's comprehensive GeoJSON ZIP, retains every polygon vertex at microdegree precision, and delta-varint encodes it as the 28.2 MB `boundaries.bin` used by the time-zone lookup. The 51.9 MB source archive stays pinned and digest-checked but is excluded from the 28.3 MB offline pack, so a browser does not download both forms.

`tools/codegen/basemap.py` builds both Natural Earth 5.1.2 map files from five pinned official GeoJSON sources per scale. It delta-encodes coordinates at 0.01° precision. The 76,008-byte 110m map is bundled; the 684,709-byte 50m map is one on-demand file, verified before use and cached after its first request.

`tools/data/geoid-tiles.mjs` is the shared pipeline for global GeographicLib PGM geoids and regional NGS float32 grids. It pins both official EGM2008 archives and their extracted PGM files, plus the corrected 2019-11-26 GEOID18 CONUS and Puerto Rico/U.S. Virgin Islands files. The pipeline checks GEOID18 against all 9 NGS correction points, emits 10° tiles with 2-post interpolation halos, reflects global-grid halos across each pole as GeographicLib does, records every tile's byte count and SHA-256 digest, and signs the canonical index with Ed25519. The Rust core reads the emitted PGM tiles directly and matches its whole-grid evaluator at ordinary points, tile edges, and the North Pole. The registry pins the production public key before a tiled geoid ships.

`tools/data/glo30-tiles.mjs` validates official Copernicus DEM GLO-30 DGED tiles as one-degree, EPSG:4326, RasterPixelIsPoint float32 grids. It rewrites them without resampling as Deflate COGs with the floating-point predictor, confirms GDAL's source and output sample checksums agree, records source and output digests, preserves gaps as missing coverage, and signs the canonical tile index with Ed25519. The index carries the 2024_1 release, EGM2008 vertical datum, DSM warning, published accuracy, exact adapted-data attribution, and required liability sentence. Production registration and hosting remain pending until the full official release is processed and a production signing key and asset bucket are configured.

`tools/data/nadcon5-assets.mjs` packs the 7 first-step regional transformations directly from the official NGS 20160901 latitude and longitude grids. It pins all 14 NGS files, checks the 7 corresponding PROJ-data GeoTIFF digests, preserves every native float32 sample, and refuses any changed input or output. Core tests compare every region with an official NGS NCAT result.

`tools/data/crs-registry.mjs` builds the curated CRS registry from EPSG v13.102 in PROJ 9.9.0 and the NGS SPCS2022 beta definition, bounds, and example-coordinate files published on June 1, 2026. It includes every fixed CRS used by a current tool: WGS 84, Web Mercator, Equidistant Cylindrical, the datum-frame CRSs, all WGS 84 UTM and UPS zones, and all 124 SPCS83 zones. The 953 SPCS2022 definitions remain labeled `beta` with their publication date and reference frame. The pipeline also produces the 953-zone Rust check fixture and 20 public golden vectors in each direction from the official NGS examples.
