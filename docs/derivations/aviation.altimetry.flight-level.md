<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Flight level and altitude (`aviation.altimetry.flight-level`)

## Method

A flight level is a pressure altitude in hundreds of feet. An altimeter set to QNH reads the pressure altitude less PA(QNH), the pressure altitude of the setting itself, so a flight level's altitude on QNH is FL × 100 ft − PA(QNH), and an altitude's flight level is (altitude + PA(QNH)) ÷ 100. The US lowest usable flight level is read from the 14 CFR 91.121(b) table: FL180 at 29.92 inHg or higher, 5 levels (500 ft) higher for each started 0.50 inHg below 29.92, down to FL210 at 26.92 inHg. The transition altitude (18,000 ft) and the rule's citation and review date come from the regulations data file.

## Equations

- PA(QNH): the ISA altitude whose pressure is QNH, by the closed-form inverse of the standard atmosphere.
- Altitude of a flight level: h = FL × 100 ft − PA(QNH).
- Flight level of an altitude: FL = (h + PA(QNH)) ÷ 100 ft.
- Lowest usable flight level: b = max(2992 − round(QNH in inHg × 100), 0); k = ⌈b ÷ 50⌉ (computed as (b + 49) div 50); LUFL = 180 + 5k when k ≤ 6, and no answer (OUT_OF_DOMAIN) when k > 6.

## Symbols and units

QNH accepts inHg, hPa, and METAR `A2942` or `Q996` groups; the table step uses QNH in inHg rounded to hundredths. Altitudes are in feet by default. The standard atmosphere uses p0 = 101,325 Pa, T0 = 288.15 K, L = −0.0065 K/m, R = 287.05287 J/(kg·K), g0 = 9.80665 m/s².

## Domain

Altimeter settings from 26.92 inHg up (below that the table ends and the tool returns OUT_OF_DOMAIN); flight levels from −20 to 600; altitudes from −1,000 m to 20,000 m (−3,281 ft to 65,617 ft).

## Approximations

The altitude conversions assume the standard atmosphere, so they do not show the error from air colder or warmer than ISA (the true altitude tool covers that). The lowest usable flight level table is the US rule only; other countries set their own transition altitudes and levels.

## Worked example

- sourcePublisher: Federal Aviation Administration (14 CFR, via eCFR)
- sourceTitle: 14 CFR Part 91, General Operating and Flight Rules
- sourceEdition: eCFR, current (rules as of 2026-09-22 in data/regulations.json)
- sourceLocator: § 91.121(b), the lowest usable flight level table (settings 29.91 through 29.42 inHg give FL185); cited by the add-aviation-suite altimetry scenario "Lowest usable flight level"
- independent: yes
- inputs: altimeter 29.42 inHg, flight level 185
- outputs: lowest usable flight level FL185 (from the regulation's table); the altitude of FL185 on this QNH, 18,033 ft, is computed in Python by tools/vectors/gen_qcodes.py, not published
- tolerance: exact for the flight level; 1e-6 relative for the altitude
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_qcodes.py`: a separate Python standard atmosphere and table rule, at six settings from 27.00 to 30.25 inHg with flight levels and altitudes in both directions, and one setting below the table that must fail (within 1e-6 relative)
- `core/vectors/aviation.altimetry.flight-level.jsonl`: those vectors (v008 to v013 and v007), run through the core on every build; v001 to v006 are kept as superseded, from the version that scaled QNH by the field's ISA pressure ratio

## Invariants

- `core/crates/gp-aviation/src/qcodes.rs` `the_91_121_table`: the lowest usable flight level steps at the table's band edges (30.10 and 29.92 give 180, 29.91 and 29.42 give 185, 29.41 gives 190, on to 26.92 giving 210), and 26.91 inHg gives no answer
