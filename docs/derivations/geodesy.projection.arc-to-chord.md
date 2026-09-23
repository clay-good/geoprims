<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Arc-to-chord correction (`geodesy.projection.arc-to-chord`)

## Method

On a map, the straight line between two points is not the projection of the straight line on the ground. A geodesic projects to a slight curve, and the angle between that curve and the straight grid line — at each end, separately — is the arc-to-chord correction, written **t − T** and known to surveyors as the second-term correction.

It is small: seconds of arc over a few kilometres. It is also the difference between a traverse that closes and one that does not, which is why it has a name.

The tool computes it exactly, from the projection rather than from the classical series:

- **t**, the grid bearing, is `atan2(ΔE, ΔN)` of the two projected points. That is what a protractor on the map measures.
- **T**, the projected geodesic's bearing at that end, is the geodesic azimuth there minus the grid convergence at that same point. The azimuth is Karney's, exact; the convergence is the projection's own.
- t − T at each end, in seconds of arc.

The two ends do not agree, and that is the physics rather than a defect: the curve leaves one end at one angle and arrives at the other at a different one. On a line running due north-south along a central meridian the correction vanishes at both ends; running east-west it is at its largest for the length.

## Equations

- t(from) = atan2(E₂ − E₁, N₂ − N₁); t(to) = atan2(E₁ − E₂, N₁ − N₂).
- T(from) = α₁ − γ₁; T(to) = (α₂ + 180°) − γ₂, with α the geodesic azimuths and γ the convergences.
- t − T at each end, wrapped to ±180° and reported in arcseconds.
- Grid distance = √(ΔE² + ΔN²); ellipsoid distance is the geodesic length.

## Symbols and units

`lat1`, `lon1`, `lat2`, `lon2` in degrees; `grid` is `utm` or `spcs`, with `zone` naming the zone. Out come `t_minus_t_from` and `t_minus_t_to` in arcseconds, both bearings, the grid and ellipsoid distances, and the grid used.

## Domain

Any two points within one zone. The tool refuses a grid it cannot place the points in rather than extrapolating a projection beyond its zone.

## Approximations

None in the method: this is the exact difference of two exactly computed bearings, not the truncated series most textbooks give. The remaining error is the geodesic solver's, which is at the level of nanoarcseconds.

## Worked example

- sourcePublisher: Charles Karney (GeographicLib)
- sourceTitle: GeographicLib's `GeoConvert` and `GeodSolve`
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeoConvert -u -z` for the grid coordinates, `-c` for the convergence, `GeodSolve -i` for the geodesic azimuths and length
- independent: yes
- inputs: 23 lines, including two running due north-south on a central meridian, two due east-west, one straddling a central meridian, where the two ends take the SAME sign rather than the opposite ones they take everywhere else, lines near a zone edge, long lines, high latitude, the equator, and three in the southern hemisphere
- outputs: the correction at each end and both distances
- tolerance: 1e-4 arcsecond on the corrections, 1e-5 m on the grid distance
- verifiedBy: golden vectors v001 to v023, run by the core on every build
- verifiedOn: 2026-09-23

Every quantity the reference uses comes from a GeographicLib command-line tool, and the arithmetic joining them is the definition above. Nothing is read back from the core. The cases that carry the most information are the degenerate ones: a line along the central meridian, where both corrections vanish, and a line straddling it. I expected the straddling line to give opposite signs at its two ends and it gives the same sign — opposite signs are what a line wholly on one side gives, which is the classical near-equal-and-opposite result. Crossing the meridian is where that rule stops holding, and the invariant now pins both halves of that.

## Differential tests

- `tools/vectors/gen_arc_chord.py` and `gen_geodesy_last.py`: 22 of the 23 vectors, from `GeoConvert` and `GeodSolve`
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.projection.arc-to-chord.jsonl`: 23 lines across both hemispheres and every orientation

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `arc_to_chord_invariants`: a line running along a central meridian has no correction at either end, to within a thousandth of an arcsecond, which is the geometry's own degenerate case; a line and the same line reversed give each other's corrections with the ends swapped, since it is one curve read from two directions; the correction grows with the line's length and with its distance from the central meridian; the grid distance and the ellipsoid distance differ by the scale factor, so their ratio is near one and on the right side of it for the zone; a line wholly on one side of the central meridian has corrections of opposite sign at its two ends, and a line straddling it has them of the same sign, which is the one place the classical rule turns over
