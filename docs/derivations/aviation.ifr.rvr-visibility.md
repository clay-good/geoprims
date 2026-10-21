<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# RVR to ground visibility (`aviation.ifr.rvr-visibility`)

## Method

When an approach has RVR minimums but the runway's RVR is not reported, 14 CFR 91.175(h)(1) makes the comparable ground visibility in the table of (h)(2) the minimum. The tool converts the entered RVR to feet, finds the table row within half a foot, and returns that row's visibility as the table writes it and as a decimal. The decimal values are read from `data/regulations.json` (entries `faa-91-175-h2-1600` to `faa-91-175-h2-6000`). It does not interpolate between rows, because the rule does not.

With `table` set to `legend`, the tool uses the Terminal Procedures Publication legend's Comparable Values of RVR and Visibility instead (read in the d-TPP cycle 2611 front matter, effective October 29, 2026). That table repeats the regulation's seven rows, marked with a star, and adds eight more: 1,200, 1,800, 2,000, 2,200, 2,600, 3,000, 3,500, and 5,500 ft. For an RVR between rows it takes the next higher listed RVR, as the legend says (its own example: 4,800 ft uses 5,000 ft, 1 mile). The legend says the starred rows "shall be used for takeoff or landing minima", so a row the regulation does not list carries the RVR_NOT_IN_REGULATION caution, and the sentence says it is not a takeoff or landing minimum. Every result names the row it used as `rvr_used`.

## Equations

- Row match: |RVR_row − RVR| < 0.5 ft.
- Table (91.175(h)(2)): 1,600 ft is 1/4 SM; 2,400 ft is 1/2; 3,200 ft is 5/8; 4,000 ft is 3/4; 4,500 ft is 7/8; 5,000 ft is 1; 6,000 ft is 1 1/4.
- Decimal visibility: the entry's value in statute miles (0.25 to 1.25), converted at 1,609.344 m per mile.

## Symbols and units

RVR in feet by default (any length unit accepted; meters are converted to feet before the match). Visibility is text as the table writes it, and a decimal in statute miles shown to three decimals.

## Domain

Only the seven RVR values in the table, each within 0.5 ft, else INVALID_INPUT at `/rvr`, with the message that 91.175(h)(2) lists only those values and does not interpolate between them.

## Approximations

None; it is exact to the regulation's table, effective 2018-04-27 and reviewed 2026-10-19. The table does not apply to Category II or III minimums, which the result's "When it applies" text says. It is the US rule only; EASA's conversion of reported visibility to RVR is a different rule that this tool does not cover. A dated reference, not legal advice.

## Worked example

- sourcePublisher: Federal Aviation Administration (eCFR)
- sourceTitle: 14 CFR 91.175(h), Comparable values of RVR and ground visibility
- sourceEdition: current eCFR text (amended 2018-04-27), read 2026-10-19
- sourceLocator: Paragraph (h)(2) table, the 2,400 ft row
- independent: yes
- inputs: RVR 2,400 ft
- outputs: visibility 1/2 statute mile (0.5 SM)
- tolerance: exact for the text; 1e-9 SM for the decimal
- verifiedBy: golden vector v004 and `core/crates/gp-aviation/tests/approach.rs` `rvr_visibility_invariants`
- verifiedOn: 2026-10-20

## Differential tests

- The first 24 vectors have no generator script; their source field is 14 CFR 91.175(h)(2) (eCFR, read 2026-10-19), and each expected value is a row of the regulation's table.
- `tools/vectors/gen_rvr_legend.py`: the legend option, v025 to v043: the legend's own 4,800 ft example, all 15 printed rows (the eight the regulation lacks with the caution), 2,800 ft taking the 3,000 ft row, and refusals at 1,100 and 6,100 ft.
- `core/vectors/aviation.ifr.rvr-visibility.jsonl`: 43 vectors. The regulation mode: Each of the seven rows three ways: in feet, as a bare number, and in meters; and three INVALID_INPUT refusals: 2,000 ft and 1,800 ft between rows, and 7,000 ft above the table.

## Invariants

- `core/crates/gp-aviation/tests/approach.rs` `rvr_legend_table`: the legend's 4,800 ft example; every printed row returns itself, with the caution exactly on the rows the regulation lacks; the regulation's rows agree in both modes; refusals outside 1,200 to 6,000 ft; the regulation mode still refuses 2,800 ft.
- `core/crates/gp-aviation/tests/approach.rs` `rvr_visibility_invariants`: every row of the table returns exactly its visibility text, the decimal visibility rises with RVR from row to row, and 2,000 ft, between rows, is refused with INVALID_INPUT.
