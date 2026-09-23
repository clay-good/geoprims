<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Check and repair a polygon (`geometry.validity.make-valid`)

## Method

A polygon is valid when its boundary means one unambiguous inside. The OGC's Simple Features specification says what that requires, and the failures are few: a ring that crosses itself, two rings that cross, a ring that touches itself at a point and pinches the interior in two, a hole outside its shell or overlapping another. Each is found by taking every pair of edges and asking whether they meet anywhere other than where consecutive edges are supposed to.

The check runs on an azimuthal equidistant plane at the shape's centre, with the geodesic edges cut into 5 km pieces first so that they are represented by their real curve rather than by a chord. Every problem found is reported with the coordinate where it happens, which is what makes the answer actionable rather than a verdict. Repair then re-forms the boundary into the valid shape it describes: a bow tie becomes the two triangles its own edges enclose. The repaired parts are counted and their area measured on the ellipsoid.

## Equations

- Validity, per OGC SF 6.1.11.1: every pair of non-consecutive edges must not intersect; consecutive edges meet only at their shared corner; holes lie inside the shell and do not overlap.
- Segment intersection by the orientation test: the sign of the cross product of each triple, with the collinear case handled by overlap on the shared line.
- Repair: split the boundary at every crossing and re-assemble the rings so that no two cross, keeping the orientation each region's own winding implies.
- Repaired area: the geodesic polygon area of each part, holes subtracted.

## Symbols and units

The polygon is latitude and longitude in degrees, holes by ring. `valid` is yes or no; `problem_count` and `problems` describe what is wrong and where; `parts` counts the repaired pieces and `area` their total area in the chosen unit; `repaired` is the mended boundary with `part` and `ring` on each vertex.

## Domain

Polygons with holes, up to a few hundred kilometres across, anywhere including the antimeridian. The plane the checks run on is placed at the shape's centre and is faithful over that span. A shape with fewer than three distinct corners has nothing to repair and is refused, saying what it needs.

## Approximations

The 5 km densification, so that a crossing between two long geodesic edges is found where the curves cross rather than where their chords do. The intersection tests themselves are exact sign tests, and a crossing is located to well under a millimetre at these scales. Repair is exact on the pieces it is given.

## Worked example

- sourcePublisher: the GEOS contributors
- sourceTitle: GEOS through shapely, `is_valid` and `make_valid`
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0
- sourceLocator: `is_valid` and `make_valid` on `+proj=aeqd +ellps=WGS84` coordinates, with the repaired area by geographiclib's PolygonArea
- independent: yes
- inputs: fourteen polygons — squares wound both ways, an L, bow ties, a crossed quadrilateral, a pinched ring, a repeated corner, a spike, and valid shapes at the equator, at 70° north and across the antimeridian
- outputs: the validity verdict, the number of repaired parts, and their area
- tolerance: exact on the verdict and the part count where pinned; 1e-8 relative on the area
- verifiedBy: golden vectors v008 to v021, run by the core on every build
- verifiedOn: 2026-09-23

GEOS is the implementation the rest of the field checks validity against, so agreement with it is the claim worth making. Over the fourteen polygons the verdicts and part counts match on twelve, and the repaired areas agree to 6.2e-11 relative on all fourteen.

The two that differ are worth their own paragraph, because neither is a fault. A square with a corner repeated is called valid by GEOS — a zero-length segment is not a topological problem — and a problem by this tool, which reports the duplicate. A square with a zero-area spike is repaired by GEOS into two parts, one of them the sliver, and by this tool into one. Both readings are defensible for a degenerate ring, and the specification does not settle them. Rather than pin either implementation's answer, those two vectors pin only the repaired area, which both agree on to a part in sixteen billion, and the limitations tell a user that a verdict on a degenerate ring is this tool's reading rather than the only one.

## Differential tests

- `tools/vectors/gen_makevalid_geos.py`: fourteen vectors from GEOS, appended to the frozen file, pinning only the area on the two degenerate rings
- `core/crates/gp-geometry/tests/validity.rs` `make_valid_invariants`: the repair's own validity, the area relationships, and the located problems
- `core/vectors/geometry.validity.make-valid.jsonl`: 21 vectors, the first seven from the original scenarios and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/validity.rs` `make_valid_invariants`: repairing a valid polygon returns it unchanged, with one part and the same area `geometry.area.polygon` gives it; repairing an invalid one produces a shape that is itself valid when fed back in, which is the property the tool exists to deliver and the one a partial repair would fail; a bow tie repairs into two parts whose areas sum to the reported total; the problems list is as long as the reported count, and every problem carries a coordinate inside the shape's own bounding box rather than a placeholder; winding the same polygon the other way changes neither the verdict nor the area; and a polygon with no area at all — every corner the same point — is refused, naming what it needs, rather than repaired into nothing
