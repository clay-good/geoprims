## 1. Time domain and assets

- [x] 1.1 Add the `time` domain to the catalog taxonomy; verify catalog validation accepts `time.sun.*` and `time.scale.*`
- [ ] 1.2 Package `leap-seconds` (IERS Bulletin C) and `tzdb` (IANA) as ledger-tracked assets; verify digests and freshness rows (done so far: the leap-second table from IERS Bulletin C 72, embedded in the time module, echoed in `meta.assets`, with an expiry warning after 2027-06-30; also tzdb 2026d, embedded in the time module and echoed in meta.assets, with the table cross-checked against IANA leap-seconds.list; pending: the asset ledger, with on-demand loading)
- [ ] 1.3 Package optional `tz-boundaries` (ODbL, with attribution) and `ngs-antinfo`; verify sizes are shown before download

## 2. Solar and twilight

- [x] 2.1 Implement NREL SPA with ΔT table and DUT1 input, plus the NOAA cross-check; verify the NREL test point and the UT1 warning scenario (NREL Table A5.1 to 1e-5°, and 25 random points against pvlib's SPA to 1e-8°; ΔT from the Espenak and Meeus polynomials with an override)
- [ ] 2.2 Implement rise/set/twilight with polar states and dated local/UTC output; verify the polar-night, after-00:00Z, and NOAA-agreement scenarios (done so far: rise, set, noon, day length, and civil, nautical, and astronomical twilight with polar states, dated local and Zulu; verified against USNO at six sites (within 1 minute) and the polar-night and after-00:00Z scenarios; pending: the 1,000-point NOAA sweep)
- [ ] 2.3 Implement the four aviation nights from dated reference data; verify both night scenarios and obtain CFI review (done so far: all four windows with their regulations and a landing check (loggable versus currency); pending: rule text as dated reference data and CFI review)
- [ ] 2.4 Implement the night-currency counter; verify the lapse-date scenario against the regulation's counting convention
- [ ] 2.5 Implement the mapping window, hotspot, shadow length, and slope incidence; verify the mapping-window scenario (done so far: mapping window, shadow length, and slope incidence; pending: hotspot)
- [ ] 2.6 Implement the sun-path diagram and map overlay; verify visual fixtures

## 3. Time scales

- [x] 3.1 Implement tzdb-based local↔UTC with DST gap/overlap and Zulu formatting; verify the DST and Zulu scenarios (IANA tzdb 2026d embedded as slim TZif with POSIX rules; 238,800 instants across 597 zones match Python zoneinfo on the same release; gaps are INVALID_INPUT and overlaps warn AMBIGUOUS_INPUT with both candidates; a new time.scale.zone-info tool; the sun tools accept zone names and use each event's own offset)
- [x] 3.2 Implement decimal hours and block time across midnight; verify the 1.3 h scenario
- [x] 3.3 Implement GPS week/rollover, GNSS offsets, JD/MJD, and day of year; verify the GPS-week, Julian-date, and stale-table scenarios
- [ ] 3.4 Implement the optional zone-from-location lookup; verify the no-boundary-data scenario

## 4. Aviation weather and IFR

- [ ] 4.1 Implement the METAR/SPECI grammar with US remarks and undecoded listing; verify the full-METAR and undecoded scenarios and a corpus of at least 300 hand-verified reports (built: header, wind with gusts and variable range, SM fractions, M/P, metric visibility, CAVOK, RVR, weather, clouds and ceiling, temperatures, A and Q groups, AO1/AO2, SLP, T group, PK WND, WSHFT/FROPA, PRESRR/PRESFR, $, flight category, and the not-decoded list with positions; both scenarios pass; pending: the 300-report corpus)
- [x] 4.2 Implement TAF decoding with timeline; verify the across-midnight scenario and month-end cases (UTC plus local times from a user-entered offset, labeled same, next, or previous day; FM periods closed at the next FM; TEMPO, BECMG, PROB30/40, wind shear, and not-decoded groups per period)
- [x] 4.3 Implement the FB winds decoder; verify the high-speed and light-and-variable scenarios
- [ ] 4.4 Implement true-referenced hand-off to wind tools and the old-observation flag; verify the hand-off and old-observation scenarios (built: winds labeled true and the OBSERVATION_OLD check against a user-supplied time, with month wrap; pending: the web hand-off carrying reference: true)
- [ ] 4.5 Implement hold entry, wind timing, and speed limits; verify the hold scenarios and obtain CFII review (built: AIM 5-3-8 entry sectors with the 5° boundary rule and left-turn mirroring, wind correction with triple-the-drift and outbound timing, and the table 5-3-1 speed limits with ABOVE_MAX_HOLDING_SPEED; pending: the hold diagram and CFII review)
- [ ] 4.6 Implement DME, arc, time-to-station, and intercept tools; verify the slant-range and overhead scenarios (done so far: `aviation.ifr.dme-slant-range` (√(DME² − h²), NO_SOLUTION overhead), `aviation.ifr.time-to-station` (exactly t ÷ sin Δ from where the timing ends, and 60 × t ÷ Δ by the rule, with distances at a groundspeed), and `aviation.ifr.dme-arc-lead` (standard-rate radius GS ÷ 60π, the lead onto the arc, and asin(r ÷ (R − r)) radials off it, with 60r ÷ R by the rule). Both scenarios pass: 5.0 NM at 6,000 ft is 4.902 NM over the ground, and 0.8 NM at 6,000 ft is refused as essentially overhead; 21 golden vectors. Pending: radial intercept angles)
- [ ] 4.7 Extend the existing descent-gradient and VDP tools (no duplicates) and implement TCH and VASI/PAPI geometry; verify both descent scenarios (built: VDP with the HAT/300 and HAT/318 rules and TCH, and the 3° glidepath vertical-speed scenario; pending: VASI/PAPI geometry)
- [ ] 4.8 Implement the station-variation warning and NOTAM/TFR geometry; verify the variation and TFR scenarios

## 5. Survey land descriptions and GNSS

- [ ] 5.1 Implement the deed parser with the editable confirmation table, curve and monument flags; verify both parsing scenarios on a corpus of 50 real deed texts (built: symbol, word, dash, and compact bearings, due north, feet, chains and links, rods, varas, meters, tangent and non-tangent curves, monuments kept as text, CURVE_CALL_INCOMPLETE and NON_METRIC_CALL; both scenarios pass; pending: the 50-deed corpus and source-phrase highlighting in the web table)
- [x] 5.2 Implement plot, closure, and implied-closing-line area without silent adjustment; verify the unclosed-deed scenario
- [x] 5.3 Implement legacy land units with jurisdictions; verify the vara and chains scenarios
- [x] 5.4 Implement the PLSS parser and nominal aliquot areas; verify the aliquot and meridian scenarios (built: aliquot parts, lots, township and range validation, BLM meridian names and codes, and nominal area; both scenarios pass)
- [x] 5.5 Implement basis-of-bearing rotation and the professional-use notice; verify the rotation scenario
- [ ] 5.6 Implement almanac parsing, DOP, sky plot, and terrain mask; verify both DOP scenarios against a reference planning tool
- [ ] 5.7 Implement the RTK budget, OPUS planning, and antenna height; verify the corresponding scenarios
- [ ] 5.8 Implement the ALTA RPP check tied to error ellipses; verify both RPP scenarios and obtain PLS review
- [ ] 5.9 Implement similarity and affine localization with warnings; verify both localization scenarios
- [ ] 5.10 (v1.1) Package BLM CadNSDI per state and implement PLSS lookup and reverse lookup; verify against BLM sample points

## 6. Drone sensors and links

- [x] 6.1 Implement VLOS (verify EASA coefficients against current AMC/GM text) and mission comparison; verify both VLOS scenarios (coefficients checked against the EASA guidelines for UAS operations in the open and specific category on 2026-09-18)
- [ ] 6.2 Present the Part 107 twilight window as a drone view of `time.sun.aviation-nights`; verify the evening scenario
- [ ] 6.3 Implement lidar planning with USGS QL comparison; verify the density scenario
- [ ] 6.4 Implement dataset size, link budget, and thermal footprint; verify the orthomosaic, FSPL, and thermal scenarios

## 7. Cuts and catalog

- [ ] 7.1 Remove `koch-estimate` and `angle-of-repose-reference` from inventories and specs; verify the catalog no longer lists them
- [ ] 7.2 Convert the sight-distance design K to a cited input; verify per `trust/citations`
- [ ] 7.3 Add the terrain-following acknowledgment and margin to drone mission export; verify that export is blocked without acknowledgment
- [ ] 7.4 Register all new operations with practitioner aliases (zulu, metar decoder, night currency, hold entry, deed plotter, township range section); verify alias fixtures and catalog counts
