<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Survey grid (`drone.mission.survey-grid`)

## Method

The area is projected onto a local transverse Mercator plane centered on it. Parallel flight lines are swept across it at the line spacing: ⌈width / spacing⌉ + 1 of them, centered so the outer lines reach the edges, as published flight planning counts them. The direction is the one with the fewest lines unless given. Each line is cut to its strip of the area, and at buffered no-fly holes. The lines are joined in serpentine order, and a transit that would cross a hole follows the hole's buffered boundary. Photo points fall at the photo spacing along each line, with the extra photos past each end. An optional crosshatch adds a second, perpendicular set. The waypoints are projected back to latitude and longitude.

## Equations

- Lines = ⌈W / S⌉ + 1, centered on the area's width W across the flight direction
- Photos per line = ⌈L / B⌉ + 1 + 2e, with L the line's length, B the photo spacing, and e the extra photos per end
- Path length = the sum of the line and transit lengths; flight time = path length ÷ groundspeed

## Symbols and units

S line spacing and B photo spacing (air base), in meters or feet; W and L on the local plane in the same unit; e a whole number from 0 to 10.

## Domain

A simple polygon of at least three corners, positive spacings of at least 0.5 m, and holes inside the area. More than 2,000 photo points are counted in full, but only the first 2,000 are drawn, with OUTPUT_TRUNCATED.

## Approximations

The sweep is on a local plane; line spacing stays true to within 1e-7 across a few kilometers, checked geodesically. The ground is taken as flat and the height constant. Flight time ignores turns, climbs, and wind.

## Worked example

- sourcePublisher: Penn State College of Earth and Mineral Sciences (Q. Abdullah)
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems, "Designing a Flight Route"
- sourceEdition: online course text, retrieved 2026-09-23
- sourceLocator: https://courses.ems.psu.edu/geog892/node/658 (Flight Lines Computations; Number of Image Computations)
- independent: yes
- inputs: a block 20 mi east-west by 13 mi north-south, line spacing 8,400 ft, air base 2,800 ft
- outputs: 10 flight lines of 43 images, 430 images (390 without the 4 extra images per line)
- tolerance: exact (whole counts)
- verifiedBy: golden vectors v011 and v012, run by the core on every build
- verifiedOn: 2026-09-24

The King Saud University SE 321 example (60 by 40 km, 920 m lines, 460 m air base: 45 lines, 6,120 photos) is vector v013.

## Differential tests

- `tools/vectors/gen_image_count.py`: the published counts evaluated independently on geographiclib arc lengths, pinned only where every length a line can take gives the same ceiling with a 1e-4 margin, for the published blocks, the Penn State block flown north-south, other extra-photo counts, 8 random rectangles worldwide, and the refusals (vectors v006 to v028)
- `core/vectors/drone.mission.survey-grid.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `image_count_invariants`: on plane rectangles the lines are ⌈W / S⌉ + 1 and the photos match the published count, the grid and the image count agree, closer spacing never means fewer lines or photos, and overshoot changes the path but not the photos; `line_spacing_is_true_geodesically`, `auto_direction_minimizes_lines`, `polygon_with_a_hole`, and `image_count_matches_the_pattern` (20 random polygons) cover the rest
