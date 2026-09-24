<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# dNBR, burn severity (`raster.index.dnbr`)

## Method

The differenced normalized burn ratio is the pre-fire NBR minus the post-fire NBR. Burning lowers near-infrared and raises shortwave-infrared reflectance, so NBR falls where fire changed the ground, and the drop grows with the change. The result is named with the severity level whose range it falls in, from Key and Benson's table LA-2. That table is printed in whole units of dNBR scaled by 1,000, so the value is read the same way: scaled, rounded to a whole number, then placed in its range.

## Equations

- dNBR = NBR_pre − NBR_post
- k = round(1,000 × dNBR)
- Class by k (table LA-2): high regrowth below −250; low regrowth −250 to −101; unburned −100 to +99; low severity +100 to +269; moderate-low +270 to +439; moderate-high +440 to +659; high severity +660 and above

## Symbols and units

NBR_pre and NBR_post are normalized burn ratios, dimensionless, from −1 to 1. dNBR is dimensionless, from −2 to 2.

## Domain

Each NBR must lie between −1 and 1, or it is refused. Key and Benson find that valid data rarely go beyond about −550 to +1,350 (scaled). Outside that range the value is computed, classed, and flagged SUSPECT_VALUE, because it more likely comes from clouds, misregistration, or missing data than from fire.

## Approximations

The subtraction is exact arithmetic. The classes are the published starting point: Key and Benson say the thresholds are flexible, depend on the scene pair, and can shift by about 100 points. Severity mapping calibrates them against field plots.

## Worked example

- sourcePublisher: USDA Forest Service, Rocky Mountain Research Station
- sourceTitle: Key and Benson, Landscape Assessment (LA), in FIREMON (RMRS-GTR-164-CD)
- sourceEdition: 2006
- sourceLocator: Table LA-2, ordinal severity levels and example dNBR ranges (scaled by 1,000), page 38 of the downloadable PDF
- independent: yes
- inputs: NBR 0.37 before and 0.27 after (and each other class edge, from both sides)
- outputs: dNBR 0.10, which is +100 scaled: low severity (+99 is unburned; +270 moderate-low; +440 moderate-high; +660 high; −101 and −250 low regrowth; −251 high regrowth)
- tolerance: exact class names; dNBR within 1e-12
- verifiedBy: golden vectors v011 to v021, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_raster.py`: the table read independently in Python for ten ordinary pairs and every class edge from both sides, the two anomaly flags, and the out-of-range refusal
- `core/vectors/raster.index.dnbr.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-raster/tests/indices.rs` `dnbr_invariants`: swapping the images negates dNBR; shifting both NBRs together changes nothing; across every whole value from −540 to +1,340 the class never steps backward in table order, and none is flagged; a value past +1,350 is flagged
