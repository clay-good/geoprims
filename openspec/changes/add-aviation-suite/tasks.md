## 1. Atmosphere

- [ ] 1.1 Implement the ICAO layer-table evaluator with geopotential/geometric conversion; verify the 10,000 ft, tropopause, geometric, and above-model scenarios and every row of the ICAO Doc 7488 table within printed precision (built: layer evaluator, geometric/geopotential, all four scenarios; pending: every printed Doc 7488 row)
- [ ] 1.2 Implement US76 to 86 km; verify the table-agreement scenario (built: US 1976 to 86 km with the M/M0 kinetic-temperature table; checked against 4 printed rows so far)
- [x] 1.3 Implement non-standard days with temperature-difference inputs; verify the ISA+20 scenario
- [ ] 1.4 Implement humidity functions (cited Magnus coefficients), virtual temperature, and moist density; verify the humid-density scenario against a published psychrometric example
- [x] 1.5 Implement cloud-base and freezing-level estimates; verify the cloud-base scenario (`aviation.atmosphere.cloud-base`: 400 ft per °C of spread, labeled a rule of thumb, beside the lifting condensation level by Bolton (1980) eq. 15 and the dry-adiabatic rate, above the field and above sea level with the elevation; the freezing level from the surface temperature at a lapse rate, standard 1.98 °C per 1,000 ft by default. The scenario: 25 °C and 15 °C give 4,000 ft by the rule and 4,125 ft by the LCL; 7 golden vectors)
- [ ] 1.6 Implement the atmosphere profile chart; verify the visual fixture

## 2. Airspeed

- [x] 2.1 Implement impact-pressure-based CAS ↔ Mach ↔ TAS ↔ EAS (subsonic and Rayleigh); verify the FL100, high-altitude, and supersonic scenarios and a round-trip property test
- [x] 2.2 Implement calibration-table IAS ↔ CAS with no default correction; verify the no-table and beyond-table scenarios
- [x] 2.3 Implement OAT-required TAS with the ISA-assumed warning; verify the standard-temperature scenario
- [x] 2.4 Implement TAT ↔ SAT with recovery factor and dynamic pressure; verify the ram-rise and compressibility-sign scenarios
- [ ] 2.5 Implement the airspeed tape gauge with labeled V-speed arcs; verify the visual fixture and the not-color-alone check

## 3. Altimetry

- [x] 3.1 Implement pressure altitude, station pressure, and altimeter setting (ISA-derived constants, NWS option); verify the 5,000 ft and standard-setting scenarios (pressure altitude with ISA constants and both scenarios; `aviation.altimetry.q-codes` gives station pressure (QFE) from QNH and the altimeter setting from station pressure by ISA and by the NWS formula, read from its published PDF)
- [x] 3.2 Implement METAR-group parsing and plausibility flags; verify the `Q1009` scenario
- [x] 3.3 Implement density altitude (dry and humid) with approximations shown; verify the hot-high and humidity scenarios
- [x] 3.4 Implement ISA temperature and deviation; verify the FL410 scenario
- [x] 3.5 Implement Q-code conversions, flight levels, and lowest usable flight level from dated reference data; verify the QFE and FL185 scenarios (`aviation.altimetry.q-codes`: QNH ↔ QFE and QNE; `aviation.altimetry.flight-level`: flight level ↔ altitude on a QNH, the 18,000 ft US transition altitude, and the lowest usable flight level from the 14 CFR 91.121(b) table, a dated `data/regulations.json` entry checked against eCFR on 2026-09-22. Both scenarios pass; 17 golden vectors from an independent ISA, and a unit test of every table band edge. The Part 91 fuel-rule links, which pointed at subpart C, now point at their real place in subpart B)
- [x] 3.6 Implement the 2020 ICAO cold-temperature equation, table method, 4% rule, and multi-segment correction; verify the -30 °C and warmer-than-ISA scenarios against Transport Canada AC 500-020 worked examples (`aviation.altimetry.cold-temperature`: the equation, the 4% rule, AIM Table 7-3-1 read bilinearly (transcribed from the FAA's image, with printed and interpolated cells spot-checked in unit tests), and up to 20 procedure altitudes at once. Both scenarios pass (+218 ft, +246 ft by the rule; 0 when warmer than ISA); 7 golden vectors. AC 500-020 Issue 04 (2025-10-28) section 4.8 was checked on 2026-09-22: it has the same equation but no worked examples, and writes T0 as 273 + 15, a 0.1 ft difference here, noted on the tool)
- [ ] 3.7 Implement true altitude and the altimetry diagram and gauge; verify the colder-air scenario and visual fixture

## 4. Wind and navigation

- [x] 4.1 Implement all wind-triangle forms with reference checks; verify the heading, find-wind, too-strong, and mixed-reference scenarios (all four forms: heading and groundspeed, find the wind, `aviation.wind.tas-from-groundspeed` (air = ground − wind), and `aviation.wind.course-from-heading` (ground = air + wind), each with true/magnetic mixing refused without a variation and variable winds refused. The two new forms reproduce the heading scenario the other way round (about 120 kt on 081.7°; course 090° at 108.7 kt); 16 golden vectors)
- [x] 4.2 Implement runway components, gusts, limits, designator parsing, and best-runway ranking; verify the runway 27, gust, designator, and ranking scenarios (components, gusts, variable winds, limits, designators, and their scenarios; `aviation.wind.best-runway` ranks every runway end from a list like `09/27, 18/36` by headwind, then crosswind, with runways beyond your limits flagged and ranked last. The ranking scenario passes (wind 200° at 12 kt: 18, 27, 09, 36); 9 golden vectors, including a VRB wind and duplicate or invalid designators)
- [x] 4.3 Implement the heading chain with deviation-card interpolation; verify the interpolation scenario (`aviation.wind.heading-chain`: true course, wind correction, true heading, variation (east positive, from the chart or a model), magnetic heading, deviation read linearly from the card around the circle, compass heading, each step listed. The scenario: +2° at 060° and -1° at 090° read +0.5° at 075° magnetic, compass 074.5°; 8 golden vectors, including a card wrapping through north)
- [x] 4.4 Implement 1-in-60 corrections (exact and rule); verify the 4 NM scenario (`aviation.wind.one-in-sixty`: track error atan(off ÷ flown), closing angle atan(off ÷ remaining), the turn to parallel and the turn to reach the destination, each beside 60 × off ÷ distance. The scenario: 4 NM off after 60 NM with 60 NM to go gives 3.81° and 7.63° exact, 4° and 8° by the rule; 8 golden vectors. Search now reads "1 in 60" as a ratio, not 1 inch)
- [x] 4.5 Implement u/v wind conversion and winds-aloft interpolation between levels; verify against a hand-computed fixture (`aviation.wind.uv` both ways in the meteorological convention, u = −s·sin θ and v = −s·cos θ for a wind from θ true; `aviation.wind.aloft-interpolate` linear in u, v, and temperature between the two levels that bracket the altitude, no extrapolation, levels in any order. The hand-computed fixture: 7,500 ft between 270° at 20 kt and 300° at 30 kt gives 288° at 24.2 kt; 21 golden vectors, including a wind veering through north and opposite winds cancelling to calm)
- [ ] 4.6 Implement wind-triangle and runway diagrams; verify visual fixtures

## 5. Performance

- [x] 5.1 Implement turn relations, stall in turn, and load-limit checks; verify the standard-rate, 60°-bank, and load-limit scenarios
- [x] 5.2 Implement climb/descent gradients, top of descent, and VDP; verify the FL350 and climb-gradient scenarios
- [ ] 5.3 Implement glide range with wind and the glide ring on the map; verify the headwind scenario (built: glide range with wind and the headwind scenario; pending: the glide ring on the map)
- [x] 5.4 Implement pivotal altitude and specific range, and the no-POH refusal; verify the pivotal-altitude and no-POH scenarios (pivotal altitude and its scenario; `aviation.performance.specific-range`: NM per US gallon through the air and over the ground and fuel per 100 NM, with gal/h or L/h flows, 8 golden vectors; the no-POH refusal lives in the table tool, which takeoff- and landing-distance searches now reach: with no table it explains that the aircraft's own POH/AFM data is required, offers the table entry, and points to density altitude, with a regression vector)

## 6. Fuel and loading

- [x] 6.1 Implement fuel planning and weights with nominal presets; verify the fuel-weight scenario (fuel weight and volume with nominal 100LL and Jet A densities, and the scenario; `aviation.loading.fuel-plan`: trip fuel by leg at each leg's burn, taxi and climb allowances, fuel to an alternate, reserve, fuel to spare, and endurance from usable fuel, with a new volume-flow quantity (gal/h, L/h, imperial gal/h); 12 golden vectors)
- [x] 6.2 Implement dated reserve presets from reference data; verify the VFR-night scenario and the citation display (five entries in `data/regulations.json`, checked against eCFR on 2026-09-22: airplane VFR day 30 and night 45 min, rotorcraft VFR 20 min (14 CFR 91.151), IFR 45 min and helicopter IFR 30 min (91.167(a)(3), 65 FR 3546); the reserve basis names the citation, the review date, that operators may require more, and the eCFR link. Both scenarios pass, plus a custom reserve; the regulation loader moved to `gp-base` so drone and aviation share it)
- [ ] 6.3 Implement weight and balance (CG, % MAC, envelope check, burn path, weight shift, ballast); verify the CG, out-of-envelope, and landing-shift scenarios (built: CG, % MAC, envelope check with the CG shift and weight change to the edge, and the landing state, with all three scenarios; pending: the envelope diagram, weight shift, and ballast)
- [x] 6.4 Implement 1D/2D/3D table interpolation with correction steps and no extrapolation; verify the bilinear and extrapolation scenarios (`aviation.loading.table-interpolate`: a full grid of one, two, or three variables read by linear, bilinear, or trilinear interpolation over the 2, 4, or 8 surrounding cells, listed with their weights; OUT_OF_DOMAIN naming the variable and the table's range, never extrapolating; percent, add, and multiply corrections applied as labeled steps. Both scenarios pass (3,000 ft and 25 °C from the four surrounding cells; 5,000 ft refused on the pressure-altitude axis, 0 to 4000); 11 golden vectors)
- [ ] 6.5 Implement local aircraft profiles with JSON export/import; verify the export round trip

## 7. Catalog, docs, and safety

- [ ] 7.1 Register all 83 operations and 20 generated endpoints, plus alias slugs and pilot vocabulary aliases (E6B, WCA, DA, PA, TOD, W&B, "crosswind calculator"); verify catalog counts and alias fixtures
- [ ] 7.2 Write docs per tool, the "Preflight performance check" guide, and a "Two pressure-altitude constant sets" explainer; verify the docs build
- [ ] 7.3 Verify the safety notice and dated regulatory references render on every aviation page
- [ ] 7.4 Promote tools meeting the stable bar; verify the verification report
