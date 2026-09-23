<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Image count and survey size (`drone.photogrammetry.image-count`)

## Method

The published flight-planning count, applied to the same sweep the survey-grid tool flies. Penn State GEOG 892 ("Designing a Flight Route") puts the first flight line on one boundary of the block and counts lines as width / line spacing + 1, rounded up, so the lines reach both edges. It counts images per line as length / air base + 1, rounded up, and then adds two images at each end of every line. King Saud University SE 321 ("Design of Photogrammetric Flight Plan") uses the same two counts and the same two extra photos at each end.

The tool plans on a transverse Mercator plane centered on the area. Lines run along the direction that gives the fewest of them (or the azimuth you give). The ⌈width / spacing⌉ + 1 lines are centered on the area's extent, so the outer two lie on the edges or an equal distance past them. Each line covers its strip, half a spacing to either side, and runs across the area's full extent within that strip. A line on or past an edge therefore flies the length of that edge. Along each line, ⌈length / photo spacing⌉ + 1 photos are spaced evenly from end to end, so no gap is wider than the photo spacing. The extra photos per end (2 by default, 0 to 10) continue at that step past each outer end, and the line is flown far enough to take them, or as far as the overshoot if that is longer.

## Equations

- Lines: N_L = ⌈W / S⌉ + 1, spanning (N_L − 1) × S ≥ W, centered on the width W.
- Photos on a line of length L: n = ⌈L / B⌉ + 1 + 2e, where e is the number of extra photos per end and B is the photo spacing (air base).
- Step along the line: B′ = L / ⌈L / B⌉ ≤ B (B′ = B when L / B is whole).
- Line length flown past each outer end: max(overshoot, e × B′).
- Total photos: the sum of n over the lines. On a rectangle this is N_L × (⌈L / B⌉ + 1 + 2e).
- Survey length: the sum of the line lengths over the area. Path length adds the ends flown past the area and the turns between lines.
- Ceilings allow a slack of 10⁻⁴ of a spacing, so millimeters of round-off on the plane do not add a line or a photo.

## Symbols and units

W is the area's width across the lines and L is a line's length over the area, both in meters on the plane. S is the line spacing and B the photo spacing, in any length unit (m, ft). e is extra photos per line end (a whole number from 0 to 10). Outputs are photos and lines (counts), survey and path length (km), area (ha), and flight time (min) when a groundspeed is given.

## Domain

An area of at least 3 corners between 85° S and 85° N, within 50 km of its center. Line spacing of at least 0.5 m and photo spacing of at least 0.1 m. No more than 5,000 lines. Overshoot of up to 10 km. Holes (ring 1, 2, and so on) cut the lines, and each piece of a line takes its own photos, but only the survey grid routes the turns around them.

## Approximations

Flat ground and a constant photo spacing. Lines run straight on a local transverse Mercator plane, whose scale error is under 10⁻⁶ within a few tens of kilometers of the center. Photos a camera takes during turns are not counted.

The University of Washington CEE 424 notes (Flight Planning sheet 1, problem 6, a Wolf-style problem) place the outer lines a quarter of a photo width inside the boundaries and count photos per strip as length / B + 4, without the + 1. For its 34.8 by 22.8 km block (G = 3,450 m, S = 2,587.5 m, B = 1,380 m), the sheet gets 13 lines of 21 photos, 273 in all. The tool, with lines on the edges, gets 15 lines of 22, 330 in all. This tool follows the Penn State and King Saud method, which the two published block examples below reproduce exactly. A flight app with its own margins can differ from this count by a line or a few photos.

## Worked example

- sourcePublisher: Penn State College of Earth and Mineral Sciences (Q. Abdullah)
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems, "Designing a Flight Route"
- sourceEdition: online course text, CC BY-NC-SA 4.0, retrieved 2026-09-23
- sourceLocator: https://courses.ems.psu.edu/geog892/node/658 (Flight Lines Computations; Number of Image Computations)
- independent: yes
- inputs: a block 20 mi east-west by 13 mi north-south, line spacing 8,400 ft, air base 2,800 ft, lines east-west, first line on the southern boundary
- outputs: (13 × 5,280 / 8,400) + 1 = 9.171, so 10 flight lines; (105,600 / 2,800) + 1 + 4 = 42.7, so 43 images per line; 430 images. Without the 4 extra images per line, 39 per line and 390 in all.
- tolerance: exact (whole counts)
- verifiedBy: golden vectors v011, v012, and v013, run by the core on every build
- verifiedOn: 2026-09-23

A second published example, King Saud University SE 321, "Design of Photogrammetric Flight Plan" (CLO3 example, https://faculty.ksu.edu.sa/sites/default/files/se_321_design_of_photogrammetric_flight_plan_example.pdf), maps 60 km east-west by 40 km north-south with B = 460 m and 920 m between lines. It gets (60,000 / 460) + 1 = 131.4, so 132 photos per strip, plus two on each side for 136. It gets (40,000 / 920) + 1 = 44.4, so 45 lines, and 6,120 photos in all. These are vectors v014 (6,120) and v015 (5,940 with no extra photos).

## Differential tests

- `tools/vectors/gen_image_count.py`: a separate Python evaluation of the published counts. It works from each block's geodesic width and length (geographiclib: the meridian arc and the parallel arcs at both edges). It pins a count only when every length a line can take gives the same ceiling, with a margin of 10⁻⁴. It produces vectors v006 to v026 of this tool and v006 to v013 of the survey grid.
- `core/vectors/drone.photogrammetry.image-count.jsonl`: 26 vectors, including both published examples, run through the core on every build. v001 to v005 pinned the version 1.0.0 line counts and are superseded by v006 to v010.
- `core/crates/gp-drone/tests/mission.rs` `image_count_matches_the_pattern`: the estimate agrees with the survey-grid pattern on 20 random polygons, as the spec requires.

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `image_count_invariants`: on plane rectangles, the lines are ⌈W / S⌉ + 1 and the photos N_L × (⌈L / B⌉ + 1 + 2e) for e = 0 to 3. Every line crosses the full length. The survey grid gives the same lines and photos and one trigger point per photo. The default equals 2 extra photos per end. Closer lines or photos never mean fewer of them. An overshoot never changes the photo count, adds nothing to the path while it is shorter than the extra photos, and lengthens the path once it is longer.
