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

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C (2023)
- sourceLocator: Chapter 16, Figure 16-26 and page 16-20, Chickasha Airport to Guthrie Airport flight log
- independent: yes
- inputs: true course 031°, wind correction 3° left, variation 7° E, compass correction +2°
- outputs: true heading 028°, magnetic heading 021°, compass heading 023°
- tolerance: 1e-12° for the printed whole-degree values
- verifiedBy: golden vector v009 and `core/crates/gp-aviation/tests/slice2.rs` `heading_chain_matches_phak_flight_log`
- verifiedOn: 2026-10-13

The handbook's flight log records the amount to add to magnetic heading as a +2° compass correction. This tool's input is the physical deviation with east positive, which is subtracted, so the same correction is entered as −2° deviation. The resulting 023° compass heading is unchanged.

## Differential tests

- `tools/vectors/gen_heading.py`: the same chain and card interpolation written separately in Python across direct, east- and west-variation, north-wrapping, one-entry, exact-entry, unsorted-card, and cross-north interpolation cases, plus invalid wind correction, variation, duplicate-heading, and deviation cases. It checks the code against the stated method, not against a separate one
- `core/vectors/aviation.wind.heading-chain.jsonl`: those cases and the independently published FAA flight log, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `heading_chain_matches_phak_flight_log`: the full FAA chain reproduces 028° true, 021° magnetic, and 023° compass
- `core/crates/gp-aviation/tests/slice2.rs` `heading_chain_invariants`: adding 360° to true course leaves every output unchanged; east variation subtracts from true heading; an empty card leaves magnetic and compass headings equal; interpolation across north follows the same signed-deviation equation
