<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# DME arc lead points (`aviation.ifr.dme-arc-lead`)

## Method

A standard-rate turn (3° per second) at groundspeed GS has radius GS ÷ (60π) NM, unless the pilot gives a radius. Turning 90° from a radial onto the arc, the turn starts one radius short of the arc's DME. Turning 90° off the arc onto an inbound radial, the turn circle lies inside the arc, tangent to both: its center is R − r from the station and r from the radial, so the lead angle is asin(r ÷ (R − r)). The rule of thumb, 60 × r ÷ R radials, is shown beside it.

## Equations

- Turn radius: r = GS ÷ (60π) NM (or as given).
- Lead onto the arc: r NM before the arc's DME.
- Lead off the arc: θ = asin(r ÷ (R − r)), in degrees of radial.
- Rule of thumb: θ_rule = 60 × r ÷ R.

## Symbols and units

R is the arc's DME in nautical miles; GS the groundspeed in knots; r the turn radius in nautical miles. Angles are in degrees.

## Domain

R > 0, GS > 0, r > 0; the turn must fit inside the arc, 2r < R, or the tool returns NO_SOLUTION.

## Approximations

A steady 90° turn in still air, at standard rate unless a radius is given. Wind moves both lead points. DME slant range is ignored, so R is taken as the ground distance. The rule of thumb reads a little short: 3.8° against 3.9° for a 10 DME arc at 120 kt.

## Worked example

- sourcePublisher: geoprims (hand check, computed)
- sourceTitle: tool example "A 10 DME arc at 120 kt", worked from the equations above
- sourceEdition: 2026
- sourceLocator: example "primary" in core/crates/gp-aviation/src/dme.rs ("radius 120 ÷ (60π) = 0.64 NM; lead asin(0.64 ÷ 9.36) = 3.9°, rule 3.8°"); method from FAA-H-8083-15B chapter 9
- independent: no
- inputs: arc 10 NM, groundspeed 120 kt
- outputs: turn radius 0.64 NM, lead onto the arc 0.64 NM, lead off the arc 3.9° (rule 3.8°)
- tolerance: 1e-9 absolute
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_dme.py`: the same turn-radius and tangent-circle relations written separately in Python, at six arcs from 7 to 20 NM (one with a given radius) and one turn too wide for its arc that must fail (within 1e-9). It checks the code against the stated equations, not against a separate method
- `core/vectors/aviation.ifr.dme-arc-lead.jsonl`: those seven vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool directly. This test runs the vectors above through the core and requires at least five per tool; in them the lead onto the arc always equals the turn radius and the exact lead off the arc is always larger than the rule's
