<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Converted meteorological visibility (`aviation.ifr.visibility-to-cmv`)

## Method

Under EASA rules for commercial air transport by aeroplane, a converted meteorological visibility (CMV) may stand in for runway visual range when the reported RVR is not available, or when an approach's minimum RVR is more than the most the aerodrome assesses. AMC10 CAT.OP.MPA.110 (ED Decision 2022/012/R, applicable from 30 October 2022) gives it as the reported visibility times a factor from its Table 16, chosen by the lights in operation and by day or night, and says to use 1.0 for flight planning. The tool reads the factor from `data/regulations.json` (entries `easa-cmv-hi-day` to `easa-cmv-planning`) and multiplies. It is the opposite direction from the FAA's table in 14 CFR 91.175(h)(2), which turns an RVR minimum into a visibility, so it is a separate tool from `aviation.ifr.rvr-visibility`.

## Equations

- CMV = reported visibility × factor.
- Table 16: high-intensity approach and runway lights 1.5 by day, 2.0 at night; any other light installation 1.0 by day, 1.5 at night; no lights 1.0 by day, not applicable at night.
- Flight planning: factor 1.0 (AMC10 CAT.OP.MPA.110(c)(1)).

## Symbols and units

Visibility and CMV in meters by default (statute miles and kilometers are accepted). The factor is a pure number.

## Domain

Reported visibility more than 0 and at most 100 km, else INVALID_INPUT. OUT_OF_DOMAIN, quoting the AMC, for take-off minima, for continuing an approach in low-visibility operations, and at night with no lights, where Table 16 gives no factor.

## Approximations

None in the arithmetic. The AMC is an acceptable means of compliance, not the regulation itself, and the operator's approved procedures govern. The text was read in EASA's consolidated Easy Access Rules for Air Operations, Revision 24 (March 2026), which EASA marks as unofficial; the ED Decision is the legal text. Table 16 was numbered Table 8 before Amendment 21 of the AMC and GM to Part-CAT.

## Worked example

- sourcePublisher: European Union Aviation Safety Agency
- sourceTitle: Easy Access Rules for Air Operations (Regulation (EU) No 965/2012), AMC10 CAT.OP.MPA.110
- sourceEdition: Revision 24, March 2026
- sourceLocator: Table 16, Conversion of reported VIS to RVR/CMV, page 967
- independent: no
- inputs: reported visibility 1,500 m, high-intensity approach and runway lights, night
- outputs: CMV 3,000 m, factor 2.0
- tolerance: 1e-12 relative
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/approach.rs` `visibility_to_cmv_table_16`
- verifiedOn: 2026-10-20

The AMC prints the factors, not this example; the 3,000 m is the printed factor applied to a chosen visibility.

## Differential tests

- `tools/vectors/gen_cmv.py`: every Table 16 cell with a factor at 800, 1,200, and 2,500 m, the planning factor, an explicit approach purpose, inputs in statute miles and kilometers, and the three refusals; 23 vectors in `core/vectors/aviation.ifr.visibility-to-cmv.jsonl`. The arithmetic is one product, so these check the table's transcription, the units, and the exclusions.

## Invariants

- `core/crates/gp-aviation/tests/approach.rs` `visibility_to_cmv_table_16`: each cell of Table 16 gives its factor and visibility × factor, citing AMC10 CAT.OP.MPA.110; night with no lights is refused as not applicable; planning uses 1.0 whatever the lights; take-off and LVO continuation are refused; 1 SM at night with other lights is 1.5 SM.
