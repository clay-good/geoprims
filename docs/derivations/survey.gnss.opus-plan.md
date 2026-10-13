<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# OPUS session planning (`survey.gnss.opus-plan`)

## Method

The tool first converts the planned observation duration to minutes. It applies the duration windows on the current NGS OPUS upload and processing pages: files from 15 minutes up to, but not including, 2 hours use RSGPS rapid-static processing; files from 2 through 48 hours use PAGES static processing. It then names an optional RINEX 2 observation file from the four-character mark ID, UTC start date, and UTC start hour, following section 5.1 of the NGS OPUS Projects User Guide.

## Equations

- Day of year: `ddd = 1 + days elapsed since January 1 in the UTC start year`
- Partial-day session character: `h = character('a' + UTC start hour)`, so `a = 00`, `b = 01`, ..., and `x = 23`
- A 24-hour data set uses the special session character `0`
- RINEX 2 observation short name: `ssssdddh.yyo`

Here `ssss` is the lower-case four-character mark ID, `ddd` is zero-padded to three digits, and `yy` is the last two digits of the year.

## Symbols and units

The session duration is a time and may use any supported time unit. `ddd` is an integer from 001 through 365, or 366 in a leap year. The UTC start hour is a whole number from 0 through 23. The year is limited to 1980 through 2079 so its two-digit RINEX 2 form stays within the tool's documented era.

## Domain

OPUS accepts files from 15 minutes through 48 hours. A date must be a real Gregorian date from 1980 through 2079. A mark ID must contain exactly four ASCII letters or digits. A partial-day file needs a whole UTC start hour from 0 through 23. A station or start hour without a date is refused because those values cannot form a complete file name. A 24-hour file does not need a start hour because NGS assigns it the special session character `0`.

## Approximations

There is no numerical approximation in the duration classification, calendar conversion, or file-name construction. The result is planning guidance, not an OPUS acceptance prediction. The tool does not inspect the observation file, receiver, observables, sampling interval, continuity, satellite geometry, or nearby CORS coverage. NGS can revise its service rules, so the dated OPUS source remains authoritative.

## Worked example

- sourcePublisher: NOAA National Geodetic Survey
- sourceTitle: OPUS Projects User Guide
- sourceEdition: Version 2.0
- sourceLocator: Section 5.1, "RINEX Files Naming Convention for GNSS Observations"
- independent: yes
- inputs: mark ID g287, January 9, 2023, start hour 16:00 UTC, and a 60-minute session
- outputs: rapid-static processing and RINEX 2 file name `g287009q.23o`
- tolerance: exact text and integer agreement
- verifiedBy: golden vector v026 and `core/crates/gp-survey/tests/gnss.rs`
- verifiedOn: 2026-10-13

The guide publishes `g287009q.23o`: `009` is January 9 and `q` is 16:00 UTC. The current NGS processing page independently classifies a 60-minute file as rapid static.

## Differential tests

- `tools/vectors/gen_gnss.py`: 25 live vectors independently derive processing classes, leap and non-leap day numbers, all boundary rules, metric time conversions, start-hour letters, the special 24-hour code, and invalid field combinations. Three older vectors remain recorded as superseded because they incorrectly used `0` for partial-day files.

## Invariants

- `core/crates/gp-survey/tests/gnss.rs` `opus_plan_matches_the_ngs_rinex_example`: reproduces the exact NGS-published file name and rapid-static classification
- `core/crates/gp-survey/tests/gnss.rs` `opus_plan_switches_to_static_at_two_hours`: checks the 15-minute, just-below-2-hour, 2-hour, and 48-hour processing boundaries
- `core/crates/gp-survey/tests/gnss.rs` `opus_plan_maps_utc_hours_and_calendar_days`: checks leap day, year end, and the `a`, `m`, and `x` hour codes
- `core/crates/gp-survey/tests/gnss.rs` `opus_plan_uses_zero_only_for_a_24_hour_file`: distinguishes the daily `0` code from a partial-day hour letter
- `core/crates/gp-survey/tests/gnss.rs` `opus_plan_invalid_inputs_name_the_field`: incomplete and non-integral naming inputs point to the field that must be corrected
