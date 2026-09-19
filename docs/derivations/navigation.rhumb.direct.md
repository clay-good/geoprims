<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Position along a rhumb line (`navigation.rhumb.direct`)

## Method

Karney's Rhumb, as in GeographicLib's Rhumb class, forward. Travel a distance on a constant course: the meridian arc gives the end's rectifying latitude, hence its latitude; the longitude follows from the isometric-latitude difference times the tangent of the course. A rhumb line spirals into a pole and cannot cross it, so a course with a northward or southward component that reaches the pole stops there; the tool warns RHUMB_REACHES_POLE and reports the distance left over.

## Equations

- μ2 = μ1 + s cos α / (a × quarter-meridian rate); φ2 from μ2.
- λ2 = λ1 + tan α × (ψ2 − ψ1), with ψ the isometric latitude.
- Due east or west (cos α = 0): λ2 = λ1 + s / (a cos β1), the parallel's radius.

## Symbols and units

φ latitude and λ longitude (degrees), α the course (degrees clockwise from north), s the distance (any length unit, up to 1e6 km), μ the rectifying and ψ the isometric latitude.

## Domain

Any start, any course, distances 0 to 1e6 km, on any cataloged or custom ellipsoid. At a pole the longitude is undefined, so the start's longitude is kept.

## Approximations

As in the inverse: Karney's series, within 1 µm of RhumbSolve (and 20 µm for starts within 0.01° of a pole, where the end longitude is ill-conditioned in the start).

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Bowditch, The American Practical Navigator (NGA Pub. 9), Article 910, Mercator sailing
- sourceEdition: 2024 edition
- sourceLocator: Example 2: a ship at lat 75°31.7'N, λ 79°08.7'W in Baffin Bay steams 263.5 nm on course 155°, arriving at L2 71°32.9'N, λ2 72°34.1'W
- independent: yes
- inputs: 75°31.7′N, 79°08.7′W, course 155°, distance 263.5 nm
- outputs: 71°32.9′N, 72°34.1′W
- tolerance: 1.2′ of latitude and 1.8′ of longitude: Bowditch counts a minute of latitude as exactly 1 nm, but on WGS 84 a minute is 1,861 m near 73°, so its run south is about 1.1′ too long (the tool gives 71°33.9′N, 72°35.6′W)
- verifiedBy: golden vector v025, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `rhumb_tools_match_rhumbsolve`: the direct problem for 1,900 of the 2,000 RhumbSolve pairs through the public tool, within 4.7e-8 m
- `core/crates/gp-geo/tests/rhumb.rs`: the same at the library level, including the near-pole starts
- `tools/vectors/gen_rhumb_diff.py`: regenerates that fixture and the vectors
- `core/vectors/navigation.rhumb.direct.jsonl`: 25 vectors from RhumbSolve and Bowditch, including a rhumb that stops at the pole

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `rhumb_invariants`: half the distance along a rhumb lies on the same line, with the same course and half the distance left
