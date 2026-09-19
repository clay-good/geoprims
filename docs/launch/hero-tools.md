# Hero tools: launch checklist

The ~30 hero tools from the [launch plan](../../openspec/changes/plan-launch-and-value-proof/design.md) (L2), each against the launch-ready bar (L4). A box is checked only when its gate passes in the build.

- **Stable:** passes `tools/trust/promotion.mjs` (derivation note, independent worked example, invariant and differential tests, 20+ vectors, search reachability).
- **Reviewed:** a signed row in [review-signoffs.md](../review-signoffs.md). Until then, every page says "Not yet independently reviewed."
- **Usable, mobile, findable:** the 5-second and task tests, the mobile sweep, and the content minimums with an OG image and explainer link. These gates are not built yet, so no tool is checked.
- **MCP and report:** the both-surfaces gate and the report button. Every stable tool passes both-surfaces, and every tool has the report button.

| Audience | Hero tool | Tool id | Stable | Reviewed | Usable / mobile / findable |
|---|---|---|---|---|---|
| Pilots | Crosswind and headwind | `aviation.wind.runway-components` | [x] | [ ] | [ ] |
| Pilots | Density altitude | `aviation.altimetry.density-altitude` | [x] | [ ] | [ ] |
| Pilots | Pressure altitude | `aviation.altimetry.pressure-altitude` | [x] | [ ] | [ ] |
| Pilots | E6B wind triangle | `aviation.wind.heading-groundspeed` | [x] | [ ] | [ ] |
| Pilots | Sunrise, sunset, twilight | `time.sun.events` | [x] | [ ] | [ ] |
| Pilots | The four nights | `time.sun.aviation-nights` | [x] | [ ] | [ ] |
| Pilots | METAR decoder | `aviation.weather.metar-decode` | [x] | [ ] | [ ] |
| Pilots | TAF decoder | `aviation.weather.taf-decode` | [x] | [ ] | [ ] |
| Pilots | Winds-aloft decoder | `aviation.weather.fb-winds-decode` | [x] | [ ] | [ ] |
| Pilots | Holding entry | `aviation.ifr.hold-entry` | [ ] | [ ] | [ ] |
| Pilots | Zulu time | `time.scale.utc-offset` | [x] | [ ] | [ ] |
| Pilots | Weight and balance | `aviation.loading.weight-balance` | [x] | [ ] | [ ] |
| Pilots | Top of descent | `aviation.performance.top-of-descent` | [x] | [ ] | [ ] |
| Pilots | Visual descent point | `aviation.performance.vdp` | [x] | [ ] | [ ] |
| Drone | GSD | `drone.photogrammetry.gsd` | [x] | [ ] | [ ] |
| Drone | Altitude for a GSD | `drone.photogrammetry.altitude-for-gsd` | [x] | [ ] | [ ] |
| Drone | Overlap and trigger | `drone.photogrammetry.trigger` | [ ] | [ ] | [ ] |
| Drone | Image count | `drone.photogrammetry.image-count` | [ ] | [ ] | [ ] |
| Drone | Flight time | `drone.power.endurance` | [ ] | [ ] | [ ] |
| Drone | mAh to Wh | `drone.power.battery-energy` | [x] | [ ] | [ ] |
| Drone | Mapping sun window | `time.sun.mapping-window` | [ ] | [ ] | [ ] |
| Drone | VLOS guidance | `drone.sensors.vlos` | [x] | [ ] | [ ] |
| Drone | Part 107 altitude | `drone.ops.part107-altitude` | [ ] | [ ] | [ ] |
| Surveyors | Coordinate converter (DMS ↔ decimal) | `geodesy.parse.coordinates` | [x] | [ ] | [ ] |
| Surveyors | UTM | `geodesy.utm.forward`, `geodesy.utm.inverse` | [x] | [ ] | [ ] |
| Surveyors | MGRS | `geodesy.grid-ref.mgrs-forward`, `geodesy.grid-ref.mgrs-inverse` | [x] | [ ] | [ ] |
| Surveyors | State plane | `geodesy.spcs.spcs83-forward`, `geodesy.spcs.spcs83-inverse` | [x] | [ ] | [ ] |
| Surveyors | Geoid and orthometric height | `geodesy.geoid.geoid-height` | [x] | [ ] | [ ] |
| Surveyors | Grid ↔ ground combined factor | `survey.reduction.combined-factor` | [x] | [ ] | [ ] |
| Surveyors | Traverse closure | `survey.cogo.traverse-closure` | [x] | [ ] | [ ] |
| Surveyors | Deed plotter | `survey.land.deed-plot` | [x] | [ ] | [ ] |
| Surveyors | Horizontal curve | `survey.curves.circular-curve` | [x] | [ ] | [ ] |
| Surveyors | Vertical curve | `survey.curves.vertical-curve` | [x] | [ ] | [ ] |
| Surveyors | Acreage from coordinates | `survey.cogo.area-by-coordinates` | [x] | [ ] | [ ] |
| Developers | H3 cell | `indexing.h3.lat-lng-to-cell` | [x] | [ ] | [ ] |
| Developers | H3 k-ring | `indexing.h3.grid-disk` | [x] | [ ] | [ ] |
| Developers | Tile and quadkey | `indexing.tile.from-point` | [x] | [ ] | [ ] |
| Developers | Tile bounds | `indexing.tile.bounds` | [x] | [ ] | [ ] |
| Developers | Geohash | `indexing.geohash.encode` | [x] | [ ] | [ ] |
| Developers | Geodesic distance | `navigation.geodesic.inverse` | [x] | [ ] | [ ] |
| Developers | Haversine (vs geodesic) | `navigation.geodesic.haversine` | [x] | [ ] | [ ] |
| Developers | Magnetic declination | `geodesy.magnetic.declination` | [x] | [ ] | [ ] |

Stable: 36 of 42 rows. `tools/trust/hero.test.mjs` checks every id and Stable box against the catalog.

## Waiting on a published worked example

The stable bar needs an independent, published worked example. These rows are held back until one turns up, rather than citing an example we computed ourselves:

| Tool | Searched (2026-09-19) |
|---|---|
| Holding entry | AIM 5-3-8 and FAA-H-8083-15B define the sectors only by figure; the FAA instrument sample tests have no entry question |
| Part 107 altitude | 14 CFR 107.51, AC 107-2A, FAA-G-8082-22, the UAG sample test, and the 2016 final rule preamble state the rule without a numeric case |
| Flight time | No FAA or manufacturer source works the arithmetic |
| Overlap and trigger, image count | Blog and vendor examples found in search did not contain the numbers when checked; the peer-reviewed footprint paper (AKJournals, 2024) is not freely retrievable |
| Mapping sun window | No published worked example of a sun-elevation window; the threshold crossing shares the SPA solver that sunrise and twilight are verified with |
