## 1. Photogrammetry

- [x] 1.1 Implement GSD (across and along track), inverse GSD, crop-factor conversion, and the equivalent-focal-length warning; verify the 1-inch, suspected-equivalent, and 2 cm scenarios
- [x] 1.2 Implement footprint, trigger distance and interval, line spacing, and camera-interval checks; verify the 75/65 and camera-too-slow scenarios
- [x] 1.3 Implement cited overlap presets; verify the forest scenario
- [ ] 1.4 Implement terrain-aware overlap using a highest-terrain value or DEM profile; verify the hill scenario (done so far: `drone.photogrammetry.terrain-overlap` from a highest-terrain value: the height left over the highest ground, the worst front and side overlap with photo and line spacing fixed by the flat plan, the GSD there and at takeoff level when a camera is given, OVERLAP_BELOW_TARGET against the planned overlap or a chosen minimum, and the flight height that holds that minimum with spacing replanned. The hill scenario (100 m over a 40 m hill at 75%) gives 60 m and 58.3%; 14 golden vectors, the minimum height by bisection. Pending: a DEM profile)
- [x] 1.5 Implement motion blur and maximum shutter; verify the 1/1000 s scenario
- [ ] 1.6 Implement oblique GSD and trapezoid footprint; verify the 45° scenario against a hand-computed fixture (done so far: `drone.photogrammetry.oblique-gsd`: a pinhole camera tilted off nadir over flat ground, by ray casting; GSD across and along at the center, along at the near and far edges, the nadir GSD for comparison, the near and far distances, and the four footprint corners; a BEYOND_HORIZON warning drops the far values when the top of the image reaches the horizon. The 45° at 100 m scenario gives 33.33 m and 300 m to the near and far edges (tan 18.43° and tan 71.57°), with a center GSD √2 times the nadir GSD; 14 golden vectors from the closed angle form. Pending: drawing the trapezoid on the map)
- [x] 1.7 Implement image-count estimation; verify the 2% agreement scenario against generated grids for 20 polygons
- [ ] 1.8 Implement the ASPRS Edition 2 calculator (RMSE_H, NVA/VVA, checkpoint error, 30-point minimum, blunders, mean error); verify all three ASPRS scenarios and the worked examples in the standard (built: product accuracy with checkpoint error, RMSE_H, and the 30-checkpoint minimum with two of the three scenarios; pending: per-checkpoint lists for blunder and mean-error checks, NVA/VVA)

## 2. Mission patterns

- [ ] 2.1 Implement the survey grid (auto direction, overshoot, holes, crosshatch) with geodesic spacing verification; verify the thin-rectangle and hole scenarios (built: serpentine sweep on a local TM plane, auto direction by minimum hull width, overshoot, crosshatch, and transits routed around buffered holes; both scenarios pass and line spacing checks geodesically to 1 mm)
- [x] 2.2 Implement corridor patterns; verify the pipeline scenario
- [x] 2.3 Implement orbits with heading and gimbal pitch; verify the tower scenario
- [x] 2.4 Implement facade scans; verify the facade GSD scenario (`drone.mission.facade`: level passes parallel to a facade line at a fixed standoff, on the chosen side, stacked from a bottom to a top height with horizontal and vertical overlap (default 75/60), stations evenly spread and serpentine, each waypoint with latitude, longitude, height above the facade's base, and the heading facing the wall; the facade GSD and photo size use the standoff as the object distance. The 30 m scenario and 12 more golden vectors, 10 of them on the equator where the geodesic positions are closed form)
- [x] 2.5 Implement geofence generation and waypoint checks; verify the outside-fence scenario (`drone.mission.geofence`: the round geodesic buffer of a point, route, or area from `geometry.buffer.geodesic`, an optional inner warning fence, the fence's area and perimeter and measured error, and each waypoint past a fence with its number and exact distance (the geodesic distance to the area minus the fence distance), raising WAYPOINT_OUTSIDE_GEOFENCE. The scenario runs a survey grid plus a waypoint 62 m out past a 50 m fence and flags it as the last waypoint, 12.00 m outside; 9 golden vectors on the equator and a meridian where the distances are closed form)
- [ ] 2.6 Implement typed waypoint heights and terrain following; verify the terrain-following scenario
- [x] 2.7 Implement KML, GeoJSON, and CSV export; verify the KML altitude-mode scenario and schema validation of each output (`drone.mission.export`: waypoints with height, heading, gimbal pitch, and action written as KML (AGL as relativeToGround; MSL, takeoff with the takeoff elevation added, and HAE with the geoid height removed as absolute, per OGC KML 2.3 §9.20), RFC 7946 GeoJSON with per-waypoint properties and a foreign member naming the tool, version, and notice, or RFC 4180 CSV; each file names the tool and core version and says it is not for navigation. Tests check the KML is well formed with relativeToGround on every placemark, the GeoJSON parses with longitude-latitude-height order, and the CSV columns and quoting; 14 golden vectors. The web page offers the file as a download)
- [ ] 2.8 Implement pattern visualization with coverage overlay; verify visual fixtures

## 3. Endurance and power

- [x] 3.1 Implement battery energy, usable energy, and C-rate; verify the 90.4 Wh and C-rate scenarios
- [ ] 3.2 Implement momentum-theory hover power with FM, efficiency, and density; verify the 1.4 kg and density-altitude scenarios (built: momentum theory with FM, efficiency, avionics power, and ISA troposphere density from altitude and temperature or density altitude; both scenarios pass; pending: the coaxial-overlap option)
- [ ] 3.3 Implement endurance, range, and heuristic derating; verify the reserve and cold-battery scenarios (built: endurance with reserve, cruise power, range, and the labeled cold derating; both scenarios pass; pending: wind-aware range through the aviation wind tools)
- [ ] 3.4 Implement payload impact and maximum payload; verify the maximum-payload scenario (built: maximum payload for a target time with NO_SOLUTION; pending: the payload-impact comparison)
- [x] 3.5 Implement the return-to-home energy budget; verify the headwind and cannot-return scenarios
- [ ] 3.6 Implement optional Peukert (off by default) and test-flight calibration (back-solve FM·η); verify the default-off scenario and a calibration fixture

## 4. Operations reference

- [ ] 4.1 Create the dated regulatory reference-data file with Part 107, Part 89, the Part 108 NPRM, and EASA entries, plus the stale-review CI warning; verify the Part 108 and stale-entry scenarios (built: data/regulations.json with Part 107, Part 89, the Part 108 NPRM (90 FR 38212, labeled proposed), and EASA entries, and tools/data/regulations.test.mjs listing entries reviewed more than 12 months ago)
- [ ] 4.2 Implement the altitude limit calculator with structure exception and height conversions; verify the tower and MSL scenarios (built: structure exception and MSL, and HAE from a user-entered geoid height; both scenarios pass; pending: DEM ground elevation and the geoid tool hand-off)
- [x] 4.3 Implement speed and kinetic-energy checks; verify the C1 energy scenario
- [x] 4.4 Implement the EASA subcategory helper; verify the legacy 2 kg scenario
- [x] 4.5 Render the not-legal-advice disclaimer with review date; verify on every operations page

## 5. Catalog and docs

- [ ] 5.1 Register all 42 operations with alias slugs and aliases (GSD calculator, overlap calculator, drone flight time); verify catalog counts
- [ ] 5.2 Write docs per tool and the "Plan a photogrammetry mission" guide; verify the guide chain end to end
- [ ] 5.3 Promote tools meeting the stable bar; verify the verification report
