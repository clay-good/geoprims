<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# How two shapes relate (`geometry.predicate.relate`)

## Method

Every shape has three parts: its interior, its boundary, and everything else (its exterior). For a polygon the boundary is its rings; for a line it is its two ends (a closed line has none); a point is all interior. How two shapes relate is fully described by which of the first shape's three parts meet which of the second's, and in what dimension they meet: not at all, at points, along lines, or over an area. Those nine answers are the DE-9IM matrix, and every named relationship (intersects, touches, within, and the rest) is a pattern over it, defined in the OGC Simple Features standard.

The tool finds the nine answers by cutting both shapes against each other. Every edge is cut wherever the other shape crosses it, touches it, or runs along it. After that, nothing changes along an edge piece or at a node, so a few sample places settle everything:

1. Each node (every corner, and every place two edges cross) is located in both shapes. Where it falls gives a dimension-0 meeting.
2. Each edge piece is located in both shapes by its middle. A piece that runs along the other shape's edge is on that edge by construction, not by testing a midpoint. This gives the dimension-1 meetings.
3. Each polygon edge piece has an inside and an outside side. Each side is located in both shapes: its own side from which way the ring is wound, and the other shape's from whether the piece lies inside it, or, for a piece along the other polygon's edge, from which way that edge is wound. This gives the dimension-2 meetings. Every area where the parts of two shapes meet is bordered by some polygon edge, so looking at both sides of every edge finds all of them.

The two exteriors always meet over an area.

With planar edges, the test of which side of a line a place falls on is exact: it is Shewchuk's orientation predicate, which gets the sign of a 2 × 2 determinant right for any double-precision input by carrying the rounding errors as extra terms. A corner that sits on an edge is found to be on it whatever its digits, which is what makes the matrix trustworthy for shapes that share edges and corners. With geodesic edges, both shapes are put on the ellipsoidal gnomonic plane at their shared center, where geodesics are nearly straight, with each edge cut into geodesic pieces of at most 5 km first, and anything within 1 mm of an edge counts as on it.

## Equations

- Orientation of c against a → b: sign of (a.x − c.x)(b.y − c.y) − (a.y − c.y)(b.x − c.x). When the rounded value is within (3 + 16ε)ε of the sum of the two products' sizes, it is recomputed exactly: the determinant expanded as six products, each split into an exact pair (Dekker), and the twelve terms summed without loss (Shewchuk's Grow-Expansion); the sign is that of the largest nonzero term.
- Crossing of a → b and c → d: when c and d fall on opposite sides of a → b and a and b on opposite sides of c → d; the crossing point is a + t(b − a), t = ((c − a) × (d − c)) / ((b − a) × (d − c)).
- Inside a polygon (away from its edges): the parity of the upward edges with the place on their left and downward edges with it on their right, the exact form of the ray-crossing rule.
- Gnomonic plane at center c: x = (m12 / M12) sin α1, y = (m12 / M12) cos α1, with the reduced length m12, the geodesic scale M12, and the azimuth α1 from the geodesic inverse problem from c (Karney 2013, section 8).
- Named tests (OGC 06-103r4): disjoint FF*FF****; intersects is not disjoint; equals T*F**FFF*; within T*F**F***; contains T*****FF*; touches FT*******, F**T*****, or F***T**** (not for two point sets); crosses T*T****** when the first shape has the lower dimension, T*****T** when the higher, 0******** for two lines; overlaps T*T***T** for two point sets or two polygons, 1*T***T** for two lines.

## Symbols and units

Both shapes are latitudes and longitudes in degrees, a polygon's holes by ring number. `kind_a` and `kind_b` say whether each is a polygon, a line, or a set of points; `edges` chooses geodesic or planar edges. `matrix` is nine characters, row by row: the first shape's interior, boundary, and exterior against the second shape's interior, boundary, and exterior, each F (they do not meet) or the dimension 0, 1, or 2 of where they meet. `tests` answers the eight named tests yes or no, and `relation` names the most specific one that holds.

## Domain

Valid shapes: a polygon's rings do not cross themselves or each other and its holes lie inside its outline, and a line does not cross itself. Up to 2,000 corners per shape. Geodesic edges need every corner within 1,000 km of the shapes' shared center and at most 4,000 geodesic pieces of 5 km per shape; beyond that the tool refuses rather than approximating. Planar edges have no size limit other than the corner count, and draw straight lines in longitude and latitude, taking the coordinates as they are typed.

## Approximations

Planar: none. Every side-of-line decision is exact, and the one computed quantity, a crossing point, is never tested against the edges it came from, since it is on both by construction. A crossing point's rounding could in principle put it on the wrong side of a third, unrelated edge a few units in the last place away; in valid shapes a third edge only comes that close at a corner, and corners are exact inputs.

Geodesic: two, both far inside the 1 mm tolerance. The chord of a 5 km geodesic piece on the gnomonic plane is within 0.05 mm of the geodesic out to 1,000 km from the center (measured with GeographicLib: 0.04 mm at 1,000 km; the error grows with the square of both the distance and the piece length). And plane distances on the gnomonic projection differ from ground distances by up to about 2.5% at 1,000 km, so the 1 mm tolerance is between 1 and 1.025 mm on the ground.

## Worked example

- sourcePublisher: the GEOS contributors
- sourceTitle: GEOS relate (the reference DE-9IM implementation used by PostGIS, QGIS, and shapely)
- sourceEdition: GEOS 3.11.4, shapely 2.0.7
- sourceLocator: `shapely.relate(a, b)`
- independent: yes
- inputs: 1,500 random pairs of point sets, lines (a quarter of them closed), and polygons (134 with a hole) with corners on a 9 × 9 grid, 30% of the second shapes made from the first by a one-step shift or a reversal, so shared edges, corners on edges, and collinear overlaps are common; and 12 geodesic cases in general position, on the same gnomonic plane
- outputs: the nine-character matrix for each pair
- tolerance: exact, character for character
- verifiedBy: `core/crates/gp-geometry/tests/relate_parity.rs` over all 1,500 pairs; golden vectors v001 to v012 (geodesic) and v022 to v033 (planar)
- verifiedOn: 2026-09-24

GEOS is a separate code base with its own noding, labelling, and graph, written in C++ over two decades; the only thing the two share is the standard's definition of the matrix. On the grid both decide every side-of-line question exactly (GEOS with double-double arithmetic, the core with Shewchuk's expansions), so there is nothing for rounding to excuse, and all 1,500 matrices agree character for character.

The fixture was checked for teeth before it was kept. Reversing which side of a shared edge the other polygon's interior lies on fails 21 pairs; dropping the dimension-0 record at a crossing fails 257; treating a closed line's ends as its boundary fails 106 (the first draft of the fixture had no closed lines and missed this, which is why a quarter of the lines are now closed).

The geodesic vectors v001 to v012 compare with GEOS on a gnomonic plane built in Python with GeographicLib's own package, not the Rust port the core uses. Degenerate geodesic cases cannot come from GEOS that way, since a corner on a geodesic edge is 0.04 mm off the chord and GEOS is exact, so v017 to v021 are degenerate by construction instead: a point that GeodSolve put halfway along the field's south edge is on its boundary (F0FFFF212); a triangle whose corner is that point touches the field there (FF2F01212); a polygon sharing the south edge corner for corner touches along it (FF2F11212); the field wound the other way equals itself (2FFF1FFF2); and a line along the field's edge lies in its boundary (F1FF0F212).

Vectors v013 to v016 are why the edge choice matters. The geodesic from 45° N, 10° W to 45° N, 10° E bows north to 45.44° N at the prime meridian, so a point at 45.1° N or 45.3° N is outside a box with that southern edge on the ground (FF0FFF212) and inside it with straight edges along 45° N (0FFFFF212).

## Differential tests

- `tools/vectors/gen_relate_geos.py`: the GEOS fixture, and the 35 golden vectors (GEOS on the grid and on the gnomonic plane, constructions with GeodSolve, and two errors)
- `core/crates/gp-geometry/tests/relate_parity.rs` `relate_matches_geos_on_the_grid`: all 1,500 pairs, exact

## Invariants

- `core/crates/gp-geometry/tests/relate.rs` `relate_invariants`: over 400 grid pairs, in planar mode and on the ground at a 1 m grid step in geodesic mode: swapping the shapes transposes the matrix; within one way is contains the other way; intersects is never disjoint; intersects, equals, touches, crosses, and overlaps do not depend on the order; the exteriors always meet over an area; and the geodesic matrix at 1 m steps equals GEOS's planar one, since there a geodesic and a straight line part by microns. Across tools: a point's relation to a polygon agrees with `geometry.predicate.point-in-polygon` (inside is within, on the boundary is touches, outside is disjoint), and two polygons' interiors meet in an area exactly when `geometry.overlay.boolean` finds an intersection.
- `core/crates/gp-geometry/src/relate.rs` unit tests: the orientation test is exact on 200 points on a line far from the origin and one unit in the last place either side of it, where the naive determinant gets the sign wrong; and the named tests read their matrix patterns.
