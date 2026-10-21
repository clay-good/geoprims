# core

The Rust compute core. One Cargo workspace; each domain crate compiles to its own WebAssembly module, and the website and the MCP server load the same bytes.

| Crate | Module | Contents |
|---|---|---|
| `gp-base` | (linked into every module) | Units, angles, errors and warnings, ECMAScript-format JSON, result envelope, tool runtime and manifests, golden-vector runner, Wasm ABI |
| `gp-units` | `base` | The units domain: 22 operations and 35 pair endpoints |
| `gp-geodesy`, `gp-navigation`, `gp-geometry`, `gp-aviation`, `gp-drone`, `gp-survey`, `gp-indexing`, `gp-time` | one each | Domain tools (being built) |
| `gp-raster` | `raster` | Spectral indices, terrain analysis, bounded GeoTIFF/COG window decoding from whole or range-backed private files, per-window index processing, nearest/bilinear grid resampling, and strip-streamed single-band export that retains no-data and georeferencing |
| `gp-astro` | (library, linked into `time`) | IAU 2006/2000A precession-nutation and sidereal time from the IERS tables; apparent places of the Sun and planets (VSOP87), the Moon (ELP 2000-82B), and the 57 navigational stars and Polaris (Hipparcos), checked against ERFA, JPL Horizons, and USNO |

`vectors/` holds the golden vectors, one JSON Lines file per tool id (format in `gp-base/src/vectors.rs`).

`fixtures/` holds deliberately bad crates that prove the build gates fail. They are not workspace members.

```bash
cargo test --manifest-path core/Cargo.toml
```
