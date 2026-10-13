<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Heading to intercept a radial (`aviation.ifr.radial-intercept`)

## Method

Plane geometry around the station follows the Instrument Flying Handbook's course-interception method. The course on the wanted radial is the radial itself outbound, or its reciprocal inbound. The angle off course is the signed difference between the radial you are on and the one you want. The intercept heading is the course turned toward the wanted radial by the intercept angle. By default that angle follows the University of Dubuque instrument SOP: twice the angle off course, kept between 20° and 90°. You can set your own from 10° to 90°. The tool refuses a radial more than 90° away (you are on the far side of the station) and an inbound intercept too shallow to reach the radial before the station.

## Equations

- Radials wrapped to [0°, 360°). Angle off: off = ((now − want + 180) mod 360) − 180, with −180 taken as +180 (clockwise of the wanted radial is positive)
- Course: want + 180° inbound, want outbound, wrapped to [0°, 360°)
- Intercept angle: your own, or clamp(2 · |off|, 20°, 90°)
- Heading: inbound, course + sign(off) · angle; outbound, course − sign(off) · angle; wrapped to [0°, 360°). On the radial (off = 0) the heading is the course.
- Side: on the radial when off = 0; left of the course when (off > 0) equals inbound; otherwise right

## Symbols and units

Radials, course, heading, and angles in degrees (any angle unit is converted). The course and heading are in the same reference as the radials, normally magnetic. `direction` is inbound (default) or outbound.

## Domain

Any two radials no more than 90° apart (more is NO_SOLUTION). A given intercept angle from 10° to 90° (otherwise INVALID_INPUT). Inbound, the intercept angle must be larger than the angle off course, or the track meets the radial only past the station (NO_SOLUTION).

## Approximations

Geometry only, in a flat plane around the station. The heading has no wind correction, and the turn onto the course is not modeled. The default angle is one training technique among several; an assigned heading from ATC or the procedure comes first.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Flying Handbook
- sourceEdition: FAA-H-8083-15B
- sourceLocator: Chapter 9, Course Interception and Figure 9-16
- independent: yes
- inputs: present position on the 160 radial; intercept the 205 radial inbound, course 025°
- outputs: radial difference 45°; doubled intercept angle 90°; intercept heading 295°; track inbound on the 205 radial
- tolerance: 1e-9°
- verifiedBy: golden vector v010
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_dme.py`: picks the heading a different way. It places the aircraft 10 NM out on the current radial and keeps whichever of course ± angle has a track that actually crosses the wanted radial's ray (not its extension past the station). Sixteen cases cover inbound and outbound, both sides, north wrapping, given and default angles, and being on the radial; four invalid or unreachable cases are refused (exact, within 1e-9°)
- `core/vectors/aviation.ifr.radial-intercept.jsonl`: those 20 vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `radial_intercept_invariants`: adding full turns to either radial changes nothing; default intercept angles double the radial difference within the published 20° to 90° bounds; mirroring the current radial across the desired radial mirrors the intercept heading and side; an explicit angle overrides the default; and a point already on the radial stays on its course
