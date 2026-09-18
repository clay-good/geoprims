# core

The Rust compute core. One Cargo workspace; each domain crate compiles to its own WebAssembly module, and the website and the MCP server load the same bytes.

| Crate | Module | Contents |
|---|---|---|
| `gp-base` | (linked into every module) | Units, angles, errors and warnings, ECMAScript-format JSON, result envelope, Wasm ABI |
| `gp-units` | `base` | The units domain |
| `gp-geodesy`, `gp-navigation`, `gp-geometry`, `gp-aviation`, `gp-drone`, `gp-survey`, `gp-indexing`, `gp-raster`, `gp-time` | one each | Domain tools (being built) |

`fixtures/` holds deliberately bad crates that prove the build gates fail. They are not workspace members.

```bash
cargo test --manifest-path core/Cargo.toml
```
