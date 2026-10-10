<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Maximum holding airspeed (`aviation.ifr.hold-speed-limit`)

## Method

A table lookup. The AIM's maximum holding airspeeds are set by altitude band: 200 KIAS through 6,000 ft, 230 KIAS from 6,001 through 14,000 ft, and 265 KIAS above. The tool finds the band the holding altitude falls in. A limit printed on the chart, when entered, replaces the table. If a planned airspeed is given, it is compared with the limit and flagged when above it.

## Equations

- max IAS = 200 kt if altitude ≤ 6,000 ft; 230 kt if 6,000 ft < altitude ≤ 14,000 ft; 265 kt above 14,000 ft
- With a published limit: max IAS = published limit (the table is not consulted)
- Status: "above the limit" with ABOVE_MAX_HOLDING_SPEED when planned IAS > max IAS; otherwise "within the limit"

## Symbols and units

Altitude in feet MSL; airspeeds are indicated, in knots by default. `basis` names the AIM band or "published on the chart".

## Domain

Any holding altitude of zero or more (a negative altitude is INVALID_INPUT). The planned airspeed and published limit are optional.

## Approximations

None: this is reference data, applied as written. The band edges are inclusive at the top, so 6,000 ft gives 200 KIAS and 14,000 ft gives 230 KIAS; a fractional altitude above 6,000 ft (such as 6,000.5 ft) falls in the 230 KIAS band. Military fields and some procedures use other limits, and ATC instructions govern; the tool does not know about them unless the published limit is entered.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aeronautical Information Manual
- sourceEdition: Current edition (rules as of 2026-09-18)
- sourceLocator: Chapter 5, Section 3, paragraph 5-3-8j, maximum holding airspeeds table: 230 KIAS from 6,001 ft through 14,000 ft MSL
- independent: yes
- inputs: holding altitude 8,000 ft, planned 240 KIAS
- outputs: maximum 230 KIAS; planned speed above the limit, with ABOVE_MAX_HOLDING_SPEED
- tolerance: exact
- verifiedBy: `holding_speed_limit` in core/crates/gp-aviation/tests/slice3.rs
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: the AIM table applied by hand at six altitudes, on and either side of both band edges (3,000, 6,000, 6,001, 14,000, 14,001, and 35,000 ft)
- `core/vectors/aviation.ifr.hold-speed-limit.jsonl`: those six vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `holding_speed_limit`: 240 KIAS at 8,000 ft is flagged against 230 KIAS; the band edges give 200 at 3,000 and 6,000 ft, 230 at 6,001 and 14,000 ft, and 265 at 14,001 and 41,000 ft; a published 175 kt limit at 5,000 ft overrides the table and flags a planned 190 kt
