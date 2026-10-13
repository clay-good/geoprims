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

The track is taken as straight, wind-free, and square to the station when the timing starts. Wind, a turn, or a late start move the answer. Across the recommended 5° to 20° bearing change, the rule reads about 3% to 5% long; at 10° it gives 12 min against the exact 11.52 min.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C
- sourceLocator: chapter 16, Figure 16-33 and the paragraph immediately following it
- independent: yes
- inputs: 75 seconds, bearing change 10°, groundspeed 120 kt
- outputs: 7.5 min and 15 NM by the published rule; the tool also gives 7.1985 min and 14.3969 NM from the exact right triangle
- tolerance: 1e-9 absolute
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_dme.py`: the FAA worked example plus 15 additional successful cases evaluated separately in Python and four refusal cases. They cover bearing changes from 0.1° to 89°, equivalent units, the optional groundspeed, and each invalid input. The generator restates the equations, so only the FAA row is an independent check
- `core/vectors/aviation.ifr.time-to-station.jsonl`: those 20 vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `time_to_station_invariants`: the exact time recovers the timed leg through sine, the rule recovers its 60-times relation, and both distances equal groundspeed times time. Doubling elapsed time doubles time and distance; doubling groundspeed doubles distance without changing time; equivalent time, angle, and speed units agree; and omitting groundspeed omits both distance outputs
