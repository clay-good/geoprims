<!-- Derivation note (trust/correctness-program layer A). -->
# Compass reliability zone (`geodesy.magnetic.uncertainty-zone`)

## Method

A magnetic compass aligns with the horizontal part of the Earth's field. Near the magnetic poles that part is weak and a compass wanders. The WMM2025 report defines a blackout zone where the horizontal intensity H is under 2,000 nT and a caution zone where it is under 6,000 nT, and gives an error model for declination that grows as H falls. The operation evaluates WMM2025 with the same code as `geodesy.magnetic.declination` and applies those definitions.

## Equations

- H = √(X² + Y²) from the model's north and east components
- Zone: blackout if H < 2,000 nT, caution if H < 6,000 nT, otherwise normal
- Declination uncertainty δD = √(0.26² + (5,417 / H)²) degrees, capped at 180°

## Symbols and units

X, Y north and east field components and H horizontal intensity, in nanotesla; declination D and its uncertainty in degrees. Latitude and longitude in degrees; the date as a calendar date; height above the ellipsoid.

## Domain

Dates within WMM2025's window, 2025.0 to 2030.0, and heights from −1 km to 850 km. A date outside is refused with OUT_OF_DOMAIN.

## Approximations

The model's main field only: crustal anomalies, disturbances from magnetic storms, and local interference are not included. The uncertainty is the report's one-sigma global estimate.

## Worked example

- sourcePublisher: Kipp Collins (pygeomag), after NOAA NCEI and the British Geological Survey
- sourceTitle: pygeomag, a Python implementation of the World Magnetic Model
- sourceEdition: pygeomag 1.1.0 with WMM2025
- sourceLocator: `GeoMag().calculate(glat, glon, alt, time)` for H and D, then the WMM2025 report's zone thresholds and error model
- independent: yes
- inputs: Resolute Bay, 74.697° N, 94.83° W, on 2026-07-01
- outputs: the caution zone, H about 3,346 nT, with the declination uncertainty and declination of golden vector v001
- tolerance: 1e-6° on angles and 1e-9 relative on H
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_compass_zone.py`: 20 places weighted toward both polar regions on dates from 2025 to 2029, each field evaluated by pygeomag rather than the core, skipping any within 5 nT of a zone edge.
- `core/vectors/geodesy.magnetic.uncertainty-zone.jsonl`: those vectors and two refusal cases.
- `core/crates/gp-geodesy/tests/geodesy.rs` `compass_zone_matches_the_declination_tool` requires the declination tool's answer.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `magnetic_views_invariants`: the uncertainty never falls below 0.26°, the zone follows the horizontal intensity alone, and across five places a weaker horizontal field never gives a smaller uncertainty.
