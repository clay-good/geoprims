<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# METAR decoder (`aviation.weather.metar-decode`)

## Method

Read a METAR or SPECI group by group in the order WMO FM 15/16 and FAA Order JO 7900.5E define: type, station, time, modifier, wind (with gust and variability), visibility (statute miles, meters, CAVOK, NDV), RVR, present weather, clouds (including /// for a type the station cannot determine), temperature and dew point, and altimeter (A or Q). A trend (BECMG or TEMPO) is a forecast, so its groups are listed but never applied to the observation. Remarks decode the US groups (AO1/AO2, SLP, T group temperatures to 0.1 °C, and others). Every group is explained or listed as not decoded. The flight category comes from the ceiling (the lowest broken, overcast, or vertical-visibility layer) and the visibility.

## Equations

- Wind direction from the first three digits (degrees true), speed and gust in kt (MPS × 3,600/1,852).
- SLPppp: 10pp.p hPa when ppp < 500, otherwise 9pp.p hPa.
- T group: sign digit then tenths of °C.
- Flight category (FAA-H-8083-28A tables 3-15 to 3-18): LIFR when ceiling < 500 ft or visibility < 1 SM; IFR when < 1,000 ft or < 3 SM; MVFR when 1,000–3,000 ft or 3–5 SM inclusive; otherwise VFR.
- Present weather: intensity qualifies the precipitation, so +TSRA is a thunderstorm with heavy rain and +SHRA heavy rain showers (table 24-3).

## Symbols and units

Directions in degrees true, speeds in kt, visibility in SM (metric reports converted, with the original text kept), heights in ft AGL, temperatures in °C, altimeter in inHg or hPa as reported.

## Domain

Any METAR or SPECI text, with or without the leading type. Groups it cannot read are listed with their positions, and the rest are still decoded.

## Approximations

None: it decodes the text as written and does not check it against the station or fetch anything. The flight category is for situational awareness and is not the 14 CFR 91.155 VFR minimums.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aviation Weather Handbook (FAA-H-8083-28A)
- sourceEdition: 2024
- sourceLocator: Chapter 24, sections 24.4.3.1 to 24.4.3.14: METAR KOKC 011955Z AUTO 22015G25KT 180V250 3/4SM R17L/2600FT +TSRA BR OVC010CB 18/16 A2992 RMK AO2 TSB25 TS OHD MOV E SLP132, decoded group by group
- independent: yes
- inputs: that report
- outputs: wind 220° at 15 kt gusting 25 kt, varying 180° to 250°; visibility 3/4 SM; thunderstorm with heavy rain and mist; overcast cumulonimbus at 1,000 ft; 18/16 °C; 29.92 inHg; SLP 1013.2 hPa; LIFR
- tolerance: exact
- verifiedBy: golden vector v007, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-aviation/tests/metar_parity.rs`: 612 live reports from the US and Europe (Aviation Weather Center data API, 2026-09-19) against python-metar 1.11.0, comparing wind, gust, visibility, temperature and dew point, altimeter, sea-level pressure, and every cloud layer. It found three decoder defects, now fixed and pinned: /// cloud types dropped, trend groups applied as current weather, and NDV visibility not read.
- `tools/vectors/gen_metar_diff.py`: regenerates that fixture
- `core/vectors/aviation.weather.metar-decode.jsonl`: 21 vectors, including hand decodes per JO 7900.5E, the handbook example, the regressions, and every 60th live report

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `metar_invariants`: on all 612 live reports, every group is explained or listed as not decoded, the flight category follows the thresholds from the decoded ceiling and visibility, and cutting the remarks leaves the body values unchanged
