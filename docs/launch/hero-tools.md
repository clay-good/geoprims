# Hero tools: launch checklist

The ~30 hero tools from the [launch plan](../../openspec/changes/plan-launch-and-value-proof/design.md) (L2), each against the launch-ready bar (L4). A box is checked only when its gate passes in the build.

- **Stable:** passes `tools/trust/promotion.mjs` (derivation note, independent worked example, invariant and differential tests, 20+ vectors, search reachability).
- **Reviewed:** a signed row in [review-signoffs.md](../review-signoffs.md). Until then, every page says "Not yet independently reviewed."
- **Usable, mobile, findable:** the 5-second and task tests, the mobile sweep, and the content minimums with an OG image and explainer link. The mobile sweep is built (`apps/web/test/browser/mobile.test.mjs`): every hero page reflows at 320 CSS px and at 640, which is 1280 at 200% zoom, in WebKit, with no sideways scroll. The rest is not, so no tool is checked: the 5-second and task tests need practitioners, there are no OG images yet (the build has no renderer for them, and the two the design names would be the first dependencies this build has), and three kinds of control are still under the 48 x 48 the theme asks for, for gloved use — the chips, the copy buttons, and the unit pickers. The header button, the theme toggle, and the search button were under it too, from hardcoding a height instead of the `--target` the styles define, and now use it.
- **MCP and report:** the both-surfaces gate and the report button. Every stable tool passes both-surfaces, and every tool has the report button.
- **Shows its work:** the tool records a trace, so the page and `explain: true` both show the formula with this call's values in it; a decoder shows every coded group beside what it says, and a tool whose answer is a list of rows shows the rows. Checked against the build.

| Audience | Hero tool | Tool id | Stable | Reviewed | Usable / mobile / findable | Shows its work |
|---|---|---|---|---|---|---|
| Pilots | Crosswind and headwind | `aviation.wind.runway-components` | [x] | [ ] | [ ] | [x] |
| Pilots | Density altitude | `aviation.altimetry.density-altitude` | [x] | [ ] | [ ] | [x] |
| Pilots | Pressure altitude | `aviation.altimetry.pressure-altitude` | [x] | [ ] | [ ] | [x] |
| Pilots | E6B wind triangle | `aviation.wind.heading-groundspeed` | [x] | [ ] | [ ] | [x] |
| Pilots | Sunrise, sunset, twilight | `time.sun.events` | [x] | [ ] | [ ] | [x] |
| Pilots | The four nights | `time.sun.aviation-nights` | [x] | [ ] | [ ] | [x] |
| Pilots | METAR decoder | `aviation.weather.metar-decode` | [x] | [ ] | [ ] | [x] |
| Pilots | TAF decoder | `aviation.weather.taf-decode` | [x] | [ ] | [ ] | [x] |
| Pilots | Winds-aloft decoder | `aviation.weather.fb-winds-decode` | [x] | [ ] | [ ] | [x] |
| Pilots | Holding entry | `aviation.ifr.hold-entry` | [x] | [ ] | [ ] | [x] |
| Pilots | Zulu time | `time.scale.utc-offset` | [x] | [ ] | [ ] | [x] |
| Pilots | Weight and balance | `aviation.loading.weight-balance` | [x] | [ ] | [ ] | [x] |
| Pilots | Top of descent | `aviation.performance.top-of-descent` | [x] | [ ] | [ ] | [x] |
| Pilots | Visual descent point | `aviation.performance.vdp` | [x] | [ ] | [ ] | [x] |
| Drone | GSD | `drone.photogrammetry.gsd` | [x] | [ ] | [ ] | [x] |
| Drone | Altitude for a GSD | `drone.photogrammetry.altitude-for-gsd` | [x] | [ ] | [ ] | [x] |
| Drone | Overlap and trigger | `drone.photogrammetry.trigger` | [x] | [ ] | [ ] | [x] |
| Drone | Image count | `drone.photogrammetry.image-count` | [ ] | [ ] | [ ] | [x] |
| Drone | Flight time | `drone.power.endurance` | [x] | [ ] | [ ] | [x] |
| Drone | mAh to Wh | `drone.power.battery-energy` | [x] | [ ] | [ ] | [x] |
| Drone | Mapping sun window | `time.sun.mapping-window` | [x] | [ ] | [ ] | [x] |
| Drone | VLOS guidance | `drone.sensors.vlos` | [x] | [ ] | [ ] | [x] |
| Drone | Part 107 altitude | `drone.ops.part107-altitude` | [x] | [ ] | [ ] | [x] |
| Surveyors | Coordinate converter (DMS ↔ decimal) | `geodesy.parse.coordinates` | [x] | [ ] | [ ] | [x] |
| Surveyors | UTM | `geodesy.utm.forward`, `geodesy.utm.inverse` | [x] | [ ] | [ ] | [x] |
| Surveyors | MGRS | `geodesy.grid-ref.mgrs-forward`, `geodesy.grid-ref.mgrs-inverse` | [x] | [ ] | [ ] | [x] |
| Surveyors | State plane | `geodesy.spcs.spcs83-forward`, `geodesy.spcs.spcs83-inverse` | [x] | [ ] | [ ] | [x] |
| Surveyors | Geoid and orthometric height | `geodesy.geoid.geoid-height` | [x] | [ ] | [ ] | [x] |
| Surveyors | Grid ↔ ground combined factor | `survey.reduction.combined-factor` | [x] | [ ] | [ ] | [x] |
| Surveyors | Traverse closure | `survey.cogo.traverse-closure` | [x] | [ ] | [ ] | [x] |
| Surveyors | Deed plotter | `survey.land.deed-plot` | [x] | [ ] | [ ] | [x] |
| Surveyors | Horizontal curve | `survey.curves.circular-curve` | [x] | [ ] | [ ] | [x] |
| Surveyors | Vertical curve | `survey.curves.vertical-curve` | [x] | [ ] | [ ] | [x] |
| Surveyors | Acreage from coordinates | `survey.cogo.area-by-coordinates` | [x] | [ ] | [ ] | [x] |
| Developers | H3 cell | `indexing.h3.lat-lng-to-cell` | [x] | [ ] | [ ] | [x] |
| Developers | H3 k-ring | `indexing.h3.grid-disk` | [x] | [ ] | [ ] | [x] |
| Developers | Tile and quadkey | `indexing.tile.from-point` | [x] | [ ] | [ ] | [x] |
| Developers | Tile bounds | `indexing.tile.bounds` | [x] | [ ] | [ ] | [x] |
| Developers | Geohash | `indexing.geohash.encode` | [x] | [ ] | [ ] | [x] |
| Developers | Geodesic distance | `navigation.geodesic.inverse` | [x] | [ ] | [ ] | [x] |
| Developers | Haversine (vs geodesic) | `navigation.geodesic.haversine` | [x] | [ ] | [ ] | [x] |
| Developers | Magnetic declination | `geodesy.magnetic.declination` | [x] | [ ] | [ ] | [x] |

Stable: 41 of 42 rows. Shows its work: 42 of 42 rows. `tools/trust/hero.test.mjs` checks every id, Stable box, and "Shows its work" box against the build.

## Waiting on a published worked example

The stable bar needs an independent, published worked example. These rows are held back until one turns up, rather than citing an example we computed ourselves. The mapping sun window left this list on 2026-09-23: its reference is pvlib's independent implementation of the same NREL algorithm, checked against the core at 250 points in `spa_parity.rs`, which is the same standing as the other tools verified against a separate implementation rather than a printed worked example. Flight time left it the same day: Bauersfeld and Scaramuzza (IEEE RA-L, 2022, Sec. VII-E) work the energy-over-power step for a DJI Mavic 3.

| Tool | Searched (2026-09-19) |
|---|---|
| Holding entry | AIM 5-3-8 and FAA-H-8083-15B define the sectors only by figure; the FAA instrument sample tests have no entry question |
| Part 107 altitude | 14 CFR 107.51, AC 107-2A, FAA-G-8082-22, the UAG sample test, and the 2016 final rule preamble state the rule without a numeric case |
| Image count | Searched again 2026-09-23. Penn State GEOG 892, the Wolf-style flight map example, and University of Washington CEE 424 all put the first and last lines at or near the edges (lines = width / spacing + 1, or a 0.2G to 0.25G margin) and add extra photos past each end; the tool, per the survey-grid spec, centers ⌈width / spacing⌉ lines and takes ⌊length / spacing⌋ + 1 photos per line, so none of them is a worked example of its method. Overlap and trigger left this list on 2026-09-23 with the Penn State GEOG 892 example |
