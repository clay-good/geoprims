# geoprims

**Exact, cited math for maps, flight, drones, and surveying. It runs on your device.**

Density altitude before a flight. Ground sample distance for a drone survey. A traverse closure, a UTM conversion, an H3 cell, sunrise at a field site. geoprims answers questions like these in a plain sentence, then shows how it got there.

- **For people:** a fast, free website. The answer comes first, with a map or diagram and a "How we got this" panel showing the formula with your numbers and its sources.
- **For AI agents:** a local MCP server that runs the same calculators and returns the same results, byte for byte. No dependencies, no network. See [mcp/README.md](mcp/README.md).

## Why trust it

- **Accurate.** Reference methods (Karney geodesics, not haversine), tested against GeographicLib, PROJ, H3, S2, and published worked examples.
- **Cited.** Every result names its source, edition, and section. The build fails if a source goes out of date.
- **Honest.** Assumptions like datum, true vs. magnetic north, and rules of thumb show up as warnings, not fine print.
- **Private.** No ads, accounts, tracking, or server-side math. Your inputs never leave your device.

## What it covers

Geodesy, navigation, geometry, aviation, drones, surveying, spatial indexing, raster and terrain, time, and units.

## Status

In development and not yet released. Hundreds of tools work locally; only the ones that pass full verification will be published. The plan and specs live in [`openspec/changes/`](openspec/changes/).

## Build and run

You need [rustup](https://rustup.rs), binaryen's `wasm-opt`, and Node 22 or newer.

```bash
npm run build
```

```bash
node mcp/server.mjs
```

To contribute, start with [AGENTS.md](AGENTS.md).

## Disclaimer

geoprims is a planning and education aid. It is not certified for navigation and is not a legal survey. Check operational values against official sources, your aircraft's POH/AFM, and current regulations.

## License

[MIT](LICENSE)
