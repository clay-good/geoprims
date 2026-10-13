<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# True altitude (`aviation.altimetry.true-altitude`)

## Method

An altimeter set to the local QNH assumes standard temperatures between the altimeter-setting station and the aircraft. When the air column is colder or warmer than standard by a fixed ISA deviation, the true height above the station differs from the indicated one. The tool applies the ICAO Doc 8168 temperature relation, the same one the cold-temperature correction uses, with the sign set so the result is true minus indicated, in both cold and warm air. The 4% per 10 °C rule is shown beside it.

## Equations

- Height above the station: H = indicated − H_station.
- Temperature error: E = (ΔT ÷ L0) · ln(1 + L0 · H ÷ (T0 + L0 · H_station)).
- True altitude = indicated + E.
- Rule of thumb: E_rule = 0.04 · (ΔT ÷ 10) · H.

## Symbols and units

L0 = −0.0019812 K/ft, T0 = 288.15 K. ΔT is the ISA deviation in °C (a temperature difference). Indicated altitude, station elevation (default 0 ft), H, and E are in feet.

## Domain

ISA deviation from −80 °C to +60 °C; station elevation from −2,000 ft to 15,000 ft; indicated altitude at or above the station and at most 60,000 ft (otherwise INVALID_INPUT).

## Approximations

The ISA deviation is taken as the same from the station up to the aircraft, which real air rarely holds. The relation uses a constant lapse rate and no humidity. It is not the procedure-altitude correction; that is the cold-temperature tool. The 4% rule is a rough check: at 8,000 ft and −20 °C it gives −640 ft against −571 ft.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: What's Your True Altitude?
- sourceEdition: FAA Safety Briefing, March/April 2014
- sourceLocator: Pages 25-26, sea-level airport at −30 °C; increasing the indicated decision altitude from 200 ft to 240 ft makes the missed approach occur at 200 ft true altitude
- independent: yes
- inputs: indicated altitude 240 ft, station elevation 0 ft, ISA deviation −45 °C (the source's −30 °C sea-level temperature minus the 15 °C ISA temperature)
- outputs: source 200 ft true altitude; equation 202.489 ft true altitude, or −37.511 ft error
- tolerance: 10 ft, one increment in the source's printed cold-temperature table
- verifiedBy: golden vector v008 and `core/crates/gp-aviation/tests/slice2.rs` `true_altitude_matches_faa_safety_briefing_example`
- verifiedOn: 2026-10-13

The source obtains its rounded value from the printed cold-temperature table. It does not use the equation, so the agreement checks the tool independently at the table's precision.

## Differential tests

- `tools/vectors/gen_coldtemp.py`: the same ICAO equation and 4% rule written separately in Python across 14 cold, warm, zero-height, boundary, sea-level, raised-station, and below-sea-level cases, plus five invalid cases. It checks the code against the stated equation, not against a separate method
- `core/vectors/aviation.altimetry.true-altitude.jsonl`: those cases and the independently published FAA example, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `true_altitude_invariants`: true altitude equals indicated altitude plus error; reversing the ISA deviation reverses the error; equivalent metric and Fahrenheit-difference inputs reproduce the outputs; zero ISA deviation gives exactly zero error
