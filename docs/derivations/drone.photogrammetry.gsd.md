<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Ground sampling distance (`drone.photogrammetry.gsd`)

## Method

Pinhole-camera similar triangles for a nadir image over flat ground. The sensor width over the focal length equals the footprint width over the height, and dividing by the pixel count gives the ground size of one pixel. A 35 mm-equivalent focal length is converted to the physical one with the crop factor (given, or the full-frame diagonal over the sensor diagonal). A focal length over 1.5 × the sensor diagonal is flagged as a likely equivalent value.

## Equations

- GSD across = Sw × H / (f × W).
- Footprint across = Sw × H / f = GSD across × W.
- GSD along = Sh × H / (f × Hpx), footprint along = Sh × H / f.
- Equivalent focal length: f = f35 / crop, crop = 43.27 mm / √(Sw² + Sh²) when not given.

## Symbols and units

Sw, Sh sensor width and height (mm), f physical focal length (mm), W, Hpx image width and height (px), H height above ground (m). GSD in cm per pixel by default, and footprints in meters.

## Domain

Positive sensor size, focal length, pixel counts, and height. Along-track values need the sensor height and image height.

## Approximations

Nadir view, flat ground at the stated height above ground, and no lens distortion. Terrain relief and camera tilt change the real GSD across the image. The height is height above ground, not above the takeoff point.

## Worked example

- sourcePublisher: Alshaibani, Helvaci, Shayea, Saad, Azizan, and Yakub
- sourceTitle: Airplane Type Identification Based on Mask RCNN and Drone Images (arXiv:2108.12811)
- sourceEdition: arXiv v1, 2021
- sourceLocator: Equation 1 and figure 5 (1-inch sensor 12.75 mm wide, 10.6 mm lens, 4,608 px, 120 m gives 3.13 cm/px)
- independent: yes
- inputs: sensor 12.75 × 8.5 mm, focal length 10.6 mm, image width 4,608 px, height 120 m
- outputs: GSD 3.13 cm/px (the core gives 3.1324 cm/px)
- tolerance: 0.005 cm (the printed rounding)
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_drone.py`: a separate Python implementation of the photogrammetric relations (Wolf, Dewitt, and Wilkinson, 4th edition, chapter 6) at 20 camera and height pairs, from 1/2.3-inch to full-frame sensors and 15 m to 500 m (within 1e-12 relative)
- `core/vectors/drone.photogrammetry.gsd.jsonl`: those vectors, the equivalent-focal-length warning case, and the published example, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `gsd_invariants`: GSD is linear in height, the footprint equals GSD × pixels in both directions, and the height-for-GSD tool inverts it exactly
