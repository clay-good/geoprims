<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Winds aloft (FB) decoder (`aviation.weather.fb-winds-decode`)

## Method

Decode FB wind and temperature groups by the forecast's coding rules. Paste one group with its level, a station line with the header levels, or the whole product: its FT header supplies the levels, and the DATA BASED ON and VALID lines are skipped. Missing groups are the lowest levels, so a station line lines up from the right.

## Equations

- Group DDSS[±TT]: direction DD × 10° true, speed SS kt.
- DD from 51 to 86: subtract 50 from DD and add 100 kt to the speed.
- 9900: light and variable (under 5 kt).
- Above 24,000 ft the temperature has no sign and is negative.

## Symbols and units

Levels in ft MSL, direction in degrees true, speed in kt, temperature in °C.

## Domain

Groups of 4, 6, or 7 characters, and levels up to 70,000 ft. A line with more groups than levels is refused with a hint to give the header.

## Approximations

None: it decodes the text as written. A forecast is only as good as its model run and its valid period (which the product header states).

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aviation Weather Handbook (FAA-H-8083-28B)
- sourceEdition: 2026
- sourceLocator: Section 27.2.1.1.2 and table 27-1 (Kansas City, MKC: 9900 1709+06 2018+00 2130-06 2242-18 2361-30 247242 258848, and 750252 at 39,000 ft). The earlier edition (FAA-H-8083-28A) printed the last group of the coded message as 550252, which would decode to 050°, against its own table's 250° at 102 kt, −52 °C. FAA-H-8083-28B corrects the message to 750252, matching the table, and that group is used.
- independent: yes
- inputs: the pasted product block
- outputs: light and variable at 3,000 ft; 170° at 9 kt, +6 °C at 6,000 ft; … 250° at 102 kt, −52 °C at 39,000 ft
- tolerance: exact
- verifiedBy: golden vectors v015 to v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: an independent Python encoder chooses the wind and temperature, codes the group by the FB rules, and expects them back (8 random groups at levels from 6,000 ft to 39,000 ft), alongside hand decodes
- `core/vectors/aviation.weather.fb-winds-decode.jsonl`: 23 vectors, including the whole handbook product and each of its table rows

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `fb_invariants`: every wind from 010° to 360° at 5 to 199 kt, with any temperature, round-trips through its code at 18,000 ft (signed) and 34,000 ft (unsigned)
