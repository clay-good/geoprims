<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Local time to UTC (Zulu) (`time.scale.utc-offset`)

## Method

With a fixed offset, UTC = local − offset. With an IANA zone name, the offset in force comes from the embedded tzdb 2026d (TZif data with POSIX rules for future years): at the UTC instant for UTC to local, and for local to UTC by finding the local time's instant. A local time that falls in a spring-forward gap is refused. One repeated at a fall-back resolves to its first occurrence, with AMBIGUOUS_INPUT. The result gives the answer in the direction asked (the Zulu time for local to UTC; the local clock time and zone for UTC to local), the Zulu time, both timestamps, the zone abbreviation, and any change of date.

## Equations

- UTC = local − offset, local = UTC + offset.
- Named zones: offset = utoff of the last transition at or before the instant, or the POSIX rule after the table.
- Day shift = UTC date − local date.

## Symbols and units

Times as YYYY-MM-DDTHH:MM (with Z for UTC input), offsets as ±HH:MM, +HHMM, or UTC−7, and zone names as in tzdb (America/Denver, and legacy names such as CET or EST5EDT).

## Domain

Offsets from −14:00 to +14:00, and dates the tzdb covers (1970 to 2100 are checked). A named zone's result echoes the tzdb release in meta.assets.

## Approximations

None for the stated tzdb release. Time-zone rules are political and change; a later release can move future offsets.

## Worked example

- sourcePublisher: Python Software Foundation
- sourceTitle: zoneinfo, IANA time zone support (Python documentation)
- sourceEdition: Python 3 documentation, retrieved 2026-09-19
- sourceLocator: America/Los_Angeles examples: 2020-10-31 12:00 is −07:00 PDT; 2020-11-01 12:00 is −08:00 PST; 2020-11-01 01:00 is −07:00 (fold 0) or −08:00 (fold 1); 08:00Z and 09:00Z that morning are 01:00 −07:00 and 01:00 −08:00
- independent: yes
- inputs: 2020-10-31T12:00, America/Los_Angeles
- outputs: 2020-10-31T19:00:00Z, local 2020-10-31T12:00:00−07:00, PDT
- tolerance: exact
- verifiedBy: golden vectors v012 to v016, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-time/tests/tz_parity.rs`: `zulu_tool_matches_zoneinfo` runs the public tool on 2,388 instants across every zone from 1970 to 2100 against Python zoneinfo on the same tzdata 2026d (offset and abbreviation identical; GMT and UTC entered as names read as the offset +00:00), and `committed_fixture` checks the zone table itself
- `tools/vectors/gen_tz_diff.py`: regenerates that fixture
- `core/vectors/time.scale.utc-offset.jsonl`: 25 vectors, including half- and quarter-hour zones, legacy names, a DST gap, a fall-back hour, and UTC to local answers

## Invariants

- `core/crates/gp-time/tests/time.rs` `utc_offset_invariants`: UTC to local and back is the identity (away from fall-back hours), and local minus UTC equals the offset shown, across fixed offsets and named zones
