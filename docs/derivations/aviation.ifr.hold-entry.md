<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Holding pattern entry (`aviation.ifr.hold-entry`)

## Method

The FAA's three recommended entries, from AIM 5-3-8j and FIG 5-3-4. Two lines through the holding fix split the approach into sectors: the holding course, and the 70° line, drawn on the holding side at 70° to the outbound course. The sector you arrive from picks the entry: parallel from sector (a), 110° wide on the holding side beyond the fix; teardrop from sector (b), 70° wide on the non-holding side beyond the fix; and direct from sector (c), the 180° half on the pattern's side of the 70° line. A left-turn hold is the mirror image: its sectors are oriented on the 70° line on its own holding side.

The tool reads the sector from the heading you arrive on. It measures that heading against the inbound course, positive toward the holding side, and looks up the sector. It also gives the headings to fly: the outbound course for a direct or parallel entry, and 30° off the outbound course toward the holding side for a teardrop.

ICAO PANS-OPS Doc 8168 Volume I and Transport Canada AIM RAC 10.5 allow a 5° zone of flexibility on each side of a sector boundary. The FAA AIM gives none. Within 5° of a boundary, the tool names the neighboring entry as also acceptable.

## Equations

- Relative angle r = heading − inbound course, wrapped to (−180°, 180°].
- Toward the holding side: r′ = r for right turns, −r for left turns, with −180° taken as +180°.
- Direct if −70° ≤ r′ ≤ 110°; teardrop if 110° < r′ ≤ 180°; parallel if −180° < r′ < −70°.
- Outbound course = inbound + 180°. Teardrop heading = outbound − 30° for right turns, outbound + 30° for left turns. Parallel heading = outbound course.
- Also acceptable: the entry across the nearest boundary (r′ = −70°, 110°, or 180°) when r′ is within 5° of it.

## Symbols and units

`inbound_course` is the holding course to the fix, `heading` your heading as you reach the fix, both in degrees (any angle unit is converted), and `turns` is right (standard) or left. Out come the `entry`, an optional `alternative`, the `relative_angle`, the `outbound_course`, the `teardrop_heading`, the `parallel_heading`, and an `explanation` in words.

## Domain

Any inbound course and heading. Angles wrap, so 360° and 0° are the same course. On a boundary itself the tool takes the direct entry at the ±70° and 110° lines and the teardrop on the reciprocal of the inbound course, the same for either turn direction, and names the other entry as also acceptable.

## Approximations

The FAA defines the sectors by where you come from. ICAO and Transport Canada define them by your heading. The two agree when heading and track agree. In a strong crosswind they differ by the wind correction angle, which matters only near a boundary, where the 5° zone of flexibility covers most of it. The entry does not depend on speed, altitude, or leg length. At a VOR intersection or DME fix, ICAO limits the entry to the radials or arc that form the fix, and the tool does not model that. Time on the outbound leg of an entry is the one minute the AIM gives; the leg timing of the hold itself (1 minute, or 1½ above 14,000 ft) belongs to `aviation.ifr.hold-wind-timing`.

The first release had one defect, which this version fixes. A left-turn hold flown exactly on the reciprocal of the inbound course got the parallel entry, while the mirror-image right-turn hold got the teardrop. Both entries are acceptable there, but the tool should give the same answer to the same picture. Both now get the teardrop, with the parallel also acceptable (vectors v031 and v032).

## Worked example

- sourcePublisher: Transport Canada
- sourceTitle: Transport Canada Aeronautical Information Manual (TC AIM), TP 14371E
- sourceEdition: AIM 2026-1, effective March 19, 2026
- sourceLocator: RAC 10.2, example 2: after a missed approach on a track of 234° to the ZHZ NDB, the pilot is to "make a right turn and hold" on an inbound track of 234°. That is the sector 3 (direct) procedure of RAC 10.5.
- independent: yes
- inputs: inbound course 234°, right turns, heading 234°
- outputs: direct entry, outbound course 054°, relative angle 0°
- tolerance: exact
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-23

The sector geometry itself was checked against AIM 5-3-8j FIG 5-3-4 and the IFH (FAA-H-8083-15B) Figure 10-6: the 70° line, and sectors of 110°, 70°, and 180°. The 5° zone of flexibility was checked against TC AIM RAC 10.5 and a published extract of PANS-OPS Volume I.

## Differential tests

- `tools/vectors/gen_hold_part107.py`: works the sector from the figure by position rather than by the tool's angle rule. It puts the fix at the origin, finds the bearing you come from (heading + 180°), and asks which side of the holding course and of the 70° line that bearing lies on. It covers 14 arrivals inside the sectors for both turn directions and four inbound courses, eight arrivals 3° from a boundary with the other entry also acceptable, the reciprocal of the inbound course for both turn directions, a heading in radians, and two missing-input errors.
- `core/vectors/aviation.ifr.hold-entry.jsonl`: those 27 vectors, the TC AIM example (v022), and the 7 hand-checked first-release vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `hold_entry_invariants`: at 13 inbound courses and every whole-degree arrival angle, a left hold mirrors a right hold exactly, boundaries and alternatives included. Turning the whole picture by 100° does not change the entry. The sectors come out 181, 70, and 109 whole degrees wide, which is 180°, 70°, and 110° with the boundaries assigned. A second entry is offered exactly when the angle is within 5° of a boundary, and it is always a neighboring sector. The headings are the outbound course and 30° off it toward the holding side. The test counts the 4,680 cases it reached, so it cannot pass empty.
