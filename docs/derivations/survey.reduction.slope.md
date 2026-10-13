<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Slope distance to horizontal and vertical (`survey.reduction.slope`)

## Method

Resolve a measured slope distance into horizontal and vertical components using its zenith angle. A vertical angle is first changed to its complementary zenith angle. When both telescope faces were observed, reduce the face-right reading to its face-left equivalent and average the pair; their departure from a full circle is the vertical-index error. Instrument height and target height carry the line-of-sight difference to the station marks. A requested long-sight correction adds the standard curvature-minus-refraction term only when the horizontal distance is beyond the entered threshold.

## Equations

- Z = 90° − α when α is a vertical angle from the horizon.
- HD = SD sin Z; VD = SD cos Z.
- Mean Z = (FL + 360° − FR) / 2; index error = (FL + FR − 360°) / 2.
- Elevation difference = HI + VD − HR + C.
- C = (1 − k) HD² / (2R) when HD is beyond the entered correction threshold.

## Symbols and units

SD is slope distance, HD horizontal distance, VD vertical difference, Z zenith angle, α vertical angle, FL and FR the face-left and face-right zenith readings, HI instrument height, HR target or reflector height, C curvature and refraction, k the refraction coefficient, and R the mean Earth radius. All distances in one compatible length unit; angles in degrees or DMS. US survey feet and international feet are not mixed.

## Domain

Slope distance must be positive. A face-left zenith is from 0° through 180° and a face-right reading from 180° through 360°. A vertical angle is from −90° through 90°. Give either a zenith or a vertical angle, not both. The optional refraction coefficient is from 0 through 1.

## Approximations

The component reduction is plane trigonometry over one sight. The optional long-sight correction uses a spherical Earth of radius 6,371,000 m and one refraction coefficient, 0.13 by default. It does not include atmospheric ppm, prism constant, elevation factor, or grid scale. Those corrections are separate tools.

## Worked example

- sourcePublisher: U.S. Geological Survey
- sourceTitle: Procedures and Best Practices for Trigonometric Leveling in the U.S. Geological Survey
- sourceEdition: Techniques and Methods 11-D3 (2020)
- sourceLocator: Section 5 example reduction table, section 1 to 2
- independent: yes
- inputs: mean slope distance 127.531 ft, mean zenith 90°04′27″, instrument height 4.850 ft, target height 4.830 ft
- outputs: mark-to-mark vertical difference −0.145 ft
- tolerance: 0.0005 ft, the precision of the printed result
- verifiedBy: golden vector v011, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py`: evaluates the trigonometric components, two-face mean, index error, height reduction, and curvature term independently in Python for 20 valid and refused cases
- `core/vectors/survey.reduction.slope.jsonl`: those 20 vectors, including the published USGS reduction, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `slope_reduction_invariants`: horizontal and vertical components reconstruct and scale with the slope distance; complementary vertical and zenith angles agree; two-face readings reduce at their mean and expose their index error; instrument and target heights enter with opposite signs; and curvature switches on only beyond its threshold
