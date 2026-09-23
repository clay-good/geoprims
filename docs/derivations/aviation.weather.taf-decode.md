<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# TAF decoder (`aviation.weather.taf-decode`)

## Method

Read the header (amendment or correction, station, issue time, valid period), then split the forecast into change groups at FM, TEMPO, BECMG, and PROBnn (alone or with TEMPO). Each prevailing period (the base forecast and each FM) runs until the next FM or the end of validity, so together they tile the valid period. Each group's conditions decode as in the METAR decoder (wind, visibility, weather, clouds and ceiling, flight category), plus wind shear (WSxxx/dddffKT), NSW, and the ICAO TX/TN maximum and minimum temperature groups. Times are shown as day and hour UTC, with local time when an offset is given, across month ends. A change group outside the valid period is flagged, not silently used. Anything unread is listed per period.

## Equations

- DDhh/DDhh validity and change periods; FMDDhhmm starts a new prevailing period.
- Hours from the start of validity = (day difference, allowing a month end) × 24 + hour difference.
- Conditions and flight categories as in the METAR decoder (FAA-H-8083-28B).

## Symbols and units

Times in UTC (day of month and hour), winds in degrees true and kt, visibility in SM or meters as forecast, heights in ft AGL, temperatures in °C.

## Domain

Any TAF text, amended or corrected, with a valid period of 1 to 48 hours.

## Approximations

None: it decodes the text as written. TEMPO and PROB periods describe temporary or possible conditions within the prevailing ones, and the decoder keeps them separate.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aviation Weather Handbook (FAA-H-8083-28B)
- sourceEdition: 2026
- sourceLocator: Section 27.4.3 (section 27.3.3 in FAA-H-8083-28A), TAF examples: KPIR 111140Z 1112/1212 … decoded line by line
- independent: yes
- inputs: the KPIR TAF
- outputs: 7 periods; base 130° at 12 kt, visibility over 6 SM, broken at 10,000 ft, wind shear at 2,000 ft from 350° at 35 kt; TEMPO to 1400Z; FM111500 160° at 15 kt gusting 25 kt; PROB30 broken cumulonimbus at 3,000 ft; last TEMPO overcast cumulonimbus at 3,000 ft
- tolerance: exact
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-aviation/tests/taf_parity.rs`: 588 live TAFs from the US, Canada, and Europe (Aviation Weather Center data API, 2026-09-19) against pytaf 1.2.1, comparing the sequence of change groups, their start and end times, wind, visibility, and every cloud layer. All agree.
- `tools/vectors/gen_taf_diff.py`: regenerates that fixture
- `core/vectors/aviation.weather.taf-decode.jsonl`: 21 vectors, including hand decodes, the handbook example, and every 40th live TAF

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `taf_invariants`: on all 588 live TAFs, the base and FM periods tile the valid period with no gap or overlap, every period starts inside it (or the tool flags it), and the period count is one plus the change groups in the text
