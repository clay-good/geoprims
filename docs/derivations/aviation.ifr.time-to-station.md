<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Time and distance to a station (`aviation.ifr.time-to-station`)

## Method

The pilot turns so the station is abeam and times how long the bearing takes to change by Δ while flying a straight track square to the station. The track flown and the two bearing lines make a right triangle with the right angle at the start of the timing. From where the timing ends, the station is the hypotenuse away: the leg flown divided by sin Δ. Dividing by the groundspeed gives the time, which is t ÷ sin Δ and needs no groundspeed. The 60-times rule, 60 × t ÷ Δ, takes sin Δ ≈ Δ ÷ 60 (Δ in degrees) and is shown beside the exact answer.

## Equations

- Time to station: T = t ÷ sin Δ.
- Rule of thumb: T_rule = 60 × t ÷ Δ.
- With a groundspeed: D = GS × T ÷ 60; D_rule = GS × T_rule ÷ 60.

## Symbols and units

t is the timed interval in minutes; Δ the bearing change in degrees; GS the groundspeed in knots. T and T_rule are in minutes, D and D_rule in nautical miles.

## Domain

t > 0; 0° < Δ < 90° (otherwise INVALID_INPUT; 5° to 20° works best); GS > 0 when given.

## Approximations

The track is taken as straight, wind-free, and square to the station when the timing starts. Wind, a turn, or a late start move the answer. The rule reads long: about 4% at 10° (12 min against 11.52 min), growing with the change.

## Worked example

- sourcePublisher: geoprims (hand check, computed)
- sourceTitle: tool example "A 10° change in 2 minutes at 120 kt", worked from the equations above
- sourceEdition: 2026
- sourceLocator: example "primary" in core/crates/gp-aviation/src/dme.rs ("rule 60 × 2 ÷ 10 = 12 min; exact 2 ÷ sin 10° = 11.52 min"); method from FAA-H-8083-15B chapter 9
- independent: no
- inputs: 2 min, bearing change 10°, groundspeed 120 kt
- outputs: time 11.52 min (rule 12 min), distance 23.04 NM (rule 24 NM)
- tolerance: 1e-9 absolute
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_dme.py`: the same right-triangle relation and 60-times rule written separately in Python, at six cases from 1° to 20° (one without a groundspeed) and one 95° change that must fail (within 1e-9). It checks the code against the stated equations, not against a separate method
- `core/vectors/aviation.ifr.time-to-station.jsonl`: those seven vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool directly. This test runs the vectors above through the core and requires at least five per tool; the vectors show the rule reading longer than the exact time at every change from 1° to 20°
