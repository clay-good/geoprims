<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Digital number to reflectance (`raster.scale.reflectance`)

## Method

Surface-reflectance products store each band as unsigned 16-bit integers. Each product states the straight line that turns a stored digital number (DN) back into reflectance, and this tool applies that product's line. For Sentinel-2 Level-2A, the band's offset is added and the sum is divided by the quantification value. Both values are in the product metadata, with −1000 and 10,000 as the usual values from processing baseline 04.00. For Landsat Collection 2 Level-2, the DN is multiplied by the published gain and the published offset is added. A DN of zero is each product's no-data value and is refused.

## Equations

- Sentinel-2 L2A: ρ = (DN + BOA_ADD_OFFSET) / QUANTIFICATION_VALUE, with BOA_ADD_OFFSET = −1000 (0 before baseline 04.00) and QUANTIFICATION_VALUE = 10,000 unless given
- Landsat Collection 2 Level-2: ρ = DN × 0.0000275 − 0.2

## Symbols and units

DN is the stored integer (dimensionless) and ρ is surface reflectance (dimensionless, usually 0 to 1).

## Domain

Any nonzero DN. Landsat DNs outside the valid range of 7,273 to 43,636, and results outside −0.2 to 1.5, are computed and flagged with SUSPECT_SCALING, because they usually mean a different product or a band that was already scaled. A quantification value of zero is refused.

## Approximations

None: the products define reflectance by these lines. The constants assumed when none are given are the usual ones. Where a product's metadata says otherwise, the metadata governs, and both Sentinel-2 values can be passed in.

## Worked example

- sourcePublisher: U.S. Geological Survey
- sourceTitle: How do I use a scale factor with Landsat Level-2 science products?
- sourceEdition: retrieved 2026-09-24
- sourceLocator: The surface reflectance example: a pixel value of 18,639, times 0.0000275, plus −0.2, gives 0.313
- independent: yes
- inputs: DN 18,639, Landsat Collection 2 Level-2
- outputs: reflectance 0.313 (0.3125725 unrounded)
- tolerance: 0.0005 (the printed rounding)
- verifiedBy: golden vector v018, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_raster.py`: each product's published line evaluated in Python for six Sentinel-2 DNs at both baselines and five Landsat DNs, including both ends of the Landsat valid range, plus custom offsets and quantification values, band names, the SUSPECT_SCALING flags, and the refusals (within 1e-12)
- `core/vectors/raster.scale.reflectance.jsonl`: those vectors and the USGS example, run through the core on every build

## Invariants

- `core/crates/gp-raster/tests/indices.rs` `reflectance_scaling_invariants`: each preset is a straight line, so one DN step is a fixed reflectance step; the two Sentinel-2 baselines differ by exactly 0.1; inverting the line recovers the DN; and every Landsat DN in the valid range gives a plausible reflectance
