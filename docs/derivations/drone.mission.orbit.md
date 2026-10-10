<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Orbit pattern (`drone.mission.orbit`)

## Method

An orbit is a ring of camera positions around a subject, each looking at it. The tool places the waypoints at equal steps of azimuth around the center, each one the given radius away along the geodesic on WGS 84, so the ring is a true circle on the ground at any latitude. Each waypoint's heading is the direction back to the center, and the gimbal pitch is the angle down from the flying height to the target height at that radius. It also gives the length of the ring and the spacing between photos along it.

## Equations

- Waypoint k of n: the geodesic direct problem from the center at azimuth 360° × k ÷ n (negative for counterclockwise) and distance r (Karney 2013).
- Heading at the waypoint: the geodesic's arrival azimuth turned by 180°, which is the way back to the center.
- Gimbal pitch: −atan((flying height − target height) ÷ r).
- Circumference: the sum of the n geodesic chords between neighbors, times (π ÷ n) ÷ sin(π ÷ n), which turns a regular polygon's perimeter into its circle's.
- Photo spacing: circumference ÷ n.

## Symbols and units

Center latitude and longitude in degrees on WGS 84; radius and heights in meters by default; n the number of photos (36 by default). Headings in degrees clockwise from true north.

## Domain

Center between 89° S and 89° N. Radius above zero. Clockwise by default.

## Approximations

The waypoints are exact to the geodesic solution. The chord-to-arc factor is the plane one, which is right to parts per billion for a ring a drone flies. The pitch treats the ground between the drone and the target as flat and aims at one height on the subject's axis. Heights are carried through as given, with no terrain.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeographicLib for Python 2.1, Geodesic.WGS84 Direct and Inverse
- sourceEdition: geographiclib 2.1, run 2026-10-10
- sourceLocator: tools/vectors/gen_orbit.py: 15 orbits of 6 to 60 photos and 10 m to 1 km radius, at latitudes from 88.5° S to 88.5° N, on the equator, and across the antimeridian, clockwise and counterclockwise
- independent: yes
- inputs: for the first, a center at 40° N, 105° W, radius 50 m, flying at 80 m around a 60 m target, 12 photos
- outputs: gimbal pitch −21.80°; circumference 314.159 m; the first waypoint 50 m due north of the center, heading 180°; each waypoint checked is where GeographicLib puts it
- tolerance: 1e-9° in position, 1e-6° in heading, 1 µm in circumference
- verifiedBy: golden vectors v006 through v020, run by the core on every build
- verifiedOn: 2026-10-10

GeographicLib for Python is Karney's own implementation of his algorithm; the core uses a Rust port of it, so this checks the port and the tool's use of it against the original.

## Differential tests

- `tools/vectors/gen_orbit.py`: GeographicLib for Python at 15 orbits, with four waypoints of each checked for position and heading, and the circumference from its own chords; it needs the library installed and replaces only its own rows
- `tools/vectors/gen_drone.py`: the gimbal pitch worked again in Python at five cases
- `core/vectors/drone.mission.orbit.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `orbit_invariants`: at three centers, every waypoint is the radius from the center by the geodesic inverse, its heading is the way to the center, the waypoints are evenly spaced, counterclockwise is the same ring in reverse, and the circumference is the spacing times the count and 2πr
- `core/crates/gp-drone/tests/mission.rs` `gimbal_pitch_for_a_tower_top`: the spec scenario, a 60 m tower from 80 m at 50 m, pitches −21.8°, with 12 waypoints facing the center
