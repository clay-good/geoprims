<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Rhumb line between two points (`navigation.rhumb.inverse`)

## Method

Karney's Rhumb, as in GeographicLib's Rhumb class. A rhumb line is straight in conformal (Mercator) coordinates, so the course is the arctangent of the longitude difference over the isometric-latitude difference. The distance follows from the meridian arc: it is the isometric-latitude difference times the mean rate ΔM/Δψ, evaluated by divided differences so that nearly east-west lines keep full accuracy (the plain quotient would be 0/0). The geodesic distance is computed alongside, so the page can show what the constant course costs.

## Equations

- ψ = isometric latitude, M = meridian distance, μ = rectifying latitude.
- Course: α = atan2(Δλ, Δψ).
- Distance: s = Δψ × D × sec α with D = ΔM/Δψ, or s = ΔM / cos α away from east-west; for Δψ = 0, s = a cos β × Δλ on the parallel.
- Extra distance: s − s_geodesic, and its percentage.

## Symbols and units

φ latitude and λ longitude (degrees, WGS 84), α the course (degrees clockwise from north), s the distance (meters, shown in the chosen unit), a the semi-major axis, β the parametric latitude.

## Domain

Any two points on any cataloged or custom ellipsoid. The course is constant, so a rhumb between points on the same parallel runs due east or west; between the same meridian, due north or south. Distances up to a full circuit of a parallel near a pole are handled.

## Approximations

Karney's series for the meridian arc and the conformal and rectifying latitudes are carried to enough terms for double precision on the WGS 84 flattening: within 1 µm of GeographicLib's RhumbSolve, 47 nm typical.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Bowditch, The American Practical Navigator (NGA Pub. 9), Article 910, Mercator sailing
- sourceEdition: 2024 edition
- sourceLocator: Example 1: a ship at lat 32°14.7'N, λ 66°28.9'W heading for Chesapeake Light, lat 36°58.7'N, λ 75°42.2'W, gives C 301.8° T and D 538.7 nm
- independent: yes
- inputs: 32°14.7′N, 66°28.9′W to 36°58.7′N, 75°42.2′W
- outputs: course 301.8°, distance 538.7 nm
- tolerance: 0.1° of course (as printed) and 1.5 nm of distance: Bowditch counts a minute of latitude as exactly 1 nm, but on WGS 84 a minute is 1,849 m near 35°, so its distance runs about 0.2% long (the tool gives 537.3 nm)
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `rhumb_tools_match_rhumbsolve`: 2,000 pairs through the public tools against GeographicLib 2.7's RhumbSolve, within 4.2e-7 m and 2.4e-11°
- `core/crates/gp-geo/tests/rhumb.rs`: the same pairs at the library level, including 200 nearly east-west, 100 nearly meridional, and 100 near-pole cases
- `tools/vectors/gen_rhumb_diff.py`: regenerates that fixture and the vectors
- `core/vectors/navigation.rhumb.inverse.jsonl`: 23 vectors from RhumbSolve and Bowditch, and v024, which pins how a sub-meter distance is shown

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `rhumb_invariants`: the rhumb is never shorter than the geodesic, reversing it keeps the distance and turns the course around, and half way along it the course to the end is unchanged
