<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# True course to compass heading (`aviation.wind.heading-chain`)

## Method

The navigation-log chain from the Pilot's Handbook: true course plus the wind correction angle is the true heading; the true heading less the variation (east positive, "east is least") is the magnetic heading; the magnetic heading less the deviation is the compass heading. The deviation is read from the user's compass card at the magnetic heading, by straight-line interpolation between the two nearest card entries around the circle, so a heading between 330° and 030° reads across north. Each step is returned. The tool does no wind math and looks up no variation.

## Equations

- True heading: TH = norm(TC + WCA).
- Magnetic heading: MH = norm(TH − VAR).
- Deviation: with the card sorted by heading, take entry a, the last at or before MH (or the last entry if none), and b, the next one around the circle; DEV = a.dev + ((MH − a.hdg) mod 360 ÷ (b.hdg − a.hdg) mod 360) × (b.dev − a.dev). With no card, DEV = 0; with one entry, DEV is that entry's value.
- Compass heading: CH = norm(MH − DEV).
- norm(x) = x mod 360, in [0°, 360°).

## Symbols and units

TC true course, WCA wind correction angle (right positive), VAR variation (east positive), DEV deviation (east positive), all in degrees.

## Domain

|WCA| < 90°, |VAR| ≤ 180°; card deviations within ±30°, each card heading listed once, at most 36 entries. Otherwise INVALID_INPUT.

## Approximations

The deviation between card entries is a straight-line interpolation, so it is only as good as the card. The wind correction and the variation are taken as entered. All other steps are exact arithmetic.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite wind-and-navigation spec, scenario "Deviation card interpolation"
- sourceEdition: add-aviation-suite change; method from FAA-H-8083-25C (2023)
- sourceLocator: openspec/changes/add-aviation-suite/specs/aviation/wind-and-navigation/spec.md, requirement "Heading chain" (+2° at 060° and −1° at 090° read +0.5° at 075°); the tool example's course, wind correction, and variation that lead to 075° are the project's own
- independent: no
- inputs: true course 070°, wind correction −5°, variation −10° (10° W), card 030° +1°, 060° +2°, 090° −1°, 120° −2°
- outputs: true heading 065°, magnetic heading 075°, deviation +0.5°, compass heading 074.5°
- tolerance: 1e-9 degree
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_heading.py`: the same chain and card interpolation written separately in Python, at six cases, including a card read across north (MH 005° between 330° and 030°) and no card, plus one card with a 40° deviation that must fail. It checks the code against the stated method, not against a separate one
- `core/vectors/aviation.wind.heading-chain.jsonl`: those vectors and the spec scenario, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool directly. This test runs the vectors above through the core and requires at least five per tool; among them, the same heading with a four-entry and a seven-entry card gives the same deviation, and zero course, correction, and variation with no card gives 000°
