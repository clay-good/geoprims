<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Facade scan (`drone.mission.facade`)

## Method

A facade scan photographs one flat, vertical face of a structure from a fixed distance with the camera held level. The standoff takes the place of the flying height in the usual coverage equations, so the photo's footprint on the wall and the size of a pixel there follow from the sensor, the lens, and the standoff. The tool spreads photo stations evenly along the wall and up it so that neighbors overlap by at least the share asked for, places each station on the ground by the geodesic along the wall and then square to it, and orders them in level passes flown back and forth.

## Equations

- Footprint on the wall: width = sensor width × standoff ÷ focal length; height = sensor height × standoff ÷ focal length.
- GSD on the wall: sensor width ÷ image width × standoff ÷ focal length.
- Stations across a span (the wall's length, or top − bottom): 1, at the middle, when the span fits in one footprint; otherwise n = ⌈(span − footprint) ÷ (footprint × (1 − overlap))⌉ + 1, the first half a footprint in from one end, the last half a footprint in from the other, evenly spaced between.
- Position: the geodesic direct problem from the wall's first end along the wall to the station's distance, then from there at 90° to the wall (right or left) for the standoff (Karney 2013).
- Heading: that 90° bearing turned about, toward the wall.
- Path length: the sum over consecutive stations of √(ground distance² + height change²).

## Symbols and units

Wall ends in degrees on WGS 84. Standoff, heights, footprints, spacings, and lengths in meters by default; sensor size and focal length in millimeters; GSD in centimeters per pixel; overlaps in percent (75 along a pass and 60 between passes by default). Heights are above the wall's base.

## Domain

Both ends between 89° S and 89° N and at least 1 cm apart. Standoff, sensor size, and focal length above zero; the top above the bottom of the scan. No more than 10,000 photos (LIMIT_EXCEEDED).

## Approximations

The wall is one straight, flat, vertical face and the camera is level and square to it. The heading is the bearing from the wall turned about; the true bearing back differs by the convergence of the meridians over the standoff, under 0.003° for an 80 m standoff at 70° latitude. Even spacing rounds the count up, so the real overlap is the one asked for or a little more. Heights take no account of sloping ground.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: geographiclib 2.1 for Python, Geodesic.WGS84 Inverse and Direct
- sourceEdition: geographiclib 2.1, run 2026-10-10
- sourceLocator: tools/vectors/gen_facade.py, first case: a wall from 40.4406°, −80.002° running east for 50 m, stations along it by Direct, then 30 m to the right by Direct
- independent: yes
- inputs: wall 40.4406°, −80.002° to 40.440599998°, −80.001410673°; standoff 30 m; top 25 m; sensor 13.2 × 8.8 mm; focal length 8.8 mm; image width 5,472 px
- outputs: footprint 45 × 30 m; GSD 0.822 cm; 1 pass of 2 photos at 12.5 m, 5.000 m apart; stations 40.4403298°, −80.0017348° and 40.4403298°, −80.0016759°; wall length 49.99997 m
- tolerance: 1e-9° for each station (0.1 mm); 1e-7 m for lengths and spacings and 1e-6 m for the path, which differs by 1e-8 m between the core built for x86 and for ARM; 1e-7° for headings; exact for counts
- verifiedBy: golden vector v015, run by the core on every build
- verifiedOn: 2026-10-10

GeographicLib for Python places every station of 11 more scans, v016 through v026. The footprints, counts, and spacings in them are worked in the generator from the equations above, and the heading follows the same definition as the tool's, so the library checks where the stations are and how far apart, not the coverage arithmetic itself.

## Differential tests

- `tools/vectors/gen_facade.py`: every station's position and height, the wall length, and the path length for 12 walls of 12 to 400 m facing every way, from 55° S to 78° N, flown from either side, with raised scan bottoms and other overlaps (1 to 50 photos each); positions from GeographicLib for Python, which the core does not use
- `tools/vectors/gen_drone.py`: the footprint, GSD, and counts in closed form for ten walls on the equator
- `core/vectors/drone.mission.facade.jsonl`: those vectors and the refusals, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `facade_invariants`: for four walls from 45° S to 70° N every station, carried along its heading by the standoff, lands on the wall between its ends; the first and last photos sit half a footprint in from the ends and from the bottom and top; spacings are even and no wider than the overlap allows; passes alternate direction; and the other side of the wall gives the same stations twice the standoff away
- `core/crates/gp-drone/tests/mission.rs` `facade_gsd_uses_the_standoff`: the spec scenario, with the GSD from the standoff and the drone south of an east-running wall, facing north
