<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Distance by vertical sextant angle (`navigation.piloting.distance-by-vertical-angle`)

## Method

From the sea horizon to the top of an object beyond the horizon, Bowditch Volume II Table 15's formula with its refraction constants. From the waterline to the top of an object within the horizon, Table 16's plane right triangle. The sea-horizon distance 1.17 √h is reported, and a waterline answer beyond it warns.

## Equations

- Table 15: D = √((tan a / 0.0002419)² + (H − h) / 0.7349) − tan a / 0.0002419.
- Table 16: D = H / tan a, in NM of 6,076.1 ft.
- Horizon: 1.17 √h NM.

## Symbols and units

a corrected angle in degrees; H object height and h height of eye in feet (meters converted); D nautical miles.

## Domain

Angles from just below the horizon to under 90°; the object higher than the eye for the sea-horizon form; a positive angle for the waterline form.

## Approximations

Standard terrestrial refraction, as the tables assume. Volume I §1209 notes the error when refraction differs.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9
- sourceEdition: 2024 edition
- sourceLocator: Volume II Table 15 (2024; Table 16 in the spec's 2019 numbering holds the plane triangle)
- independent: yes
- inputs: H − h = 100 ft at 0°30', and five other entries
- outputs: 1.8 NM, and the table's entries to 0.1 NM
- tolerance: 0.05 NM
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/piloting.rs`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_piloting.py distance-by-vertical-angle`: Python Table 15 formula at 14 random angles and heights (1e-9 NM) with the horizon (1e-12).
- `core/vectors/navigation.piloting.distance-by-vertical-angle.jsonl`: those vectors, run through the core on every build.

## Invariants

- `vertical_angle_table_15`: six Table 15 entries; a waterline answer beyond the horizon warns `BEYOND_HORIZON`.
