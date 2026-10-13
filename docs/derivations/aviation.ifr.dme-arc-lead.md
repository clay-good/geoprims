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

- sourcePublisher: Chief of Naval Air Training
- sourceTitle: Primary Instrument Navigation, T-6B
- sourceEdition: CNATRA P-765 (Rev. 08-16), Change 4
- sourceLocator: Chapter 6, section 607, pp. 6-12 to 6-13
- independent: yes
- inputs: 12 DME arc, 180 kt groundspeed, and the manual's 0.5%-of-groundspeed turn-radius rule
- outputs: 0.9 NM turn radius; 60 / 12 = 5 radials per NM; 0.9 × 5 = 4.5 lead radials
- tolerance: 1e-9 absolute
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_dme.py`: the same turn-radius and tangent-circle relations written separately in Python at 16 arcs from 7 to 40 NM, including given radii and equivalent metric units, plus four refused inputs (within 1e-9). It checks the code against the stated equations, while the CNATRA vector independently checks the published training rule
- `core/vectors/aviation.ifr.dme-arc-lead.jsonl`: those 20 vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `dme_arc_lead_invariants`: scaling the arc and groundspeed together scales both distance outputs and leaves both angles unchanged; a given radius overrides groundspeed; equivalent metric inputs agree; and the exact lead satisfies the tangent-circle equation
