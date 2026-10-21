<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ISO week date (`time.scale.iso-week`)

## Method

ISO 8601-1 numbers weeks from Monday to Sunday, gives each week to the year that holds its Thursday, and makes week 01 the week holding 4 January. The tool finds the weekday from the day count (1970-01-01 was a Thursday), moves to that week's Thursday, and reads the week-year and the week number from the Thursday's day of year. In reverse, week 01's Monday is 4 January less its weekday, and the date is that Monday plus seven days a week and the weekday. A year has 53 weeks when it starts on a Thursday, or on a Wednesday in a leap year.

## Equations

- Weekday d = ((days since 1970-01-01 + 3) mod 7) + 1, Monday 1 to Sunday 7.
- Thursday = date − d + 4; week-year = the Thursday's year; week = (Thursday − 1 January of that year) div 7 + 1.
- Date = (4 January − (its weekday − 1)) + 7 (week − 1) + (d − 1).

## Symbols and units

Dates as YYYY-MM-DD; week dates as YYYY-Www-D (or YYYY-Www for the Monday).

## Domain

Years 0001 to 9999; a week past the year's last (week 53 in a 52-week year) or a weekday outside 1 to 7 is INVALID_INPUT naming the year's weeks.

## Approximations

None: exact calendar arithmetic. Only the ISO rule; calendars that start weeks on Sunday or count the week of 1 January as week 1 differ.

## Worked example

- sourcePublisher: International Organization for Standardization
- sourceTitle: ISO 8601-1:2019, Date and time, Part 1 (with Amd 1:2022)
- sourceEdition: ISO 8601-1:2019 with Amd 1:2022
- sourceLocator: week dates; the add-celestial-and-moon scenarios 2027-01-01 and 2026-01-01
- independent: no
- inputs: 2027-01-01
- outputs: 2026-W53-5, a Friday; Monday 2026-12-28, Sunday 2027-01-03
- tolerance: exact
- verifiedBy: golden vector v001 and `core/crates/gp-time/tests/calendar.rs` `iso_week_scenarios_and_rule`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_calendar.py week`: 17 dates around year ends and in 53-week years from Python's date.isocalendar, four week dates back to dates by date.fromisocalendar, and a refusal (2025 has 52 weeks); 22 vectors in `core/vectors/time.scale.iso-week.jsonl`.

## Invariants

- `core/crates/gp-time/tests/calendar.rs` `iso_week_scenarios_and_rule`: the spec's anchors; Monday and Sunday of a week; the reverse conversion and a week alone; the refusal of week 53 in a 52-week year; for every year 1970 to 2100, 4 January is in week 01 and 28 December in the year's last week.
