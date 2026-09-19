<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Height for a target GSD (`drone.photogrammetry.altitude-for-gsd`)

## Method

The GSD relation solved for height: for a nadir camera over flat ground, the height above ground that gives the target pixel size. The resulting height is compared with the user's altitude ceiling (for example, the 400 ft Part 107 limit) and flagged if above it.

## Equations

- H = GSD × f × W / Sw.
- Footprint across = GSD × W.
- Equivalent focal length: f = f35 / crop, as in the GSD tool.

## Symbols and units

GSD target ground sampling distance (cm per pixel at the interface), Sw sensor width (mm), f physical focal length (mm), W image width (px), H height above ground (m).

## Domain

Positive target GSD, sensor size, focal length, and pixel count. An optional ceiling adds the ABOVE_ALTITUDE_CEILING warning when H exceeds it.

## Approximations

Same as the GSD tool: nadir view, flat ground, no lens distortion. The height is above ground at the image, so hilly sites need the height over the highest ground for the GSD to hold everywhere.

## Worked example

- sourcePublisher: Alshaibani, Helvaci, Shayea, Saad, Azizan, and Yakub
- sourceTitle: Airplane Type Identification Based on Mask RCNN and Drone Images (arXiv:2108.12811)
- sourceEdition: arXiv v1, 2021
- sourceLocator: Equation 1 and figure 5, inverted (a 3.13 cm/px GSD with the 12.75 mm, 10.6 mm, 4,608 px camera came from 120 m)
- independent: yes
- inputs: target GSD 3.13 cm, sensor width 12.75 mm, focal length 10.6 mm, image width 4,608 px
- outputs: about 120 m (the core gives 119.91 m; the published GSD is rounded to 0.01 cm)
- tolerance: 0.2 m (the effect of the GSD's printed rounding)
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_drone.py`: a separate Python implementation at 20 camera and GSD pairs, from 0.37 cm to 6.26 cm (within 1e-12 relative)
- `core/vectors/drone.photogrammetry.altitude-for-gsd.jsonl`: those vectors and the published example, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `gsd_invariants`: the height for the GSD computed at height H is H again (within 1e-9 relative), for four sensor sizes and five heights
