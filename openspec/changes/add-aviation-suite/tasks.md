## 1. Atmosphere

- [ ] 1.1 Implement the ICAO layer-table evaluator with geopotential/geometric conversion; verify the 10,000 ft, tropopause, geometric, and above-model scenarios and every row of the ICAO Doc 7488 table within printed precision
- [ ] 1.2 Implement US76 to 86 km; verify the table-agreement scenario
- [ ] 1.3 Implement non-standard days with temperature-difference inputs; verify the ISA+20 scenario
- [ ] 1.4 Implement humidity functions (cited Magnus coefficients), virtual temperature, and moist density; verify the humid-density scenario against a published psychrometric example
- [ ] 1.5 Implement cloud-base and freezing-level estimates; verify the cloud-base scenario
- [ ] 1.6 Implement the atmosphere profile chart; verify the visual fixture

## 2. Airspeed

- [ ] 2.1 Implement impact-pressure-based CAS ↔ Mach ↔ TAS ↔ EAS (subsonic and Rayleigh); verify the FL100, high-altitude, and supersonic scenarios and a round-trip property test
- [ ] 2.2 Implement calibration-table IAS ↔ CAS with no default correction; verify the no-table and beyond-table scenarios
- [ ] 2.3 Implement OAT-required TAS with the ISA-assumed warning; verify the standard-temperature scenario
- [ ] 2.4 Implement TAT ↔ SAT with recovery factor and dynamic pressure; verify the ram-rise and compressibility-sign scenarios
- [ ] 2.5 Implement the airspeed tape gauge with labeled V-speed arcs; verify the visual fixture and the not-color-alone check

## 3. Altimetry

- [ ] 3.1 Implement pressure altitude, station pressure, and altimeter setting (ISA-derived constants, NWS option); verify the 5,000 ft and standard-setting scenarios
- [ ] 3.2 Implement METAR-group parsing and plausibility flags; verify the `Q1009` scenario
- [ ] 3.3 Implement density altitude (dry and humid) with approximations shown; verify the hot-high and humidity scenarios
- [ ] 3.4 Implement ISA temperature and deviation; verify the FL410 scenario
- [ ] 3.5 Implement Q-code conversions, flight levels, and lowest usable flight level from dated reference data; verify the QFE and FL185 scenarios
- [ ] 3.6 Implement the 2020 ICAO cold-temperature equation, table method, 4% rule, and multi-segment correction; verify the -30 °C and warmer-than-ISA scenarios against Transport Canada AC 500-020 worked examples
- [ ] 3.7 Implement true altitude and the altimetry diagram and gauge; verify the colder-air scenario and visual fixture

## 4. Wind and navigation

- [ ] 4.1 Implement all wind-triangle forms with reference checks; verify the heading, find-wind, too-strong, and mixed-reference scenarios
- [ ] 4.2 Implement runway components, gusts, limits, designator parsing, and best-runway ranking; verify the runway 27, gust, designator, and ranking scenarios
- [ ] 4.3 Implement the heading chain with deviation-card interpolation; verify the interpolation scenario
- [ ] 4.4 Implement 1-in-60 corrections (exact and rule); verify the 4 NM scenario
- [ ] 4.5 Implement u/v wind conversion and winds-aloft interpolation between levels; verify against a hand-computed fixture
- [ ] 4.6 Implement wind-triangle and runway diagrams; verify visual fixtures

## 5. Performance

- [ ] 5.1 Implement turn relations, stall in turn, and load-limit checks; verify the standard-rate, 60°-bank, and load-limit scenarios
- [ ] 5.2 Implement climb/descent gradients, top of descent, and VDP; verify the FL350 and climb-gradient scenarios
- [ ] 5.3 Implement glide range with wind and the glide ring on the map; verify the headwind scenario
- [ ] 5.4 Implement pivotal altitude and specific range, and the no-POH refusal; verify the pivotal-altitude and no-POH scenarios

## 6. Fuel and loading

- [ ] 6.1 Implement fuel planning and weights with nominal presets; verify the fuel-weight scenario
- [ ] 6.2 Implement dated reserve presets from reference data; verify the VFR-night scenario and the citation display
- [ ] 6.3 Implement weight and balance (CG, % MAC, envelope check, burn path, weight shift, ballast); verify the CG, out-of-envelope, and landing-shift scenarios
- [ ] 6.4 Implement 1D/2D/3D table interpolation with correction steps and no extrapolation; verify the bilinear and extrapolation scenarios
- [ ] 6.5 Implement local aircraft profiles with JSON export/import; verify the export round trip

## 7. Catalog, docs, and safety

- [ ] 7.1 Register all 83 operations and 28 generated endpoints with pilot vocabulary aliases (E6B, WCA, DA, PA, TOD, W&B, "crosswind calculator"); verify catalog counts and alias fixtures
- [ ] 7.2 Write docs per tool, the "Preflight performance check" guide, and a "Two pressure-altitude constant sets" explainer; verify the docs build
- [ ] 7.3 Verify the safety notice and dated regulatory references render on every aviation page
- [ ] 7.4 Promote tools meeting the stable bar; verify the verification report
