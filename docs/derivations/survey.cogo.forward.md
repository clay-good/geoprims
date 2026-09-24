<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Coordinates from a bearing and distance (`survey.cogo.forward`)

## Method

Plane coordinate geometry. The bearing or azimuth is read as a grid azimuth, and the horizontal distance is split into its latitude (the northing change) and departure (the easting change), which are added to the known point. Bearings are accepted as quadrant bearings (N 45°00′00″ E or S44-30-00W) or decimal azimuths. US survey feet and international feet may not be mixed.

## Equations

- Latitude = d · cos(az), departure = d · sin(az)
- N₂ = N₁ + d · cos(az), E₂ = E₁ + d · sin(az)
- A quadrant bearing converts to an azimuth: N θ E = θ, S θ E = 180° − θ, S θ W = 180° + θ, N θ W = 360° − θ

## Symbols and units

N northing and E easting of the known point, d the horizontal distance, all in one length unit, which the new coordinates keep. az is the azimuth in degrees clockwise from grid north.

## Domain

Any plane point, any direction, and any horizontal distance. A direction that is not a readable bearing or azimuth is refused with a message naming the field.

## Approximations

None on the plane. The distance must be a horizontal grid distance: reduce slope distances first and scale ground distances by the combined factor. A true or magnetic bearing needs the convergence or declination applied before it is a grid bearing.

## Worked example

- sourcePublisher: University of Memphis, Department of Civil Engineering
- sourceTitle: CIVL 1112 Surveying - Traverse Calculations (course notes)
- sourceEdition: retrieved 2026-09-24
- sourceLocator: Page 4, the latitudes and departures table: course AB, S 6°15′ W, 189.53 ft, latitude −188.403, departure −20.634, and courses BC through EA
- independent: yes
- inputs: from (0, 0), S 6-15 W, 189.53 ft
- outputs: northing −188.403, easting −20.634 ft; the other four courses likewise
- tolerance: 0.0005 ft (the table's rounding)
- verifiedBy: golden vectors v006 to v010, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_survey.py`: an independent Python implementation at ten random state-plane-sized points with quadrant bearings to the second, in every quadrant (within 1e-11 relative)
- `core/vectors/survey.cogo.forward.jsonl`: those vectors, the published table, decimal azimuths due north and due west, and a point in meters, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `inverse_forward_invariants`: forward undoes inverse at four pairs from sub-foot to state-plane scale; see `survey.cogo.inverse`
