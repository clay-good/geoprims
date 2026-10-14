<!-- Derivation note (trust/correctness-program layer A). -->
# Sensor crop factor (`drone.photogrammetry.crop-factor`)

## Method

The crop factor compares a sensor's diagonal with the 43.267 mm diagonal of a 36 × 24 mm frame. Multiplying a physical focal length by it gives the focal length that frames the same view on full frame. The operation shares its constant and its function with the camera model every photogrammetry tool here uses.

## Equations

- Sensor diagonal d = √(s_w² + s_h²)
- Crop factor = 43.2666 mm ÷ d
- 35 mm-equivalent focal length = f × crop factor; f = equivalent ÷ crop factor

## Symbols and units

s_w, s_h sensor width and height in millimeters; f physical focal length. The crop factor is unitless.

## Domain

Positive sensor width and height. Give neither focal length, or one of them, not both.

## Approximations

The comparison is by diagonal, so the equivalent focal length matches the diagonal field of view exactly and the width and height only for a 3:2 sensor.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/photogrammetry, requirement "GSD and footprint" (35 mm-equivalent focal lengths); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: sensor 13.2 × 8.8 mm, f 8.8 mm
- outputs: diagonal 15.864 mm, crop factor 2.727, 24.0 mm equivalent
- tolerance: 0.01 mm
- verifiedBy: golden vector v001 and the `focused_photogrammetry_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `crop_vectors`: 18 sensors from 1/2.3-inch to medium format, with a physical focal length, an equivalent one, or neither, by a separate Python evaluation.
- `core/vectors/drone.photogrammetry.crop-factor.jsonl`: those vectors, the full-frame identity, and two refusal cases.
- The integration suite requires `drone.photogrammetry.gsd` to derive the same physical focal length from the equivalent one.

## Invariants

- A 36 × 24 mm sensor has a crop factor of exactly 1.
- Physical to equivalent and back returns the focal length.
- A smaller sensor has a larger crop factor.
