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

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aeronautical Information Manual
- sourceEdition: current edition, checked 2026-10-13
- sourceLocator: paragraph 5-3-8j, pilot action, wind correction
- independent: yes
- inputs: the AIM gives an 8° left inbound correction and calls for 24° right outbound; the primary tool example uses an inbound course of 360°, 120 KTAS, and a 16.700772 kt crosswind from 270° to produce that 8° correction
- outputs: inbound correction 8° left (heading 352°) and outbound heading 204°, which is 24° right of the 180° outbound course; the tool also derives 118.83 kt on both legs and 60 seconds outbound
- tolerance: 1e-9 relative
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_aviation.py`: the FAA worked relationship plus 15 additional successful cases evaluated with a separate Python wind triangle and four refusal cases. They cover calm air, headwind, tailwind, crosswind from both sides, course wrap, longer timing, equivalent units, left turns, the animation start, and every refusal branch. Only the FAA row is an independent check
- `core/vectors/aviation.ifr.hold-wind-timing.jsonl`: those 20 vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `holding_wind_timing`: the published 8°/24° relationship holds; a direct headwind has no drift and gives the groundspeed-ratio timing; rotating the course and wind rotates only the headings; scaling airspeed and wind together scales groundspeeds but preserves angles and timing; doubling inbound time doubles outbound time; equivalent units agree; and turn direction leaves the operational outputs unchanged
