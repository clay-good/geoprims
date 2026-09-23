<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Overlap, trigger interval, and line spacing (`drone.photogrammetry.trigger`)

## Method

Classical flight-planning geometry for a vertical camera over flat ground. One photo covers a ground rectangle whose sides are the sensor sides scaled by height over focal length (the same footprint the GSD tool reports, GSD × pixels). Photos along a line are spaced so that consecutive footprints share the front overlap, the air base; flight lines are spaced so that adjacent strips share the side overlap. The trigger interval is the air base over the groundspeed. In landscape orientation the sensor width (the long side, `image_width` pixels) lies across the track, as in Penn State GEOG 892, where the longer dimension of the array is perpendicular to the flight direction; portrait swaps the two.

## Equations

- Footprint across = Sw × H / f, footprint along = Sh × H / f (landscape; portrait swaps them).
- Trigger distance (air base) B = footprint along × (1 − front overlap).
- Line spacing SP = footprint across × (1 − side overlap).
- Trigger interval = B / groundspeed.
- Fastest groundspeed the camera allows = B / camera minimum interval; `TRIGGER_TOO_FAST` when the interval is under that minimum.
- Presets (Pix4D vendor guidance, not a standard): general 75/60, forest 85/85; a given overlap overrides its preset value.

## Symbols and units

Sw, Sh sensor width and height (mm), f physical focal length (mm), H height above ground (m or ft), overlaps in percent (0 to 99), groundspeed (m/s, kn, mph, km/h). Distances are returned in meters and the interval in seconds.

## Domain

Positive height, groundspeed, sensor size, and focal length; overlaps from 0% to 99%; a positive camera minimum interval when given. A 35 mm-equivalent focal length is converted with the crop factor, as in the GSD tool.

## Approximations

Nadir camera, flat ground at the stated height above ground, no lens distortion, a constant groundspeed along the line, and no wind drift or crab. Ground that rises toward the camera shrinks the footprint and cuts the real overlap (the terrain overlap tool). The spacing is not adjusted to fit a whole number of lines over an area.

## Worked example

- sourcePublisher: Penn State College of Earth and Mineral Sciences (Q. Abdullah)
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems, "Designing a Flight Route"
- sourceEdition: online course text, CC BY-NC-SA 4.0, retrieved 2026-09-23
- sourceLocator: https://courses.ems.psu.edu/geog892/node/658 (Flight Lines Computations, Number of Image Computations, Aircraft Speed and Image Collection)
- independent: yes
- inputs: 12,000 × 7,000 px array of 10 µm pixels (120 × 70 mm), long side across the track, f = 100 mm, 1 ft GSD at 10,000 ft, 60% end lap, 30% side lap, 150 knots
- outputs: coverage 12,000 × 7,000 ft, line spacing 8,400 ft, air base 2,800 ft, 11.067 s between exposures (the core gives 2,560.32 m = 8,400 ft, 853.44 m = 2,800 ft, and 11.060 s)
- tolerance: 0.01 m on the distances; 0.01 s on the interval, since the course converts 150 knots with 1.15 mph per knot (253.0 ft/s) where the core uses the exact 1,852 m per nautical mile (253.2 ft/s)
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-09-23

A second published example, University of Washington CEE 424 Flight Planning sheet 1, problems 6 and 7 (f = 305 mm, 230 mm format at 1:15,000, 60/25 overlap, 260 km/h), gives an air base of 1,380 m, strips 2,587.5 m apart before the course adjusts them to fit the area, and 19.12 s between exposures; it is vector v007 (the core gives 19.108 s; the sheet's own arithmetic, 1.38 / 260 × 3,600, is 19.108).

## Differential tests

- `tools/vectors/gen_photogrammetry_overlap.py`: a separate Python evaluation of the footprint, air base, spacing, and interval relations for vectors v008 to v022 (portrait, presets and overrides, the camera check, 0% and 99% overlap, feet, miles per hour, knots, and a 35 mm-equivalent focal length), and the error cases
- `tools/vectors/gen_drone.py`: the original five vectors v001 to v005
- `core/vectors/drone.photogrammetry.trigger.jsonl`: all 22 vectors, including the two published examples, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `trigger_invariants`: the footprints equal the GSD tool's at the same height, trigger distance and line spacing are the footprints times (1 − overlap), everything scales linearly with height and the interval inversely with groundspeed, the interval times the groundspeed is the trigger distance, portrait swaps the two footprints, more overlap always means closer photos and lines, and the camera warning fires exactly when the interval is under the camera minimum, with the fastest speed times that minimum equal to the trigger distance
