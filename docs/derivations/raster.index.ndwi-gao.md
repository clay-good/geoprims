<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# NDWI (Gao) (`raster.index.ndwi-gao`)

## Method

Water inside leaves absorbs shortwave infrared near 1.6 µm while near-infrared passes through the leaf, so the normalized difference of the two tracks how much liquid water vegetation holds. The Awesome Spectral Indices catalog lists this index as NDMI, the normalized difference moisture index; its NDWI is McFeeters' water index.

The tool takes one pixel's surface reflectance in each band, as fractions from 0 to 1, and applies the formula. It warns when the values look like unscaled digital numbers rather than reflectance, and refuses a pixel where the denominator is zero, since the index is undefined there (in an image, such a pixel is no-data).

## Equations

- NDWI (Gao) = (NIR − SWIR1) / (NIR + SWIR1)

## Symbols and units

Inputs: `nir`, `swir1`, surface reflectance of the near-infrared and shortwave infrared 1 (about 1.6 µm) bands, 0 to 1 (unitless). Output: `ndwi`, unitless.

## Domain

Surface reflectance from any sensor with the bands named, after atmospheric correction; values outside 0 to 1, or that look like raw digital numbers, raise SUSPECT_SCALING. A zero denominator is refused.

## Approximations

None: the formula is evaluated exactly on the reflectance given. What the value means depends on the sensor's band placement, the atmospheric correction, and the scene, which the index cannot see.

## Worked example

- sourcePublisher: Montero, D., and others, Awesome Spectral Indices (Scientific Data 10, 197, 2023)
- sourceTitle: spyndex, the Python front end of the Awesome Spectral Indices catalog; the index as catalogued: NDMI, from Gao (1996), Remote Sensing of Environment
- sourceEdition: spyndex 0.12.0
- sourceLocator: `spyndex.computeIndex("NDMI", params)`, with the catalog's constants
- independent: yes
- inputs: 400 random reflectance sets (0.01 to 0.6)
- outputs: the index value
- tolerance: 1e-12
- verifiedBy: `core/crates/gp-raster/tests/indices.rs` `indices_match_spyndex`; golden vectors v011 to v020
- verifiedOn: 2026-09-24

Awesome Spectral Indices is a curated, published catalog of spectral index formulas, each tied to its original paper, maintained independently of this tool; spyndex evaluates its formulas. On all 400 sets the two agree to 1e-12.

## Differential tests

- `tools/vectors/gen_indices_spyndex.py`: the spyndex fixture (400 reflectance sets for each of nine indices) and ten vectors appended to this tool's file
- `core/crates/gp-raster/tests/indices.rs` `indices_match_spyndex`: all 3,600 sets, within 1e-12

## Invariants

- `core/crates/gp-raster/tests/indices.rs` `index_invariants`: bounded between −1 and 1, zero for equal bands, negated by swapping the two bands, and unchanged when both bands are multiplied by the same factor (brighter or dimmer light).
