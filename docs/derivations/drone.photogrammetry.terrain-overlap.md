<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Overlap over high terrain (`drone.photogrammetry.terrain-overlap`)

## Method

A mission planned at height h above the takeoff point fixes its photo spacing and line spacing from the flat-ground footprint at h. Over ground t above the takeoff point the camera is only h − t up, so each footprint shrinks by (h − t)/h while the spacing stays the same, and the overlap there drops. This is the endlap-at-highest-relief relation of Pryor (1959), with the takeoff level as his datum plane, and the terrain effect in Wolf, Dewitt, and Wilkinson, ch. 18. The tool also inverts it: the height above takeoff at which a plan (spacing replanned at that height) still keeps the lowest overlap you accept over the highest ground.

## Equations

- Height over the highest ground: h − t.
- Overlap there: o_t = 1 − (1 − o) × h / (h − t), for front and side overlap alike.
- Pryor's form, with E1 the endlap at the datum and E2 + 50 the endlap at the highest relief: E1 = (E2 + 50) + (50 − E2) × t / h, which rearranges to the line above.
- GSD there = pixel pitch × (h − t) / f; GSD at takeoff level = pixel pitch × h / f.
- Height that holds o_min: H ≥ t × (1 − o_min) / (o − o_min), taking the larger over front and side; reported only when o_min is below every planned overlap and t > 0.

## Symbols and units

h flight height above takeoff (m or ft), t highest ground above takeoff (negative for a valley), o planned overlap and o_min lowest accepted overlap (percent), f focal length (mm). Heights are returned in meters, overlaps in percent, and GSD in centimeters per pixel.

## Domain

h > 0 and t < h (the ground must stay below the drone). Overlaps from 0% to 99%. A result below zero means gaps between photos or lines.

## Approximations

Vertical camera, the stated highest point only (not a terrain profile), and a flat plan at h. Trees and buildings on the high ground bring the imaged surface closer still. Pryor places the highest point where its relief displacement costs the most overlap; the relation is the same one used here.

## Worked example

- sourcePublisher: Highway Research Board (W. T. Pryor, Bureau of Public Roads)
- sourceTitle: Relationship of Topographic Relief, Flight Height, and Minimum and Maximum Overlap, Highway Research Board Bulletin 228
- sourceEdition: Bulletin 228, 1959
- sourceLocator: p. 31 (the 20,000 ft example) and pp. 36-37 (the endlap equation and the 2/9 relief-to-height ratio); https://onlinepubs.trb.org/Onlinepubs/hrbbulletin/228/228-005.pdf
- independent: yes
- inputs: flight height 20,000 ft above the datum (the lowest ground), 65% endlap there, 4,444 ft of relief, lowest accepted endlap 55%
- outputs: 55% endlap at the point of highest relief, and 20,000 ft as the height that holds 55% over 4,444 ft (the core gives 55.001% and 6,095.4 m = 19,998 ft)
- tolerance: 0.01 percentage points on the overlap and 1 m on the height, since the bulletin rounds the relief (2/9 × 20,000 = 4,444.4 ft) to the foot
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-23

The bulletin's second example, p. 32 (1,300 ft flight height, 65% maximum endlap, 2/9 of 1,300 = 289 ft of relief), is vector v024.

## Differential tests

- `tools/vectors/gen_photogrammetry_overlap.py`: a separate Python evaluation of the relation for the replacement vectors v015 to v022 and the new vectors v025 to v029 (feet, a target above the plan, a valley with GSD, and the error cases)
- `tools/vectors/gen_drone.py`: the original vectors v001 to v014
- `core/vectors/drone.photogrammetry.terrain-overlap.jsonl`: all 21 current vectors, including both published examples, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `terrain_overlap_invariants`: flat ground returns the planned overlap and no warning; overlap falls strictly as the ground rises (and rises over a valley), with the warning exactly when the ground is above takeoff; the result equals one minus the trigger tool's spacing at h over the GSD tool's footprint at h − t, for front and side; scaling h and t together leaves the overlap unchanged; the GSD over the high ground is the takeoff GSD times (h − t)/h; and flying at the reported height gives exactly the lowest accepted overlap over the same ground
