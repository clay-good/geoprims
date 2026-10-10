<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Dataset size (`drone.sensors.dataset-size`)

## Method

Three storage estimates for a mapping job, each given only when its inputs are. An orthomosaic's uncompressed size is its pixel count times the bytes per pixel, shown with a range for an assumed compression. The raw capture is the image count times the size of one image. A LAS point cloud is its fixed header plus one record per point, at the record size the LAS 1.4 specification gives the chosen point format, and LAZ is that divided by an assumed ratio. The assumptions used are listed in the result.

## Equations

- Orthomosaic bytes = area ÷ GSD² × bands × bit depth ÷ 8; compressed range = that ÷ the low ratio and ÷ the high ratio (2 and 10 by default).
- Raw capture = image count × megabytes per image.
- LAS bytes = 375 + points × record size, with record sizes 20, 28, 26, 34, 57, 63, 30, 36, 38, 59, and 67 bytes for point formats 0 through 10 (format 6 by default).
- LAZ = LAS ÷ ratio (7 by default).
- 1 GB = 10⁹ bytes; 1 MB = 10⁶ bytes.

## Symbols and units

Area in any area unit (km² by default), GSD in any length unit (cm by default). Bands 3 by default, bit depth 8. Results are in gigabytes.

## Domain

Area and GSD above zero and given together; an orthomosaic over a petabyte is refused. Point format 0 to 10. The high compression ratio must be at least the low one.

## Approximations

The LAS size is exact for a file with no variable length records and no extra bytes per point: the specification's sizes are minimums, and real files often carry both. The orthomosaic and LAZ figures rest on assumed compression ratios, which depend on the content; processing software also adds overviews and tiles. The ratios are inputs, not measurements.

## Worked example

- sourcePublisher: American Society for Photogrammetry and Remote Sensing
- sourceTitle: LAS Specification 1.4 R15
- sourceEdition: 1.4 R15 (2019-07-09), read 2026-10-09
- sourceLocator: Public Header Block, Header Size: "For LAS 1.4 this size is 375 bytes"; and the "Minimum PDRF Size" line of each Point Data Record Format, 0 through 10: 20, 28, 26, 34, 57, 63, 30, 36, 38, 59, and 67 bytes
- independent: yes
- inputs: 1,000,000 points in each point format, 0 through 10
- outputs: 0.020000375 GB for format 0 and 0.030000375 GB for format 6, and so on: 375 bytes plus a million records of the format's size
- tolerance: 1e-12 GB
- verifiedBy: golden vectors v010 through v020, run by the core on every build
- verifiedOn: 2026-10-09

The specification gives the header and record sizes; the file size is that arithmetic. The orthomosaic figure, 7.5 GB for 1 km² at 2 cm in 8-bit RGB, is the spec scenario's and is arithmetic on the pixel count (vector v001).

## Differential tests

- `tools/vectors/gen_sensing.py`: the orthomosaic, raw capture, LAS, and LAZ arithmetic worked again in Python, with refused inputs
- `core/vectors/drone.sensors.dataset-size.jsonl`: those vectors, and 11 that hold the LAS size to the specification's record size for each point format, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/planning.rs` `dataset_size_invariants`: for every point format and three point counts the LAS size is the header plus the records and LAZ is LAS over the ratio; the orthomosaic scales with area, inversely with GSD squared, and with bands and bits; and each estimate appears only when its inputs are given
