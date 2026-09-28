# assets

The asset registry and the on-demand reference data (platform/data-assets).

| Path | What |
|---|---|
| `registry.json` | Every dataset: id, version, license, attribution, source, and per-file SHA-256 and size. Tools name assets by id and version only |
| `data/<id>/<version>/<file>` | On-demand files, served at `/assets/<id>/<version>/<file>` on the web and read from disk by the MCP server. A bundled file may instead name its shipped path with `bundledIn` |

How loading works: the core never reads files. A tool that needs data returns `ASSET_UNAVAILABLE` with `{id, version, key}`; the host (`packages/runtime/src/assets.mjs`) finds the file in the registry, checks its SHA-256, supplies it with `gp_asset_put`, and retries. A digest mismatch is `ASSET_INTEGRITY` and the bytes are never used.

Bundled datasets (WMM2025, IGRF-14, tzdb, leap seconds) are compiled into their modules and listed with `bundledIn`.

The registry currently contains 8 verified datasets: EGM96-15, WMM2025, WMMHR2025, IGRF-14, NADCON5, IERS leap seconds, IANA tzdb, and the bundled Natural Earth 110m base map. Each file's recorded byte count and SHA-256 digest is checked against the repository in the runtime test suite. The remaining first-release datasets stay tracked in `openspec/changes/establish-platform-foundation/tasks.md`; they are added only after their real files and build pipelines exist.

`tools/data/geoid-tiles.mjs` is the shared pipeline for global GeographicLib PGM geoids. It pins both official EGM2008 archives and their extracted PGM files, emits 10° tiles with 2-post interpolation halos, records every tile's byte count and SHA-256 digest, and signs the canonical index with Ed25519. The registry pins the production public key before a tiled geoid ships.
