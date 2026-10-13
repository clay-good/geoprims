<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Antenna height from a slant measurement (`survey.gnss.antenna-height`)

## Method

NGS defines antenna height as the vertical distance from the survey mark to the antenna reference point (ARP). Its field instructions call for several slant measurements from the mark to a named slant-height measurement point on the antenna. For one such measurement, the slant and the antenna radius form a right triangle. The tool reduces the slant to the vertical leg, then adds the signed vertical offset from the measurement point to the ARP. An offset above the measurement point is positive; one below it is negative. The radius and offset must come from the record or manual for the exact antenna and measurement point.

## Equations

- Vertical distance from the mark to the measurement point: `p = sqrt(s² - r²)`
- Vertical antenna height to the ARP: `v = p + o`
- Difference from using the raw slant: `d = s - v`

The iGage manual writes its model-specific offset as a positive distance `h` below the ARP and therefore prints `v = sqrt(s² - r²) - h`. In this tool that same value is entered as the signed offset `o = -h`.

## Symbols and units

`s` is the measured slant height, `r` is the horizontal radius from the antenna axis to the named measurement point, `o` is that point's signed vertical offset to the ARP, `p` is the vertical leg to the measurement point, `v` is the vertical height to the ARP, and `d` is the slant-minus-vertical difference. All are lengths. Inputs may use any supported length units; the calculation converts them to meters.

## Domain

The slant must be over 0 m, no more than 100 m, and longer than the radius. The radius must be 0 m or more. The signed measurement-point offset must be between -1 m and 1 m. A radius of 0 m is valid when the measurement point is on the antenna axis.

## Approximations

The equation itself is exact Euclidean geometry. It assumes the antenna axis is plumb over the mark, the tape is straight, and the radius and offset describe the exact point touched by the tape. It does not average repeated measurements, test the setup for plumb, or apply phase-center corrections. Those corrections belong to GNSS processing after the field height has been reduced to the ARP.

## Worked example

- sourcePublisher: iGage Mapping Corporation
- sourceTitle: iGage iG9 User Manual
- sourceEdition: Revision L, May 13, 2021
- sourceLocator: Page 83, "Slant Height" to "Vertical Height"
- independent: yes
- inputs: slant 2.123 m, receiver radius 0.07505 m, and SHMP offset 0.0742 m below the ARP (tool offset -0.0742 m)
- outputs: vertical height 2.0474 m
- tolerance: 0.0001 m, the precision printed by the source
- verifiedBy: golden vector v006 and `core/crates/gp-survey/tests/gnss.rs`
- verifiedOn: 2026-10-13

The manual also publishes 1.569 m slant as 1.4931 m vertical with the same receiver dimensions; golden vector v007 and the same invariant test check it. From the manual's rounded inputs, the tool returns 2.047473 m and 1.493004 m, both within 0.0001 m of the printed results.

## Differential tests

- `tools/vectors/gen_gnss.py`: 26 vectors independently evaluate the right-triangle reduction in Python across metric and US units, positive and negative offsets, a zero radius, close and far geometric limits, the two published iGage cases, and invalid slants, radii, and offsets

## Invariants

- `core/crates/gp-survey/tests/gnss.rs` `antenna_height_matches_the_igage_worked_examples`: both published iGage examples agree to the manual's printed 0.0001 m precision
- `core/crates/gp-survey/tests/gnss.rs` `antenna_height_preserves_the_right_triangle`: removing the signed offset reconstructs the measured slant by the Pythagorean identity
- `core/crates/gp-survey/tests/gnss.rs` `antenna_height_is_unit_invariant`: meters and an equivalent mix of centimeters and millimeters give the same result
- `core/crates/gp-survey/tests/gnss.rs` `antenna_height_offset_is_linear`: changing only the signed offset changes the vertical height by the same amount and the difference by the opposite amount
- `core/crates/gp-survey/tests/gnss.rs` `antenna_height_invalid_inputs_name_the_field`: invalid radius, slant geometry, and offset errors point to the field that needs correction
