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
| Pilots | METAR decoder | `aviation.weather.metar-decode` | [ ] | [ ] | [ ] |
| Pilots | TAF decoder | `aviation.weather.taf-decode` | [ ] | [ ] | [ ] |
| Pilots | Winds-aloft decoder | `aviation.weather.fb-winds-decode` | [ ] | [ ] | [ ] |
| Pilots | Holding entry | `aviation.ifr.hold-entry` | [ ] | [ ] | [ ] |
| Pilots | Zulu time | `time.scale.utc-offset` | [x] | [ ] | [ ] |
| Pilots | Weight and balance | `aviation.loading.weight-balance` | [x] | [ ] | [ ] |
| Pilots | Top of descent | `aviation.performance.top-of-descent` | [ ] | [ ] | [ ] |
| Pilots | Visual descent point | `aviation.performance.vdp` | [ ] | [ ] | [ ] |
| Drone | GSD | `drone.photogrammetry.gsd` | [x] | [ ] | [ ] |
| Drone | Altitude for a GSD | `drone.photogrammetry.altitude-for-gsd` | [x] | [ ] | [ ] |
| Drone | Overlap and trigger | `drone.photogrammetry.trigger` | [ ] | [ ] | [ ] |
| Drone | Image count | `drone.photogrammetry.image-count` | [ ] | [ ] | [ ] |
| Drone | Flight time | `drone.power.endurance` | [ ] | [ ] | [ ] |
| Drone | mAh to Wh | `drone.power.battery-energy` | [ ] | [ ] | [ ] |
| Drone | Mapping sun window | `time.sun.mapping-window` | [ ] | [ ] | [ ] |
| Drone | VLOS guidance | `drone.sensors.vlos` | [ ] | [ ] | [ ] |
| Drone | Part 107 altitude | `drone.ops.part107-altitude` | [ ] | [ ] | [ ] |
| Surveyors | Coordinate converter (DMS ↔ decimal) | `geodesy.parse.coordinates` | [ ] | [ ] | [ ] |
| Surveyors | UTM | `geodesy.utm.forward`, `geodesy.utm.inverse` | [x] | [ ] | [ ] |
| Surveyors | MGRS | `geodesy.grid-ref.mgrs-forward`, `geodesy.grid-ref.mgrs-inverse` | [x] | [ ] | [ ] |
| Surveyors | State plane | `geodesy.spcs.spcs83-forward`, `geodesy.spcs.spcs83-inverse` | [x] | [ ] | [ ] |
| Surveyors | Geoid and orthometric height | `geodesy.geoid.geoid-height` | [x] | [ ] | [ ] |
| Surveyors | Grid ↔ ground combined factor | `survey.reduction.combined-factor` | [x] | [ ] | [ ] |
| Surveyors | Traverse closure | `survey.cogo.traverse-closure` | [x] | [ ] | [ ] |
| Surveyors | Deed plotter | `survey.land.deed-plot` | [ ] | [ ] | [ ] |
| Surveyors | Horizontal curve | `survey.curves.circular-curve` | [ ] | [ ] | [ ] |
| Surveyors | Vertical curve | `survey.curves.vertical-curve` | [ ] | [ ] | [ ] |
| Surveyors | Acreage from coordinates | `survey.cogo.area-by-coordinates` | [ ] | [ ] | [ ] |
| Developers | H3 cell | `indexing.h3.lat-lng-to-cell` | [x] | [ ] | [ ] |
| Developers | H3 k-ring | `indexing.h3.grid-disk` | [x] | [ ] | [ ] |
| Developers | Tile and quadkey | `indexing.tile.from-point` | [x] | [ ] | [ ] |
| Developers | Tile bounds | `indexing.tile.bounds` | [x] | [ ] | [ ] |
| Developers | Geohash | `indexing.geohash.encode` | [x] | [ ] | [ ] |
| Developers | Geodesic distance | `navigation.geodesic.inverse` | [x] | [ ] | [ ] |
| Developers | Haversine (vs geodesic) | `navigation.geodesic.haversine` | [x] | [ ] | [ ] |
| Developers | Magnetic declination | `geodesy.magnetic.declination` | [x] | [ ] | [ ] |

Stable: 24 of 42 rows. `tools/trust/hero.test.mjs` checks every id and Stable box against the catalog.
