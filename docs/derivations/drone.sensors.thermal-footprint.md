<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Thermal pixel footprint (`drone.sensors.thermal-footprint`)

## Method

The tool turns a thermal camera's instantaneous field of view (IFOV) and line-of-sight distance into the width covered by one detector pixel. If the camera specification gives pixel pitch and focal length instead, their ratio supplies the IFOV. It then multiplies the one-pixel footprint by the requested number of pixels across the target, 3 by default. When a target size is entered, it rearranges the same relation to find the farthest measurement distance.

## Equations

- `IFOV = pixel pitch / focal length` when IFOV is not entered
- `pixel footprint = IFOV × distance`
- `minimum target width = pixels × IFOV × distance`
- `maximum distance = target width / (pixels × IFOV)`

IFOV is converted to radians and all lengths to meters before the equations are evaluated. The results are then displayed in the units declared by the output fields or selected profile.

## Symbols and units

IFOV is the angular width of one detector element, normally in milliradians. Pixel pitch and focal length are lengths and must use compatible units. Distance is the line-of-sight range to the target. Pixel footprint and target width are lengths. The pixel count is the required width across the target; FLIR recommends at least 3 × 3 pixels for an accurate temperature measurement.

## Domain

Enter either a positive IFOV or both a positive pixel pitch and focal length, never both forms. Distance must be above 0 and no more than 100 km. The target size, when entered, must be positive. The required pixel width is from 1 through 20 and defaults to 3.

## Approximations

The footprint uses the small-angle relation specified for the tool. The pitch-to-focal-length form uses their ratio rather than an optical distortion model. It is geometry only: lens blur, detector response, atmosphere, emissivity, reflected heat, view angle, focus, and camera calibration can require a larger target or shorter range. The 3 × 3 rule is manufacturer guidance, not a universal standard or an accuracy guarantee.

## Worked example

- sourcePublisher: Teledyne FLIR
- sourceTitle: Understanding Distance:Size Ratio
- sourceEdition: Web article, 2020-05-08
- sourceLocator: FLIR E8 worked example at 10 ft
- independent: yes
- inputs: equivalent IFOV 2.583333 mrad and distance 10 ft
- outputs: 0.31 in for one pixel and 0.93 in across 3 × 3 pixels
- tolerance: 0.01 in for the one-pixel value and 0.03 in for the 3-pixel value, matching the source's printed precision
- verifiedBy: golden vector v007 and `core/crates/gp-drone/tests/thermal.rs`
- verifiedOn: 2026-10-13

FLIR obtains 0.31 in from its E8 field-of-view calculator at 10 ft. Dividing 0.31 in by 120 in gives the equivalent 2.583333 mrad input. The tool returns 7.874 mm (0.31 in) for one pixel and 23.622 mm (0.93 in) for the default 3-pixel target width.

## Differential tests

- `tools/vectors/gen_sensing.py`: 26 vectors independently evaluate the stated relations in Python across direct IFOV and pitch/focal-length inputs, radians, degrees, milliradians, meters, feet, pixel rules from 1 through 20, inverse target distances, domain boundaries, and refusal cases
- vector v007 reproduces FLIR's published E8 example within the source's printed precision

## Invariants

- `core/crates/gp-drone/tests/thermal.rs` `angle_and_length_units_do_not_change_the_geometry`: equivalent angle and distance units leave both sizes unchanged
- `core/crates/gp-drone/tests/thermal.rs` `target_at_the_required_pixel_width_returns_the_starting_distance`: the inverse range returns the starting distance when the target equals the required footprint
- `core/crates/gp-drone/tests/thermal.rs` `pixel_pitch_over_focal_length_matches_the_same_ifov`: the two supported camera-input forms agree
- `core/crates/gp-drone/tests/thermal.rs` `flir_e8_worked_example_is_reproduced`: the published one-pixel and 3 × 3 sizes are reproduced
