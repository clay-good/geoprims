# assets

The asset registry and the on-demand reference data (platform/data-assets).

| Path | What |
|---|---|
| `registry.json` | Every dataset: id, version, license, attribution, source, and per-file SHA-256 and size. Tools name assets by id and version only |
| `data/<id>/<version>/<file>` | On-demand files, served at `/assets/<id>/<version>/<file>` on the web and read from disk by the MCP server |

How loading works: the core never reads files. A tool that needs data returns `ASSET_UNAVAILABLE` with `{id, version, key}`; the host (`packages/runtime/src/assets.mjs`) finds the file in the registry, checks its SHA-256, supplies it with `gp_asset_put`, and retries. A digest mismatch is `ASSET_INTEGRITY` and the bytes are never used.

Bundled datasets (WMM2025, IGRF-14, tzdb, leap seconds) are compiled into their modules and listed with `bundledIn`.
