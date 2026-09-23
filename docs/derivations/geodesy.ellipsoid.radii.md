<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Radii of curvature and degree lengths (`geodesy.ellipsoid.radii`)

## Method

An ellipsoid curves differently along the meridian than across it, so at any latitude there are two radii of curvature rather than one: the meridional radius M, how sharply the surface bends going north, and the prime-vertical radius N, how sharply it bends going east. Every other figure here follows from those two — their geometric mean, the radius in any azimuth by Euler's formula, and the length of a degree of latitude or longitude, which is simply the arc one degree subtends on the corresponding circle. The meridian arc from the equator is the integral of M, evaluated in closed form.

## Equations

- W = √(1 − e² sin²φ).
- Prime vertical: N = a / W. Meridional: M = a(1 − e²) / W³.
- Gaussian mean curvature radius: R = √(MN).
- Euler, the radius in azimuth α: 1/R(α) = cos²α / M + sin²α / N.
- One degree of latitude: M · π/180. One degree of longitude: N cos φ · π/180.
- Meridian arc from the equator: a[E(φ, e) − e² sin φ cos φ / W], with E the incomplete elliptic integral of the second kind, evaluated by Carlson's symmetric forms.

## Symbols and units

φ is geodetic latitude in degrees, α an azimuth in degrees clockwise from north. a is the semi-major axis and e² the first eccentricity squared of the chosen ellipsoid. All radii, degree lengths, and arcs are in metres.

## Domain

Latitudes −90° to 90°, any azimuth, any ellipsoid in the registry or one given by a and 1/f. At the poles a degree of longitude is zero and M equals N, which is the one latitude where a single radius of curvature is the whole story.

## Approximations

The radii and degree lengths are closed-form identities with no approximation in them. The meridian arc uses Carlson's elliptic integrals, which are evaluated to the precision of a double rather than truncated as a series, so the arc agrees with an independent geodesic solver to a micrometre over thousands of kilometres.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeodSolve, the inverse geodesic problem on WGS 84
- sourceEdition: GeographicLib version 2.7
- sourceLocator: `echo "44.5 0 45.5 0" | GeodSolve -i -p 6` gives 111131.777653 m, and `echo "0 0 45 0" | GeodSolve -i -p 6` gives 4984944.377978 m; both run for this note
- independent: yes
- inputs: lat 45
- outputs: degree of latitude 111131.77765280259 m, meridian arc from the equator 4984944.377977744 m
- tolerance: 1e-6 m, which is the precision GeodSolve was asked to print
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_frames.py`: mpmath at 40 digits, evaluating the closed forms independently of the core, across latitudes and ellipsoids
- `core/vectors/geodesy.ellipsoid.radii.jsonl`: 25 vectors from that evaluation, run through the core on every build

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `ellipsoid_radii_invariants`: over seven latitudes, the meridional radius is the smaller, the Gaussian mean lies between the two and equals √(MN), Euler's formula gives M due north and N due east and agrees at 45°, a degree of longitude equals N cos φ in radians and falls below a degree of latitude away from the equator, and the meridian arc only grows going north
- `core/crates/gp-geodesy/tests/frames.rs` `degree_lengths_at_45`: the spec's own scenario, including a degree of longitude going to zero at the pole
