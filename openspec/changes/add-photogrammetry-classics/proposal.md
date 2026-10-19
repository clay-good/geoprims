## Why

The photogrammetry group computes everything a drone mission plan needs and nothing a photogrammetry course asks: photo scale, relief displacement, height from parallax, the base-to-height ratio and what it means for vertical precision. Students, instructors, and anyone checking a vendor's "expected vertical accuracy" claim work these by hand from Wolf, Dewitt & Wilkinson or the USACE manual, and the formulas are short, exact, and well published. They also explain why a mapping plan's overlap matters: the base-height ratio is where "80% overlap for better heights" comes from.

Depends on: `add-drone-suite` (camera geometry, GSD, overlap), `add-equipment-profiles` (camera profiles).

## What Changes

Four operations in `drone.photogrammetry`:

| Tool | Question | Source |
|---|---|---|
| `photo-scale` | What is the scale of a vertical photo at this flying height, and the flying height for a scale? | USACE EM 1110-1-1000 (1993) ch. 10 eq. 10-2, 10-3; Wolf, Dewitt & Wilkinson 4th ed. |
| `relief-displacement` | How far does a tall object's top shift from its base on the photo, and how tall is it from that shift? | EM 1110-1-1000 eq. 10-6 (d = r h / H) |
| `parallax-height` | How high is a point from the parallax difference between two photos? | EM 1110-1-1000 eq. 10-14, 10-15; Wolf et al. example 8-3 |
| `stereo-base` | What base-to-height ratio does this overlap give, and what vertical precision does a parallax measurement precision imply? | B = (1 − endlap) × ground coverage along track (Penn State GEOG 892 lesson 4); σ_h = (H / B)(H / f) σ_p, derived from h = H − fB / p and labeled derived |

Each draws its geometry: the camera, the ground, the object, and the rays.

## Capabilities

### New Capabilities

- `drone/classic-photogrammetry`: Photo scale, relief displacement, parallax heights, and base-height ratio with vertical precision.

### Modified Capabilities

None.

## Non-goals

- Bundle adjustment, orientation, or any processing of images. These are single-photo and stereo-pair formulas.
- Claiming a product accuracy from B/H. The tool states the ASPRS rule that accuracy is assessed with checkpoints, not predicted from GSD or geometry.

## Impact

- `core/gp-drone` grows by four operations.
- New sources-ledger rows: USACE EM 1110-1-1000 (31 March 1993) chapter 10; the Penn State lesson is already `psu-geog892`; `wolf-photogrammetry` exists.
- One explainer, "Why overlap changes heights," linking base-height ratio to the overlap presets.
