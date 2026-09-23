<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Block time, out to in (`time.scale.block-time`)

## Method

Block time is a subtraction with one rule attached. Subtract the out time from the in time; if the in time is earlier and both are clock times with no date, the flight crossed midnight, so add 24 hours. That is the whole method, and the rule is what makes it worth a tool: 2215 to 0140 is three hours twenty-five, and a plain subtraction gives minus twenty hours thirty-five.

The midnight rule applies only when there is no date to settle the question. Give full timestamps and the dates decide, so a thirty-hour gap stays thirty hours instead of being folded into six. A clock-only pair can only ever describe a span under 24 hours, so wrapping it is the only reading that exists.

Times are accepted in the forms an operational document uses: `2215`, `22:15`, `2215Z`, or a full RFC 3339 timestamp. The `Z` is accepted and means what it says; mixing a zoned timestamp with a bare clock time is not a subtraction anyone can do correctly, so it is refused rather than assumed.

## Equations

- With dates: duration = in − out.
- Clock only: duration = (in − out) mod 24 h, in minutes.
- Equal clock times with no date give zero, not 24 hours: a zero-length block is the reading that matches what was written.

## Symbols and units

`out_time` and `in_time` are clock times or timestamps. Out come `minutes`, `hours` (decimal), `hm` (`h:mm`), `duration` (a readable phrase) and `note`, which says when the midnight rule was applied.

## Domain

Any pair of times the parser accepts. With dates, any span. Clock-only, a span from zero to 23:59.

## Approximations

None: the arithmetic is exact in whole minutes. The one judgement is the midnight rule, and it is a reading of the input rather than an approximation of a number.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: 14 CFR 1.1, General definitions
- sourceEdition: Current as of the review date
- sourceLocator: "Flight time": from moving under its own power for flight until coming to rest after landing
- independent: yes
- inputs: 24 pairs, including 2215 to 0140 across midnight, 2359 to 0000, 0005 to 0004, equal times, a full day either side of noon, and the same span written as 0915/1045, 09:15/10:45 and 0915Z/1045Z
- outputs: the minutes and the h:mm string in each case
- tolerance: exact
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-09-23

The expected values are computed in `tools/vectors/gen_time_scale.py` as `(in − out) mod 1440` in integer minutes, and the generator asserts each hand-written expectation against that formula before writing the vector, so a case written down wrong fails the generator rather than being published.

The pairs that matter are the ones a minute either side of midnight. 2359 to 0000 is one minute, not 1,439; 0005 to 0004 is 1,439 minutes, not one. A tool that took the absolute difference would get both backwards and still look right on every daytime flight.

## Differential tests

- `tools/vectors/gen_time_scale.py`: 18 of the 24 vectors, in integer minutes with a self-check
- `core/crates/gp-time/tests/time.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/time.scale.block-time.jsonl`: 24 vectors, every input form and both sides of midnight

## Invariants

- `core/crates/gp-time/tests/time.rs` `block_time_invariants`: 2215 to 0140 is 205 minutes, the case the midnight rule exists for, and it is not the 1,235 an absolute difference would give; a minute either side of midnight comes out one minute and 1,439 minutes the right way round; equal clock times give zero and not a full day; the three input spellings of one span — `0915`, `09:15`, `0915Z` — give identical results; every clock-only answer is under 24 hours, which is the property the wrap guarantees; and full timestamps are not wrapped, a span of more than a day staying more than a day
