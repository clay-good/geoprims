## 1. Time domain and assets

- [ ] 1.1 Add the `time` domain to the catalog taxonomy; verify catalog validation accepts `time.sun.*` and `time.scale.*`
- [ ] 1.2 Package `leap-seconds` (IERS Bulletin C) and `tzdb` (IANA) as ledger-tracked assets; verify digests and freshness rows
- [ ] 1.3 Package optional `tz-boundaries` (ODbL, with attribution) and `ngs-antinfo`; verify sizes are shown before download

## 2. Solar and twilight

- [ ] 2.1 Implement NREL SPA with ΔT table and DUT1 input, plus the NOAA cross-check; verify the NREL test point and the UT1 warning scenario
- [ ] 2.2 Implement rise/set/twilight with polar states and dated local/UTC output; verify the polar-night, after-00:00Z, and NOAA-agreement scenarios
- [ ] 2.3 Implement the four aviation nights from dated reference data; verify both night scenarios and obtain CFI review
- [ ] 2.4 Implement the night-currency counter; verify the lapse-date scenario against the regulation's counting convention
- [ ] 2.5 Implement the mapping window, hotspot, shadow length, and slope incidence; verify the mapping-window scenario
- [ ] 2.6 Implement the sun-path diagram and map overlay; verify visual fixtures

## 3. Time scales

- [ ] 3.1 Implement tzdb-based local↔UTC with DST gap/overlap handling and Zulu formatting; verify the DST and Zulu scenarios
- [ ] 3.2 Implement decimal hours and block time across midnight; verify the 1.3 h scenario
- [ ] 3.3 Implement GPS week/rollover, GNSS offsets, JD/MJD, and day of year; verify the GPS-week, Julian-date, and stale-table scenarios
- [ ] 3.4 Implement the optional zone-from-location lookup; verify the no-boundary-data scenario

## 4. Aviation weather and IFR

- [ ] 4.1 Implement the METAR/SPECI grammar with US remarks and undecoded listing; verify the full-METAR and undecoded scenarios and a corpus of at least 300 hand-verified reports
- [ ] 4.2 Implement TAF decoding with timeline; verify the across-midnight scenario and month-end cases
- [ ] 4.3 Implement the FB winds decoder; verify the high-speed and light-and-variable scenarios
- [ ] 4.4 Implement true-referenced hand-off to wind tools and the old-observation flag; verify the hand-off and old-observation scenarios
- [ ] 4.5 Implement hold entry, wind timing, and speed limits; verify the hold scenarios and obtain CFII review
- [ ] 4.6 Implement DME, arc, time-to-station, and intercept tools; verify the slant-range and overhead scenarios
- [ ] 4.7 Extend the existing descent-gradient and VDP tools (no duplicates) and implement TCH and VASI/PAPI geometry; verify both descent scenarios
- [ ] 4.8 Implement the station-variation warning and NOTAM/TFR geometry; verify the variation and TFR scenarios

## 5. Survey land descriptions and GNSS

- [ ] 5.1 Implement the deed parser with the editable confirmation table, curve and monument flags; verify both parsing scenarios on a corpus of 50 real deed texts
- [ ] 5.2 Implement plot, closure, and implied-closing-line area without silent adjustment; verify the unclosed-deed scenario
- [ ] 5.3 Implement legacy land units with jurisdictions; verify the vara and chains scenarios
- [ ] 5.4 Implement the PLSS parser and nominal aliquot areas; verify the aliquot and meridian scenarios
- [ ] 5.5 Implement basis-of-bearing rotation and the professional-use notice; verify the rotation scenario
- [ ] 5.6 Implement almanac parsing, DOP, sky plot, and terrain mask; verify both DOP scenarios against a reference planning tool
- [ ] 5.7 Implement the RTK budget, OPUS planning, and antenna height; verify the corresponding scenarios
- [ ] 5.8 Implement the ALTA RPP check tied to error ellipses; verify both RPP scenarios and obtain PLS review
- [ ] 5.9 Implement similarity and affine localization with warnings; verify both localization scenarios
- [ ] 5.10 (v1.1) Package BLM CadNSDI per state and implement PLSS lookup and reverse lookup; verify against BLM sample points

## 6. Drone sensors and links

- [ ] 6.1 Implement VLOS (verify EASA coefficients against current AMC/GM text) and mission comparison; verify both VLOS scenarios
- [ ] 6.2 Present the Part 107 twilight window as a drone view of `time.sun.aviation-nights`; verify the evening scenario
- [ ] 6.3 Implement lidar planning with USGS QL comparison; verify the density scenario
- [ ] 6.4 Implement dataset size, link budget, and thermal footprint; verify the orthomosaic, FSPL, and thermal scenarios

## 7. Cuts and catalog

- [ ] 7.1 Remove `koch-estimate` and `angle-of-repose-reference` from inventories and specs; verify the catalog no longer lists them
- [ ] 7.2 Convert the sight-distance design K to a cited input; verify per `trust/citations`
- [ ] 7.3 Add the terrain-following acknowledgment and margin to drone mission export; verify that export is blocked without acknowledgment
- [ ] 7.4 Register all new operations with practitioner aliases (zulu, metar decoder, night currency, hold entry, deed plotter, township range section); verify alias fixtures and catalog counts
