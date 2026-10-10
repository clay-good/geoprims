<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Holding wind correction and timing (`aviation.ifr.hold-wind-timing`)

## Method

Each leg of the hold is solved as a straight leg of the wind triangle. The inbound leg flies the inbound course; the outbound leg flies its reciprocal. For each, the wind correction angle comes from the crosswind component, and the groundspeed from the along-course components. The outbound time that gives the target inbound time is the inbound distance divided by the outbound groundspeed. Two outbound headings are given: the plain wind correction that holds the outbound track, and the FAA "triple the drift" heading (outbound course minus three times the inbound correction), which allows for drift in the turns and is a rule of thumb.

## Equations

- Outbound course: OC = IC + 180°, wrapped to [0°, 360°).
- For a course C: a = W_dir − C; s = (W_spd ÷ TAS) · sin a; WCA = asin s; GS = TAS · cos(WCA) − W_spd · cos a. No answer (NO_SOLUTION) when |s| > 1 or GS ≤ 0.
- Inbound heading = IC + WCA_in; outbound track heading = OC + WCA_out.
- Triple-the-drift outbound heading = OC − 3 · WCA_in.
- Outbound time = GS_in · t_in ÷ GS_out.

## Symbols and units

IC inbound course and W_dir the direction the wind blows from, in degrees with the same north reference; TAS and W_spd in knots; WCA in degrees, right positive; t_in the inbound leg time (default 1 min); outbound time in seconds.

## Domain

TAS > 0, wind speed ≥ 0, inbound time > 0. The wind must be weak enough to hold both courses with a positive groundspeed. The turn direction input is accepted but does not change these outputs.

## Approximations

Straight legs in a steady wind; drift in the turns is not modeled. The triple-the-drift heading is a rule of thumb from the FAA handbook, not an exact answer, and the outbound time is exact only for straight legs.

## Worked example

- sourcePublisher: geoprims (tool example, computed)
- sourceTitle: tool example "Inbound 360°, 120 KTAS, wind 300° at 20 kt", worked from the equations above
- sourceEdition: 2026; method from FAA-H-8083-15B (2012)
- sourceLocator: example "primary" in core/crates/gp-aviation/src/ifr.rs; method from FAA-H-8083-15B chapter 10 (correct into the wind inbound, triple it outbound); the handbook gives the method, not these numbers
- independent: no
- inputs: inbound course 360°, TAS 120 kt, wind from 300° at 20 kt
- outputs: inbound correction −8.3° (heading 351.7°), outbound heading 204.9° by triple the drift (188.3° to hold the track), groundspeed 108.7 kt inbound and 128.7 kt outbound, outbound time 50.7 s
- tolerance: 1e-9 relative
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: the same per-leg wind triangle and triple-the-drift rule written separately in Python, at five holds with headwind, tailwind, and crosswind cases (within 1e-9 relative). It checks the code against the stated equations, not against a separate method
- `core/vectors/aviation.ifr.hold-wind-timing.jsonl`: those five vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `holding_wind_timing`: a wind straight down the inbound course gives no correction and an outbound time of 60 × 100 ÷ 140 s; a crosswind from the left gives a left (negative) inbound correction, and the outbound heading is 180° − 3 × that correction (within 1e-9)
