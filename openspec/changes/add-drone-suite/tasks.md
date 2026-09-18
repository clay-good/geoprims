## 1. Photogrammetry

- [x] 1.1 Implement GSD (across and along track), inverse GSD, crop-factor conversion, and the equivalent-focal-length warning; verify the 1-inch, suspected-equivalent, and 2 cm scenarios
- [x] 1.2 Implement footprint, trigger distance and interval, line spacing, and camera-interval checks; verify the 75/65 and camera-too-slow scenarios
- [x] 1.3 Implement cited overlap presets; verify the forest scenario
- [ ] 1.4 Implement terrain-aware overlap using a highest-terrain value or DEM profile; verify the hill scenario
- [x] 1.5 Implement motion blur and maximum shutter; verify the 1/1000 s scenario
- [ ] 1.6 Implement oblique GSD and trapezoid footprint; verify the 45° scenario against a hand-computed fixture
- [x] 1.7 Implement image-count estimation; verify the 2% agreement scenario against generated grids for 20 polygons
- [ ] 1.8 Implement the ASPRS Edition 2 calculator (RMSE_H, NVA/VVA, checkpoint error, 30-point minimum, blunders, mean error); verify all three ASPRS scenarios and the worked examples in the standard (built: product accuracy with checkpoint error, RMSE_H, and the 30-checkpoint minimum with two of the three scenarios; pending: per-checkpoint lists for blunder and mean-error checks, NVA/VVA)

## 2. Mission patterns

- [ ] 2.1 Implement the survey grid (auto direction, overshoot, holes, crosshatch) with geodesic spacing verification; verify the thin-rectangle and hole scenarios (built: serpentine sweep on a local TM plane, auto direction by minimum hull width, overshoot, crosshatch, and transits routed around buffered holes; both scenarios pass and line spacing checks geodesically to 1 mm)
- [x] 2.2 Implement corridor patterns; verify the pipeline scenario
- [x] 2.3 Implement orbits with heading and gimbal pitch; verify the tower scenario
- [ ] 2.4 Implement facade scans; verify the facade GSD scenario
- [ ] 2.5 Implement geofence generation and waypoint checks; verify the outside-fence scenario
- [ ] 2.6 Implement typed waypoint heights and terrain following; verify the terrain-following scenario
- [ ] 2.7 Implement KML, GeoJSON, and CSV export; verify the KML altitude-mode scenario and schema validation of each output
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
