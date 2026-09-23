<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# IANA time zone at a moment (`time.scale.zone-info`)

## Method

A time zone is not an offset. It is a named set of rules whose offset changes over time, and the only correct way to answer "what is the offset in Denver" is to ask "at what instant". So this tool takes a zone and a moment, and returns the offset, the abbreviation, whether daylight saving is in force, and when it next changes.

The data is the IANA tz database, compiled to TZif and embedded in the module so nothing is fetched at run time. A TZif file holds an explicit list of transitions up to some horizon and a POSIX rule string for everything after it, and both are used: instants inside the listed range get the listed transition, instants past it are worked out from the rule. The tzdb release is echoed in `meta.assets`, so an answer can always be traced to the data that produced it.

The next-change output is the part people actually need, because it is what turns "the offset is −06:00" into "and it becomes −07:00 on 1 November". It is the next instant at which this zone's offset differs from the current one.

## Equations

There are none. This is a table lookup over a published dataset, plus a POSIX-rule evaluation past the last listed transition. That is worth stating plainly: a zone's history is legislative, not mathematical, and any tool that computed it from a formula would be wrong.

## Symbols and units

`zone` is an IANA zone name such as `America/Denver`; `time` is an RFC 3339 instant. Out come `offset` (±hh:mm), `abbr`, `dst` (yes/no), `next_change` (an instant), `next_offset` and the canonical `zone_name`.

## Domain

Any zone in the embedded release, at any instant it covers. Zones with no daylight saving, such as `America/Phoenix` and `Asia/Kolkata`, simply report `dst: no` and the next change their rules give, which may be none.

## Approximations

None, within the release. The caveat is the release itself: zone rules change by legislation, often at short notice, so an answer for a future date is as good as the embedded tzdb is current. The release is reported with every result rather than assumed.

## Worked example

- sourcePublisher: Internet Assigned Numbers Authority; Python Software Foundation
- sourceTitle: IANA Time Zone Database; Python `zoneinfo`
- sourceEdition: IANA tzdb 2026c; CPython 3.13
- sourceLocator: the compiled TZif for each zone, read independently by `zoneinfo`; the next transition found by bisection rather than read from the core
- independent: yes
- inputs: 26 zone-and-instant pairs, including both hemispheres, a half-hour offset (Kolkata) and a three-quarter-hour one (Kathmandu), zones without daylight saving, and the instants either side of a northern and a southern changeover
- outputs: the offset, abbreviation, DST state, next change and next offset
- tolerance: exact, including the next change to the second
- verifiedBy: golden vectors v001 to v026, run by the core on every build
- verifiedOn: 2026-09-23

Two independent checks run here, at different scales. The vectors pin 26 specific answers including the next transition to the second. Separately, `core/crates/gp-time/tests/tz_parity.rs` compares the core's offset and abbreviation against Python's `zoneinfo` for **every zone in the release** at instants from 1970 to 2100 — a committed fixture of four per zone on every build, and 400 per zone (238,800 instants) on demand. The vectors catch a wrong answer at a named moment; the parity test catches a whole zone being read wrongly.

## Differential tests

- `tools/vectors/gen_time_gps_tz.py`: 20 of the 26 vectors, from Python `zoneinfo`, with the next transition found by bisection
- `core/crates/gp-time/tests/tz_parity.rs`: every zone in the release against `zoneinfo`, 1970 to 2100
- `core/vectors/time.scale.zone-info.jsonl`: 26 vectors across both hemispheres and every kind of offset

## Invariants

- `core/crates/gp-time/tests/time.rs` `zone_info_invariants`: a zone answers differently in January and July where it observes daylight saving, and identically where it does not, so the DST flag tracks the rules rather than the calendar; Denver an hour either side of the spring change gives −07:00 then −06:00, which pins the transition to the hour rather than the day; the next change is always strictly later than the instant asked about, and the offset it reports is not the current one; half-hour and three-quarter-hour offsets survive formatting, Kolkata reading +05:30 and Kathmandu +05:45; UTC has no daylight saving and a zero offset; and an unknown zone name is refused rather than falling back to UTC
