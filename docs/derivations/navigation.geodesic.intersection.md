<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Where two geodesic segments cross (`navigation.geodesic.intersection`)

## Method

Two geodesics on an ellipsoid cross at two antipodal points. Karney's method finds the closer one by iteration rather than by projecting onto a sphere: each step solves the inverse geodesic problem between the current estimates on the two lines, moves along each line by the amount that reduces the separation, and repeats until the two estimates are the same point. The result is reported as the displacement along each line from its own starting point, which is what tells you whether the crossing falls inside the segments or out beyond an end.

## Equations

- Each line is parameterised by arc length s from its start, with position from the direct geodesic problem.
- Given estimates x on line X and y on line Y, the inverse problem between them gives the distance and the two azimuths; the correction to (x, y) follows from the angle each line makes with the joining geodesic, and is applied until the distance falls below the tolerance.
- The displacements come back signed: negative is behind the segment's start, and greater than the segment's length is past its end.
- `within` is whether both displacements lie between zero and their own segment's length.

## Symbols and units

Latitudes and longitudes in degrees; displacements and segment lengths in metres. The crossing is returned as a latitude and longitude. `within` is yes or no, and `position` says which side of which segment a crossing outside them lies on.

## Domain

Any two segments on any ellipsoid in the registry, including segments that do not actually meet, where the crossing of their extensions is reported with `within` saying no. Two segments lying along the same geodesic have no single crossing and are refused.

## Approximations

The iteration is run to convergence, so the crossing is exact to the precision of the geodesic solver beneath it, which is Karney's and good to a few nanometres. What is approximate is the question rather than the answer: the closer of the two antipodal crossings is returned, so two routes each longer than a quarter of the globe can meet somewhere other than where the reader had in mind.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: IntersectTool, the intersection of two geodesic segments on WGS 84
- sourceEdition: GeographicLib version 2.7
- sourceLocator: `echo "40.6413 -73.7781 51.47 -0.4543 64.1466 -21.9426 38.7223 -9.1393" | IntersectTool -i -p 6` gives 4573738.528838 1271766.762973 0 0; run for this note
- independent: yes
- inputs: New York (40.6413, −73.7781) to London (51.47, −0.4543), crossed with Reykjavík (64.1466, −21.9426) to Lisbon (38.7223, −9.1393)
- outputs: the crossing at 53.37318484730815°, −14.563115347598284°, 4573738.528837532 m along the first and 1271766.762973459 m along the second, within both
- tolerance: 1e-6 m, which is the precision IntersectTool was asked to print
- verifiedBy: golden vector v001 and the 62 in the same file, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_intersect.py`: IntersectTool over segment pairs that cross inside, outside, near the poles, and across the antimeridian
- `core/vectors/navigation.geodesic.intersection.jsonl`: 62 vectors from that reference, run through the core on every build

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `geodesic_intersection_invariants`: over four pairs, naming the segments the other way round swaps the two displacements and leaves the crossing where it is; walking a segment backwards puts the crossing the same distance from its other end; the displacement reported along a segment equals the geodesic distance from that segment's start to the crossing; and `within` is exactly whether both displacements fall inside their own lengths
