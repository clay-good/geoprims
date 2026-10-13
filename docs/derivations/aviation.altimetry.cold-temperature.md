<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Cold temperature altitude correction (`aviation.altimetry.cold-temperature`)

## Method

The ICAO Doc 8168 (2020) correction for a procedure altitude when the airport is colder than standard. The ISA deviation is the reported airport temperature less the ISA temperature at the airport elevation. For each procedure altitude, its height above the airport goes into the ICAO equation, which assumes that deviation holds from the airport up to the altitude. The correction is added to the published altitude. Beside it the tool shows two approximations: the 4% per 10 °C rule, and AIM Table 7-3-1 read by bilinear interpolation. When the airport is at or above ISA, every correction is zero and a note says why.

## Equations

- ISA at the airport: T_ISA = 15 + L0 · H_aerodrome (°C); ΔT = T_reported − T_ISA
- Height above the airport: H = altitude − H_aerodrome
- ICAO correction (ΔT < 0): ΔH = (−ΔT / L0) · ln(1 + L0 · H / (T0 + L0 · H_aerodrome)); ΔH = 0 when ΔT ≥ 0
- Altitude to fly = altitude + ΔH
- 4% rule (ΔT < 0): 0.04 · (−ΔT / 10) · H; otherwise 0
- Table: bilinear interpolation in AIM Table 7-3-1 (heights 200 to 5,000 ft, temperatures +10 to −50 °C); a temperature above +10 °C uses the +10 °C row; outside the table no value is given

## Symbols and units

L0 = −0.0019812 K/ft, T0 = 288.15 K. Elevation, altitudes, H, and ΔH in feet; temperatures in °C. The top-level `correction` and `corrected` are for the first altitude in the list; each row carries its own correction, altitude to fly, rule, and table value.

## Domain

Airport elevation from −2,000 to 15,000 ft; reported temperature from −80 °C to 60 °C; one to 20 altitudes, each at or above the airport elevation (an altitude below it is INVALID_INPUT).

## Approximations

The ICAO equation assumes the ISA deviation stays constant from the airport to the altitude; real layers can differ. Transport Canada AC 500-020 writes T0 as 273 + 15, which moves a correction by about 0.1 ft. The 4% rule and the table (built for a sea-level airport) run higher than the equation; at the worked example the rule gives about 246 ft and the table 280 ft against the equation's 218 ft. They are shown only for comparison. Which segments to correct, and by which method, is set by the procedure, the FAA Cold Temperature Airports list in AIM 7-3, and the operator; the tool does not decide it.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aeronautical Information Manual
- sourceEdition: current edition effective August 7, 2025
- sourceLocator: Paragraph 7-3-1 and Table 7-3-1, worked statement that 1,314 ft at −12 °C is approximately 150 ft
- independent: yes
- inputs: height above airport 1,314 ft, reported temperature −12 °C
- outputs: approximately 150 ft from the FAA table; bilinear interpolation of its printed cells gives 141.912 ft, while the separate ICAO equation gives 123.683 ft
- tolerance: 10 ft, one printed table increment around the source's approximate value
- verifiedBy: golden vector v008 and `core/crates/gp-aviation/tests/slice2.rs` `cold_temperature_matches_aim_table_example`
- verifiedOn: 2026-10-13

This example independently verifies the table output, not the answer-first ICAO equation. The page labels the table as an approximation and displays both values separately.

## Differential tests

- `tools/vectors/gen_coldtemp.py`: the ICAO equation, 4% rule, and AIM table worked separately in Python across 15 airport, temperature, altitude, boundary, and interpolation cases, plus four invalid cases. It uses the same equation as the core, so it checks the arithmetic and table reading, not the choice of equation
- `core/vectors/aviation.altimetry.cold-temperature.jsonl`: those cases and the independently published FAA example, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/src/coldtemp.rs` `the_spec_scenario`: 1,500 ft above a 2,000 ft airport at −30 °C corrects by 218 ft within 1 ft, and a deviation of +0.5 °C gives exactly zero
- `core/crates/gp-aviation/src/coldtemp.rs` `the_table_reads_its_printed_cells_and_between_them`: the interpolated table returns its printed cells exactly (280 ft at −30 °C and 1,500 ft), the midpoint between two rows, and no value colder than −50 °C or above 5,000 ft
- `core/crates/gp-aviation/tests/slice2.rs` `cold_temperature_invariants`: corrected altitude equals published altitude plus correction for every row; the top-level values equal the first row; equivalent metric inputs reproduce the outputs; at ISA temperature the correction is exactly zero
