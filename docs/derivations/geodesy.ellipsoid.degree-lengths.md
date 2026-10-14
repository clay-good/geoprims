<!-- Derivation note (trust/correctness-program layer A). -->
# Length of a degree of latitude and longitude (`geodesy.ellipsoid.degree-lengths`)

## Method

On an ellipsoid, a degree of latitude is a little longer near the poles than at the equator, and a degree of longitude shrinks with the cosine of the latitude to nothing at the poles. The operation reports both at a latitude, with the arcminute and arcsecond, from the same functions as `geodesy.ellipsoid.radii`.

## Equations

- Degree of latitude = M(φ + ½°) − M(φ − ½°), the meridian arc across the degree centered on φ
- Degree of longitude = (π / 180) × N(φ) × cos φ, with N = a / √(1 − e² sin² φ)
- Arcminute = degree / 60; arcsecond = degree / 3,600

## Symbols and units

φ geodetic latitude in degrees; M meridian arc; N prime-vertical radius of curvature. Lengths in meters.

## Domain

Latitudes from −90° to 90°. Within half a degree of a pole the degree of latitude is folded at the pole.

## Approximations

The lengths are for the degree centered on the latitude and change along it.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: GeographicLib, GeodSolve utility
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeodSolve -i -p 9` along the prime meridian from 59.5° to 60.5°, with the degree of longitude from the closed-form prime-vertical radius
- independent: yes
- inputs: latitude 60° on WGS 84
- outputs: a degree of latitude of 111,412.3 m and a degree of longitude of 55,800.0 m
- tolerance: 2e-6 m
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `degree_vectors`: 20 latitudes on five ellipsoids, the degree of latitude from GeodSolve and the degree of longitude from the closed form in Python.
- `core/vectors/geodesy.ellipsoid.degree-lengths.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same degrees as `geodesy.ellipsoid.radii`.

## Invariants

- The degree of longitude is zero at the poles and largest at the equator.
- The degree of latitude is smallest at the equator.
- 3,600 arcseconds make the degree.
