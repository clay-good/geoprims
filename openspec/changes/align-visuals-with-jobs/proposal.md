## Why

A visual on a tool page should answer a question the reader has, drawn from their own numbers. Some did not. The GSD page (`/drone/photogrammetry/gsd/`) showed a world map: the tool has no location, so "Try the example" framed an empty globe, and editing the camera changed nothing on it. An audit of every page that carries the map, running each tool's own example through the core, found nine like it:

| Page | What the map showed | Why |
|---|---|---|
| GSD | Empty world | No location; the manifest declared a polygon it could not draw |
| COGO forward, area by coordinates, traverse closure | Empty world | Northing and easting on a local grid are not places on the earth |
| MGRS to lat/lon | Empty world | The declared cell mapped only its size, so nothing drew |
| S2 covering | Empty world | S2 cells had no outline source |
| Glide range, true to magnetic, SPCS zone lookup | Empty world | The location is optional and the example leaves it out |

Seven more tools declared a vector diagram and never drew one, including the photo overlap tool, whose answer is a picture.

Depends on: `build-web-experience` (`web/map-canvas`), which this narrows.

## What Changes

- **A visual earns its place.** It draws the reader's inputs or the answer, redraws on every result, and helps with the job. A tool with nothing to draw shows no visual. The answer stays first.
- **The map is for places.** A page carries the map only when its inputs or outputs are places on the earth. If the current inputs leave it nothing to draw, it hides, with one line saying how to get it back ("Enter a latitude and longitude to see this on the map.").
- **Diagrams where the geometry is not on the map.** New diagrams, each drawn from core values:
  - Drone: camera footprint for GSD and height-for-GSD, photo overlap for the trigger tool, oblique footprint, the Part 107 ceiling near a structure, and the return-to-home energy budget.
  - Flight: wind triangles for find-wind, course-from-heading, and airspeed-for-groundspeed, plus the holding racetrack with wind-corrected headings and outbound time.
  - Direction and height: a true and magnetic compass, the turn between two bearings, and horizon dip.
  - Survey: plane sketches for COGO forward and inverse, area by coordinates, and traverse closure.
- **S2 cells draw like H3 cells.** The covering, cell lookup, cell info, and neighbor pages outline their cells. MGRS to lat/lon marks its point.
- **A gate.** `apps/web/test/visual-purpose.test.mjs` runs every map page's example through the core and fails if the map would be empty. It also fails if a tool declares a vector diagram and draws none. Counts are asserted, so the test cannot pass by reaching nothing.

## Non-goals

- A visual on every page. About 200 tools answer with a number or a table, and a picture would only slow the reader down.
- New map layers, basemaps, or tile sources.
- Reworking the 20 `gauge` declarations that nothing renders. Task 6 settles them tool by tool.

## Impact

Manifest `visualization` changes for 15 tools; no result, vector, or tool version changes. The canvas and diagram code is in `apps/web/src/components/MapCanvas.svelte`, `apps/web/src/lib/map/layers.js`, and `apps/web/src/lib/diagrams.js`.
