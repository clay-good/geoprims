<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Read any coordinate (`geodesy.parse.coordinates`)

## Method

Normalize look-alike marks (′ ″ ’ ” º − to their ASCII forms), then try the notations from most to least specific: MGRS, UTM, and UPS references (UPS written as a zone letter, easting, and northing: A or B south and Y or Z north as in MGRS, or N or S when both values are too large to be degrees), then (for a single word that is not a coordinate) the grid codes, labeled pairs (`lat: … lon: …` in either order), packed aviation (ddmmssN dddmmssW, only when neither coordinate contains a space), and a pair split by hemisphere letters, by the second degree sign, or by whitespace. Each half is read as components (degrees, minutes, seconds, each with its mark or none), and the components are checked strictly: minutes and seconds under 60, only the last component with decimals, and hemisphere letters that fit the axis and do not contradict a sign. Without hemisphere letters, a pair is read as latitude, longitude, and when both values fit within ±90° the other reading is reported. When the first value cannot be a latitude, the order is inferred as longitude, latitude, and the tool says so. Decimal commas (40,45 -79,98) are recognized and noted. No reading is chosen silently.

A grid code (a full Plus Code, Maidenhead, GARS, GEOREF, or a geohash that contains a letter) names a cell, not a point, and is read as the cell's center by the same decoders the grid tools use, with a note saying so. A code that is valid in two grids, like fn20 (a Maidenhead square and a geohash), is read one way and the other reading is listed: lower case reads as a geohash, since geohashes are written in lower case, and upper case as Maidenhead. A short Plus Code needs a nearby place to complete it and is refused.

## Equations

- Decimal degrees = D + M/60 + S/3600, negative for S and W or a minus sign.
- Packed: ddmm[ss][.s] for latitude and dddmm[ss][.s] for longitude.

## Symbols and units

D degrees, M minutes, S seconds. Output in decimal degrees with the notation recognized (DD, DDM, DMS, packed, MGRS, UTM, UPS, Plus Code, Maidenhead, GARS, GEOREF, or geohash) and a DMS rendering.

## Domain

Latitude within ±90° and longitude within ±180° (decimal longitudes outside are normalized with a note). Malformed components, contradictory hemispheres, and minutes or seconds of 60 or more are refused with INVALID_INPUT, naming the problem.

## Approximations

None: every notation converts by exact arithmetic. The reading of an ambiguous pair is a stated default with its alternative shown.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Decimal degrees (worked example)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: The US Capitol, 38° 53′ 23″ N, 77° 00′ 32″ W, is 38.8897°, −77.0089° (D + M/60 + S/3600)
- independent: yes
- inputs: 38° 53′ 23″ N, 77° 00′ 32″ W
- outputs: 38.8897°, −77.0089° (the core gives 38.8897222°, −77.0088889°)
- tolerance: 5e-5° (the printed rounding)
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/parse_parity.rs`: 1,500 strings written independently by `tools/vectors/gen_parse_diff.py` in ten notations (signed decimal, hemisphere letters, DMS with symbols, Unicode primes, spaces, or hyphens, degrees and decimal minutes, packed aviation, labeled pairs, and signed DMS), each against the exact value it encodes, within 1e-9°. It found three defects, now fixed and pinned: a panic on signed DMS with degree signs (a byte slice inside the two-byte °), spaced DMS with a one- or two-digit longitude silently read as packed notation (1°25′ became 125°), and an integer underflow on one-digit decimal degrees with a hemisphere letter.
- `tools/vectors/gen_parse_grids.py`: grid-code cell centers from independent implementations (Google's openlocationcode, pygeohash, the maidenhead package) from the published GARS and GEOREF letter schemes, and UPS points from PROJ's inverse (EPSG:32661 and 32761), with the two-grid case and a refused short Plus Code, with a UPS reference outside the polar area refused, pinned as vectors v026 to v042; `core/crates/gp-geodesy/tests/parse_grids.rs` checks the routing against the grid tools' own decoders that no coordinate is taken for a code, and that UPS text reads back what `geodesy.ups.forward` writes
- `core/vectors/geodesy.parse.coordinates.jsonl`: 42 vectors, including the scenarios, the Wikipedia example, the regressions, and every 150th fixture string

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `parse_coordinates_invariants`: whatever the formatter writes (DD, DDM, DMS, with letters or signs) reads back to the point within the printed rounding, and Unicode primes read the same as ASCII marks
