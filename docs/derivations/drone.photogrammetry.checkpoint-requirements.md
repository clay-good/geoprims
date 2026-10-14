<!-- Derivation note (trust/correctness-program layer A). -->
# ASPRS checkpoint requirements (`drone.photogrammetry.checkpoint-requirements`)

## Method

The ASPRS Positional Accuracy Standards, Edition 2, recommend a number of checkpoints by project area in Table C.1, counted separately for open ground (non-vegetated accuracy) and for vegetated ground (vegetated accuracy). Section 7.12 asks that checkpoints be at least twice as accurate as the product they test. The operation applies both to a stated area. It shares its table and counting with `drone.photogrammetry.gcp-plan`, which measures the area from a boundary and lays the points out.

## Equations

- Area of each land cover = project area × its share, rounded to whole km².
- Checkpoints for that area: 30 up to 500 km², then 5 more for each further 250 km² or part, to 70 at 2,500 km².
- Total = open-ground count + vegetated count (when any of the area is vegetated).
- Checkpoint RMSE ≤ ½ × the product's RMSE class, horizontally and vertically.

## Symbols and units

The project area in any area unit; the vegetated share in percent; accuracy classes and checkpoint accuracies in centimeters or any length unit.

## Domain

A positive area with no more than 2,500 km² of either land cover, where Table C.1 ends and the standard leaves the count to the client. Accuracy classes, when given, must be positive.

## Approximations

None: the counts and the ratio are the standard's. The standard reads the table by area band, and this follows the band the rounded area falls in.

## Worked example

- sourcePublisher: American Society for Photogrammetry and Remote Sensing
- sourceTitle: ASPRS Positional Accuracy Standards for Digital Geospatial Data
- sourceEdition: Edition 2, Version 1.0 (February 2023)
- sourceLocator: Annex C, Table C.1 (recommended number of checkpoints based on area), every row from 500 km² to 2,500 km²
- independent: yes
- inputs: project areas of 500, 750, 1,000, 1,250, 1,500, 1,750, 2,000, 2,250, and 2,500 km²
- outputs: 30, 35, 40, 45, 50, 55, 60, 65, and 70 checkpoints
- tolerance: exact
- verifiedBy: golden vectors v002 to v010, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `checkpoint_vectors`: the table transcribed as rows and looked up in Python, at every row's edge, just past the first edges, and for projects in hectares, acres, and square miles with a vegetated share.
- `core/vectors/drone.photogrammetry.checkpoint-requirements.jsonl`: 20 cases and three refusal cases.
- The integration suite checks the edges at 500, 501, and 2,500 km² and the 2,600 km² refusal.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `checkpoint_requirements_invariants`: across ASPRS Table C.1's areas and four vegetated shares, the count never falls as the area grows, a wholly open project has no vegetated count, and the checkpoint accuracy is half the class whatever the area.
