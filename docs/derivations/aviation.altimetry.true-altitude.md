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

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite altimetry spec, scenario "Flying into colder air"
- sourceEdition: add-aviation-suite change; ICAO Doc 8168 Vol II, 7th edition (2020) relation
- sourceLocator: openspec/changes/add-aviation-suite/specs/aviation/altimetry/spec.md, requirement "True altitude from indicated altitude"; the scenario states only that true altitude is below indicated, and the numbers are worked from the equation above in tools/vectors/gen_coldtemp.py
- independent: no
- inputs: indicated 8,000 ft, ISA deviation −20 °C, station elevation 0 ft
- outputs: true altitude 7,428.9 ft, error −571.1 ft, 4% rule −640 ft
- tolerance: 1e-6 relative
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_coldtemp.py`: the same ICAO equation and 4% rule written separately in Python, at six cases (cold and warm, sea-level and raised stations, zero deviation, FL350) plus one indicated altitude below the station that must fail (within 1e-6 relative). It checks the code against the stated equation, not against a separate method
- `core/vectors/aviation.altimetry.true-altitude.jsonl`: those seven vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool directly. This test runs the vectors above, which include ±20 °C giving errors of equal size and opposite sign (±571.1 ft) and a zero deviation giving zero error, and it requires at least five vectors per tool
