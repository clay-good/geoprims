## Purpose

Turns any human-written or machine-emitted coordinate or angle string into an unambiguous latitude/longitude or angle, reports every assumption made, and formats results back in any notation with controlled precision.

## ADDED Requirements

### Requirement: Supported input notations
The parser SHALL accept: decimal degrees (`40.446111, -79.982222`); degrees-minutes-seconds with symbols (`40°26'46"N 79°58'56"W`) including Unicode prime/double-prime (`′ ″`), typographic quotes, and ASCII (`' "`); degrees-decimal minutes (`40 26.767N 79 58.933W`); hemisphere letters as prefix or suffix (`N40.446 W79.982`); signed values; space-, colon-, or dash-separated components (`40:26:46N`); packed aviation/nautical forms (`402646N0795856W`, `4026.767N07958.933W`); and labeled forms (`lat=40.4 lon=-79.9`, `lon: -79.9, lat: 40.4`). It SHALL also route MGRS, USNG, UTM, UPS, Maidenhead, GARS, GEOREF, geohash, and Plus Code strings to their decoders.

#### Scenario: DMS with symbols
- **WHEN** the input is `40°26'46"N 79°58'56"W`
- **THEN** the result is latitude 40.446111… and longitude -79.982222… (exact fractions 40 + 26/60 + 46/3600, and so on)

#### Scenario: Packed aviation form
- **WHEN** the input is `402646N0795856W`
- **THEN** the result equals the DMS scenario's result

#### Scenario: Labeled reversed order
- **WHEN** the input is `lon: -79.98, lat: 40.45`
- **THEN** latitude is 40.45 and longitude is -79.98 with no order warning

### Requirement: Ambiguity is reported, never silently resolved
When an input admits more than one interpretation, the parser SHALL return the preferred interpretation plus all alternatives with reasons, and SHALL raise warning `AMBIGUOUS_INPUT`. Cases that SHALL be treated as ambiguous include: two unlabeled numbers without hemisphere letters (order assumed lat, lon); unlabeled pairs where the first value is outside ±90 (order assumed lon, lat, flagged); `E` that could be East or an exponent; a comma that could be a decimal comma or a separator; and strings valid in more than one grid notation (e.g. short geohashes).

#### Scenario: First value outside latitude range
- **WHEN** the input is `-105.27, 40.01`
- **THEN** the parser returns latitude 40.01, longitude -105.27 and warning `AMBIGUOUS_INPUT` stating the order was inferred as lon, lat because -105.27 cannot be a latitude

#### Scenario: Unlabeled pair, both orders valid
- **WHEN** the input is `40.45 -79.98`
- **THEN** the parser returns latitude 40.45, longitude -79.98, warning `AMBIGUOUS_INPUT`, and lists latitude -79.98, longitude 40.45 as the alternative

#### Scenario: Decimal comma ambiguity
- **WHEN** the input is `40,5 -79,9` and the number format setting is decimal point
- **THEN** the parser reports `AMBIGUOUS_INPUT`, prefers latitude 40.5 and longitude -79.9 (decimal commas), and lists the alternative reading as invalid because it would produce four numbers

### Requirement: Strict component validation
The parser SHALL reject: minutes or seconds ≥ 60; negative minutes or seconds; a minus sign combined with a hemisphere letter that contradicts it (`-40N`); latitude hemisphere letters on longitude values (`79W` as latitude); degrees beyond ±90 latitude or ±180 longitude (with longitude normalization per numeric-determinism only for decimal input); mixed notations within one coordinate; and trailing unparsed characters. Each rejection SHALL name the offending component.

#### Scenario: Seconds overflow
- **WHEN** the input is `40°26'60"N 79°58'56"W`
- **THEN** the result is `INVALID_INPUT` stating seconds must be less than 60

#### Scenario: Contradictory sign
- **WHEN** the input is `-40.5N, 79.9W`
- **THEN** the result is `INVALID_INPUT` stating that a minus sign contradicts hemisphere N

#### Scenario: Negative zero degrees
- **WHEN** the input is `-0°30'00", 10°`
- **THEN** latitude is -0.5 (the sign applies to the whole angle), not +0.5

### Requirement: Formatting with precision control
The formatter SHALL output DD, DMS, DDM, signed or hemisphere style, with user-selected decimal places, and SHALL carry rounding correctly across components (59.9999″ at 0 decimals becomes the next minute, never `60"`). Default precision SHALL correspond to about 1 cm on the ground (DD 7 decimals, DMS 0.001″, DDM 0.00001′), and the formatter SHALL report the ground resolution of the chosen precision.

#### Scenario: Rounding carry
- **WHEN** 10.9999999° is formatted as DMS with 0 decimal seconds
- **THEN** the output is `11°00'00"`

#### Scenario: Precision resolution reported
- **WHEN** a coordinate is formatted as DD with 4 decimals
- **THEN** the result reports a latitude resolution of about 11.1 m

### Requirement: Angle arithmetic
The domain SHALL provide angle arithmetic tools: add and subtract DMS angles, normalize, convert between degree, radian, gradian, and mil variants (per units spec), and compute the smallest signed difference between two bearings.

#### Scenario: Bearing difference across north
- **WHEN** the signed difference from bearing 350° to bearing 10° is requested
- **THEN** the result is +20°

### Requirement: Batch-safe parsing
Parsing tools SHALL support batch invocation where each line or row is parsed independently, with per-row warnings and errors.

#### Scenario: Mixed notation batch
- **WHEN** a batch contains DD, DMS, and MGRS rows
- **THEN** each row is decoded with its detected notation reported per row
