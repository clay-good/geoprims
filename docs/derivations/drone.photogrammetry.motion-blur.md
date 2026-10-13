<!-- Derivation note (trust/correctness-program layer A). -->
# Motion blur (`drone.photogrammetry.motion-blur`)

## Method

During an exposure, a camera moving over the ground travels a distance equal to its groundspeed times the exposure time. Dividing that distance by the ground sampling distance gives the image smear in pixels. Solving the same equation for exposure gives the slowest shutter that stays within the chosen blur limit.

## Equations

- Ground travel during exposure: d = v × t
- Motion blur: b = d ÷ GSD = v × t ÷ GSD
- Maximum exposure: t_max = b_limit × GSD ÷ v

## Symbols and units

v is groundspeed in m/s, t is exposure time in seconds, GSD is ground distance per pixel in meters, and b is blur in pixels. The default blur limit is 0.5 px, an explicit GeoPrims product choice rather than a camera or photogrammetry standard.

## Domain

Groundspeed is positive or zero; exposure and GSD are positive; the blur limit is 0.01 to 10 px. At zero groundspeed there is no finite shutter limit, so the tool returns `NO_SOLUTION` instead of infinity.

## Approximations

The equation is exact for straight, level ground motion during the exposure. It does not model vibration, gimbal movement, turns, gusts, rolling-shutter distortion, or a GSD that changes with terrain. The displayed shutter fraction is rounded to a whole denominator, so a camera setting should use the next faster available shutter.

## Worked example

- sourcePublisher: GeoPrims
- sourceTitle: add-drone-suite photogrammetry specification
- sourceEdition: repository specification, accessed 2026-10-13
- sourceLocator: motion-blur scenario
- independent: no; this scenario verifies the product requirement but does not satisfy the independent-example promotion gate
- inputs: groundspeed 10 m/s, exposure 1/1,000 s, GSD 2.741 cm, default blur limit 0.5 px
- outputs: 0.3648 px of blur; maximum exposure 0.0013705 s, about 1/730 s
- tolerance: 0.001 px and 0.000001 s
- verifiedBy: `core/crates/gp-drone/tests/drone.rs` `blur_at_one_thousandth`
- verifiedOn: 2026-10-13

## Differential tests

None yet. The tool remains experimental until an independent published worked example and the full stable-bar vector count are added.

## Invariants

The equation is linear in groundspeed and exposure, inverse in GSD, and the maximum exposure is linear in both GSD and the chosen blur limit. These properties are candidates for the promotion test set.
