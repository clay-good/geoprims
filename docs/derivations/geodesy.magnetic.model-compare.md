<!-- Derivation note (trust/correctness-program layer A). -->
# Compare WMM and IGRF declination (`geodesy.magnetic.model-compare`)

## Method

WMM2025 and IGRF-14 are two published fits of the Earth's main magnetic field. The operation evaluates both at the same geodetic point and decimal year, with the same spherical-harmonic code as `geodesy.magnetic.declination`, and reports each declination and the differences WMM2025 less IGRF-14 in declination, inclination, and total intensity.

## Equations

- For each model: X, Y, Z from the model's coefficients at decimal year t, then D = atan2(Y, X), I = atan2(Z, H), F = √(X² + Y² + Z²) with H = √(X² + Y²)
- ΔD = ((D_WMM − D_IGRF + 540°) mod 360°) − 180°, the shorter way round
- ΔI = I_WMM − I_IGRF; ΔF = F_WMM − F_IGRF

## Symbols and units

X, Y, Z north, east, and down field components, H horizontal and F total intensity, in nanotesla; declination D and inclination I in degrees. Latitude and longitude in degrees; the date as a calendar date, turned into a decimal year that counts each calendar year as one; height above the ellipsoid.

## Domain

Dates both models cover, 2025.0 to 2030.0, and heights from −1 km to 850 km. A date or height outside is refused with OUT_OF_DOMAIN. Where either model's horizontal field is under 1 nT, declination is undefined and the operation refuses with DEGENERATE_GEOMETRY.

## Approximations

Both models describe only the main field from the core. Crustal anomalies, magnetic storms, and local interference are not included, and they often exceed the difference between the models. The difference says how far two fits of the same field disagree, not how wrong either one is.

## Worked example

- sourcePublisher: Kipp Collins (pygeomag) and Karl Laundal (ppigrf), after NOAA NCEI, the British Geological Survey, and IAGA
- sourceTitle: pygeomag (World Magnetic Model) and ppigrf (International Geomagnetic Reference Field)
- sourceEdition: pygeomag 1.1.0 with WMM2025; ppigrf 2.1.0 with IGRF-14
- sourceLocator: `GeoMag().calculate(glat, glon, alt, time)` for WMM2025; `ppigrf.igrf(lon, lat, h, date)` for IGRF-14, asked for the instant at the same fraction of 2025 to 2030 because ppigrf interpolates in elapsed time
- independent: yes
- inputs: Anchorage, 61.2181° N, 149.9003° W, on 2027-01-01
- outputs: WMM2025 about 13.921° east and IGRF-14 about 13.908° east, a difference of about 0.0126°, as in golden vector v001
- tolerance: 1e-6° on angles and 0.001 nT on intensity
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_model_compare.py`: 22 places on dates from 2025 to 2029, including heights of 1,655 m and 400 km, each model evaluated by its own library rather than the core.
- `core/vectors/geodesy.magnetic.model-compare.jsonl`: those vectors and two refusal cases.
- `core/crates/gp-geodesy/tests/geodesy.rs` `model_compare_matches_the_declination_tool` requires the declination tool's answer under each model.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `magnetic_views_invariants`: each declination equals the declination tool's answer for that model, and the difference is WMM less IGRF, brought within ±180°, at five places from the equator to the poles.
