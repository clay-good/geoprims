<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Time of useful consciousness (`aviation.physiology.time-of-useful-consciousness`)

## Method

The tool quotes two FAA tables and computes nothing. For an altitude it returns the row of AC 61-107B Change 1 Figure 2-3, both its time of useful consciousness (TUC, also called effective performance time) and its time after a rapid decompression, and the row of the Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C) Figure 17-1 beside it, each named. An altitude between two rows reports both rows and says that no interpolation is made, because a time between two printed ranges is not something either table states. Every result carries the advisory circular's caution about individual variation and the physiology notice.

## Equations

None: table lookup. On a printed row, that row. Between rows r and r + 1 (alt_r < h < alt_r+1), "value_r at alt_r; value_r+1 at alt_r+1".

## Symbols and units

h is the altitude in feet (any length unit is accepted). Table entries are kept as printed, in minutes (min) or seconds (s), as text ranges. "nominal" is the advisory circular's own word for the rapid-decompression time at 40,000 ft and above.

## Domain

The advisory circular's rows run from 18,000 to 50,000 ft; other altitudes are refused with OUT_OF_DOMAIN rather than borrowing an end row that does not describe them. The handbook's rows run from 20,000 to 45,000 ft MSL; outside them its column says "not tabulated" instead of quoting a neighbor.

## Approximations

The tables are averages. AC 61-107B Change 1 added a caution that times vary widely among individuals and that the lower value should be assumed limiting; the tool shows its first and last sentences on every result. The handbook's table differs from the circular's in places (at 22,000 ft, 5 to 10 minutes against 10 minutes; at 45,000 ft it has a row the circular lacks), which is why both are shown and named. The altitude is read as the altitude the person breathes at, so a pressurized cabin's cabin altitude, not the airplane's flight level, unless the cabin has lost pressure.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: AC 61-107B Change 1, Aircraft Operations at Altitudes Above 25,000 Feet MSL or Mach Numbers Greater Than .75; and the Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: Change 1, 9/9/15; FAA-H-8083-25C (2023)
- sourceLocator: AC Figure 2-3, the 25,000 ft row (page 24); PHAK chapter 17, Figure 17-1
- independent: yes
- inputs: altitude 25,000 ft
- outputs: 3–5 min; 1.5–2.5 min after a rapid decompression; PHAK 3 to 5 minutes; the caution and the notice
- tolerance: exact text
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/physiology.rs` `time_of_useful_consciousness_invariants`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_tuc.py`: every row of both tables as printed, nine altitudes between rows (reporting both rows), and the two refusals at 17,999 and 50,001 ft; 21 vectors in `core/vectors/aviation.physiology.time-of-useful-consciousness.jsonl`. The lookup is trivial, so these check the transcription and the between-rows rule, not a second method.

## Invariants

- `core/crates/gp-aviation/tests/physiology.rs` `time_of_useful_consciousness_invariants`: the spec's 25,000 ft and 23,000 ft scenarios; each printed row returns exactly its own entry; the caution and the notice are always present; "not tabulated" outside the handbook's rows; refusal outside the circular's.
