<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Decimal hours to hours and minutes (`time.scale.decimal-hours`)

## Method

This tool exists because of one mistake, made constantly: reading 1.3 hours as one hour thirty. It is one hour eighteen. The decimal part is a fraction of an hour, not a count of minutes, and the two agree only at .0 and .5.

So the conversion is minutes = hours × 60, rounded to the nearest whole minute, and the reverse is h + mm/60. The rounding is the only decision. A Hobbs meter reads in tenths of an hour, and a tenth is six minutes exactly, so every tenth lands on a whole minute and nothing is lost. Values that are not tenths — a tach time of 4.05, a computed 3.33 — round to the nearest minute, which is the resolution a logbook is kept in.

The same input field accepts both forms: `1.3` is decimal hours and `1:18` is hours and minutes, and both give the whole set of outputs, so the tool converts in either direction without a mode to set.

## Equations

- minutes = round(hours × 60).
- hours = h + mm / 60.
- A tenth of an hour is exactly 6 minutes; a hundredth is exactly 0.6 minutes.

## Symbols and units

`time` is decimal hours (`1.3`) or hours and minutes (`1:18`). Out come `minutes` (whole minutes), `hours` (decimal), `hm` (`h:mm`) and `duration` (a readable phrase).

## Domain

Any non-negative duration. This is a duration, not a time of day, so it is not wrapped at 24 hours: 100 hours is 100 hours, which is what a total time or an interval between inspections needs.

## Approximations

Rounding to the nearest minute is the only one, and it is deliberate. A duration of 0.016666… hours is one minute; 4.05 hours is 243 minutes, from 243.0 exactly; a value falling between two minutes goes to the nearer.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: 14 CFR 1.1, General definitions
- sourceEdition: Current as of the review date
- sourceLocator: "Flight time": from moving under its own power for flight until coming to rest after landing — the quantity a logbook records in decimal hours
- independent: yes
- inputs: 25 durations, including 1.3 h, a tenth, a hundredth, 4.05 h, 23.99 h, 100 h, and the h:mm forms 1:18, 2:45 and 12:00
- outputs: the minutes, the decimal hours, and the h:mm string in each case
- tolerance: exact on minutes, 5e-16 relative on decimal hours
- verifiedBy: golden vectors v001 to v025, run by the core on every build
- verifiedOn: 2026-09-23

The expected minute counts are computed in `tools/vectors/gen_time_scale.py` as integers, not floats, so a float rounding in the core cannot be reproduced by accident in the reference. Every case asserts the whole set — minutes, decimal hours and the h:mm string together — so a tool that got the minutes right and formatted them wrong still fails.

## Differential tests

- `tools/vectors/gen_time_scale.py`: 20 of the 25 vectors, in integer minutes
- `core/crates/gp-time/tests/time.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/time.scale.decimal-hours.jsonl`: 25 vectors, both input forms, tenths, non-tenths, and a duration over a day

## Invariants

- `core/crates/gp-time/tests/time.rs` `decimal_hours_invariants`: 1.3 hours is 78 minutes and 1:18, which is the error the tool exists to prevent, asserted directly against the 90 minutes a misreading would give; the two input forms agree, so `1.3` and `1:18` produce identical results; every tenth of an hour is a whole number of minutes, none of the ten losing anything to rounding; the conversion round trips, decimal to h:mm and back; it is monotonic, a longer duration never giving fewer minutes; and it does not wrap at a day, 25 hours staying 1,500 minutes rather than becoming 60
