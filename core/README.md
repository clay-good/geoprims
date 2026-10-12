# core

The Rust compute core. One Cargo workspace; each domain crate compiles to its own WebAssembly module, and the website and the MCP server load the same bytes.

| Crate | Module | Contents |
|---|---|---|
| `gp-base` | (linked into every module) | Units, angles, errors and warnings, ECMAScript-format JSON, result envelope, tool runtime and manifests, golden-vector runner, Wasm ABI |
| `gp-units` | `base` | The units domain: 22 operations and 35 pair endpoints |
| `gp-geodesy`, `gp-navigation`, `gp-geometry`, `gp-aviation`, `gp-drone`, `gp-survey`, `gp-indexing`, `gp-time` | one each | Domain tools (being built) |
| `gp-raster` | `raster` | Spectral indices, terrain analysis, and a bounded GeoTIFF decoder that reads private host-supplied bytes and retains no-data, EPSG, and affine grid metadata |

`vectors/` holds the golden vectors, one JSON Lines file per tool id (format in `gp-base/src/vectors.rs`).

`fixtures/` holds deliberately bad crates that prove the build gates fail. They are not workspace members.

```bash
cargo test --manifest-path core/Cargo.toml
```
