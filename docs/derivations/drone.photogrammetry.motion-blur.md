<!-- Derivation note (trust/correctness-program layer A). -->
# Motion blur (`drone.photogrammetry.motion-blur`)

## Method

During an exposure, a camera moving over the ground travels a distance equal to its groundspeed times the exposure time. Dividing that distance by the ground sampling distance gives the image smear in pixels. Solving the same equation for exposure gives the slowest shutter that stays within the chosen blur limit.

## Equations

- Ground travel during exposure: d = v × t
- Motion blur: b = d ÷ GSD = v × t ÷ GSD
- Maximum exposure: t_max = b_limit × GSD ÷ v

## Symbols and units

v is groundspeed in m/s, t is exposure time in seconds, GSD is ground distance per pixel in meters, and b is blur in pixels. The default blur limit is 0.5 px. Grenzdörffer, Niemeyer, and Schmidt use the same limit as their criterion for a sharp UAS image.

## Domain

Groundspeed is positive or zero; exposure and GSD are positive; the blur limit is 0.01 to 10 px. At zero groundspeed there is no finite shutter limit, so the tool returns `NO_SOLUTION` instead of infinity.

## Approximations

The equation is exact for straight, level ground motion during the exposure. It does not model vibration, gimbal movement, turns, gusts, rolling-shutter distortion, or a GSD that changes with terrain. The displayed shutter fraction is rounded to a whole denominator, so a camera setting should use the next faster available shutter.

## Worked example

- sourcePublisher: MDPI, Drones
- sourceTitle: Enhancing Direct Georeferencing Using Real-Time Kinematic UAVs and Structure from Motion-Based Photogrammetry for Large-Scale Infrastructure
- sourceEdition: volume 8, issue 12, article 736 (2024)
- sourceLocator: section 2.1.1, equation (1), and the worked example following it
- independent: yes
- inputs: groundspeed 10 m/s, exposure 1/500 s, GSD 2.0 cm
- outputs: 1 px of blur; applying the separately published 0.5 px limit gives a maximum exposure of 0.001 s, or 1/1,000 s
- tolerance: exact to 0.000000000001 px and 0.000000000001 s
- verifiedBy: `core/vectors/drone.photogrammetry.motion-blur.jsonl` `v006`
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` evaluates Han and Han's equation independently in Python over 22 cases spanning 1–30 m/s, 0.3–10 cm GSD, 1/100–1/8,000 s exposures, and 0.25–2 px limits. `core/vectors/drone.photogrammetry.motion-blur.jsonl` holds the results, including the published example and the GeoPrims scenario.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `motion_blur_invariants` verifies that blur is linear in groundspeed and exposure and inverse in GSD, while maximum exposure is linear in GSD and the selected limit and inverse in groundspeed.
