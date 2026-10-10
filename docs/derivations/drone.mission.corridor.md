<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Corridor mapping lines (`drone.mission.corridor`)

## Method

A corridor is mapped with flight lines that run beside a centerline, such as a pipeline or road, and follow its bends. The tool projects the centerline to a flat plane centered on its middle point, copies it sideways once per line, and takes the corners back to latitude and longitude. The number of lines comes from the corridor width and the line spacing, the lines are centered on the centerline, and every second line is reversed so the drone flies back and forth.

## Equations

- Lines: n = ⌈width ÷ spacing⌉, at least 1.
- Offset of line k (from 0): (k − (n − 1) ÷ 2) × spacing, positive to the right of the centerline's direction.
- Ends: the centerline's end moved by the offset, square to its first or last leg.
- Bends: the corner sits on the bisector of the two legs, offset ÷ cos(half the turn) from the centerline's corner, so each leg stays the offset away. That distance is capped at 4 × the offset.
- Centerline length: the sum of the geodesic lengths of its legs (Karney 2013).
- Path length: the length on the plane of all lines in flight order, with the straight hops between them.

## Symbols and units

Centerline points in degrees on WGS 84, in order. Width, spacing, and offsets in meters by default; lengths in kilometers by default.

## Domain

Two or more centerline points, no two in a row the same (DEGENERATE_GEOMETRY), all within 50 km of the middle point (OUT_OF_DOMAIN). Width and spacing above zero. No more than 1,000 lines or 100,000 waypoints (LIMIT_EXCEEDED).

## Approximations

The plane is a transverse Mercator projection with scale 1 at the middle point. Distances across it are stretched by about 1 part in 10 million 3 km east or west of that point and 3 parts in 100,000 at 50 km, so offsets far from the middle of a long east-west corridor are short on the ground by that share. The 4 × cap applies where the centerline turns more than about 151°; there the lines no longer keep the offset. The lines carry no height and assume flat ground.

## Worked example

- sourcePublisher: GEOS (through Shapely) and PROJ (through pyproj), with GeographicLib for Python
- sourceTitle: Shapely 2.0.7 offset_curve (GEOS 3.11.4) with mitered joins; pyproj 3.6.1 (PROJ 9.3.0) transverse Mercator; geographiclib 2.1 Geodesic.WGS84.Inverse
- sourceEdition: GEOS 3.11.4, PROJ 9.3.0, geographiclib 2.1, run 2026-10-10
- sourceLocator: tools/vectors/gen_corridor.py, first case: a three-point centerline through 40°, −105° with a 17° bend, offset by −52.5, 0, and 52.5 m
- independent: yes
- inputs: centerline 39.99639704°, −105.010538846°; 40°, −105°; 40.006303957°, −104.990630783°; width 120 m; spacing 52.5 m
- outputs: 3 lines at −52.5, 0, and 52.5 m; centerline 2.0479 km; path 6.2487 km; the first line runs 39.9968291°, −105.0107886° to 40.0004030°, −105.0003348° to 40.0066598°, −104.9910356°
- tolerance: 2e-8° for each waypoint (about 2 mm); 1e-9 km for the centerline; 1e-6 km for the path
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-10-10

The same libraries give 17 more corridors, v007 through v023: two to six centerline points, one to six lines, bends up to 120°, from 55° S to 69° N, up to 52 km long.

## Differential tests

- `tools/vectors/gen_corridor.py`: every waypoint of every line from GEOS's mitered offset on PROJ's transverse Mercator plane, the centerline length from GeographicLib for Python, and the path length from the GEOS corners, for 18 corridors; the core uses none of the three. Bends go up to 120° with legs long beside the offsets, so no case reaches the 4 × cap or a pinched inside corner
- `tools/vectors/gen_drone.py`: the line count worked from the width and spacing in five cases
- `core/vectors/drone.mission.corridor.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `corridor_invariants`: along straight 1.5 km centerlines at four latitudes and four width and spacing pairs, every line end is its offset from the matching centerline end by the geodesic and on the correct side, offsets are centered and one spacing apart, odd lines run backward, and the path is the lines plus the hops between them
- `core/crates/gp-drone/tests/mission.rs` `pipeline_corridor`: the spec scenario, three lines at −52.5, 0, and 52.5 m
