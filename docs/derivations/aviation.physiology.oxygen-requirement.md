<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Supplemental oxygen requirement (`aviation.physiology.oxygen-requirement`)

## Method

The tool names the supplemental-oxygen paragraph that applies at a cabin altitude, quotes what it requires, and, given a time at that altitude, says how many of those minutes the rule covers. It follows 14 CFR 91.211 by default, or 135.89 or 121.329 when the user picks Part 135 or 121. Each threshold is read from `data/regulations.json`, not written in the code. Every threshold is "above": an altitude equal to a threshold falls in the band below it. When a flight altitude is given, the tool adds the pressurized-airplane rule for it. Every result carries the citation, the review date, the link, and a notice that it is a dated reference, not medical or legal advice.

## Equations

Part 91 (91.211(a); entries `faa-91-211-a1`, `-a2`, `-a3`):
- h ≤ 12,500 ft: no 91.211(a) rule; 0 minutes.
- 12,500 < h ≤ 14,000 ft: (a)(1), crew; minutes = max(t − 30, 0).
- 14,000 < h ≤ 15,000 ft: (a)(2), crew the entire time; minutes = t.
- h > 15,000 ft: (a)(2) and (a)(3), crew the entire time and every occupant provided; minutes = t.

Part 135 (135.89(a); entries `faa-135-89-a1`, `-a2`):
- h ≤ 10,000 ft: no rule; 0 minutes.
- 10,000 < h ≤ 12,000 ft: (a)(1), each pilot; minutes = max(t − 30, 0).
- h > 12,000 ft: (a)(2), each pilot continuously; minutes = t.

Part 121 (121.329; entries `faa-121-329-crew-1`, `-crew-2`, `-pax-2`, `-pax-3`):
- h ≤ 10,000 ft: no rule; 0 minutes.
- 10,000 < h ≤ 12,000 ft: (b)(1) and (c); minutes = t.
- h > 12,000 ft: (b)(2) and (c); minutes = t.
- Passengers: above 10,000 ft, 10 percent for the part over 30 minutes; above 14,000 ft, 30 percent; above 15,000 ft, all.

Flight altitude F, when given:
- Part 91 (entries `faa-91-211-b1i`, `-b1ii`, `-b1ii-two`): F ≤ FL250, no rule; above FL250, a 10-minute supply for each occupant; above FL350, one pilot masked unless two pilots have quick-donning masks; above FL410, that exception ends.
- Part 135 (entries `faa-135-89-b2`, `-b3`): F ≤ 25,000 ft, no mask rule; above 25,000 ft through 35,000 ft, (b)(2); above 35,000 ft, (b)(3).
- Part 121: the tool points to 121.333 and does not quote it.

## Symbols and units

h (cabin pressure altitude) and F (flight altitude) in feet by default (any length unit accepted); t (time at that altitude) and the minutes on oxygen in minutes by default (any time unit). The part is 91, 135, or 121; leaving it out gives 91. The thresholds above are the regulation values held in `data/regulations.json` (Part 91 effective 2016-12-30; Parts 135 and 121 effective 2017-01-01; all reviewed 2026-10-19).

## Domain

Cabin altitude from −2,000 to 60,000 ft, else INVALID_INPUT at `/cabin_altitude` (also when it is missing). Time from 0 to 10,000 minutes, else INVALID_INPUT at `/duration`. The time and flight altitude are optional; without a time, no minutes are returned.

## Approximations

It quotes the rules' thresholds as of the review date. The time over 30 minutes is simple subtraction, and the duration is the user's estimate of the time in the altitude band. The tool does not judge fitness: hypoxia can begin well below these altitudes. Operator rules, the airplane's limitations, and other parts of the regulations may require more. Not medical or legal advice.

## Worked example

- sourcePublisher: Federal Aviation Administration (eCFR)
- sourceTitle: 14 CFR 91.211, Supplemental oxygen
- sourceEdition: current eCFR text (effective 2016-12-30), read through the eCFR versioner API on 2026-10-19
- sourceLocator: Paragraph (a)(1): above 12,500 ft through 14,000 ft MSL, the part of the flight at those altitudes that is more than 30 minutes
- independent: yes
- inputs: cabin altitude 13,500 ft, 45 minutes, Part 91
- outputs: paragraph 14 CFR 91.211(a)(1); 15 minutes on oxygen
- tolerance: exact for the paragraph; 1e-9 min for the minutes
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/physiology.rs` `oxygen_requirement_invariants`
- verifiedOn: 2026-10-20

The paragraph and its 30-minute allowance are printed in the regulation. The 15 minutes is 45 less 30.

## Differential tests

- No generator script in `tools/vectors/` names this tool. The vectors' source field is 14 CFR 91.211, 135.89, and 121.329 (eCFR, read 2026-10-19); each expected paragraph is read from the regulation's thresholds.
- `core/vectors/aviation.physiology.oxygen-requirement.jsonl`: 24 vectors. Part 91 on and just above each threshold (12,500, 12,600, 14,000, 14,001, 15,000, 15,001 ft) and inside the bands; Part 135 at 10,000, 11,000, 12,000, and 12,001 ft; Part 121 at 10,000 to 16,000 ft; two flight-altitude cases (Part 91 at 27,000 ft and Part 135 at 37,000 ft), which check the paragraph only; a cabin in meters with a time in hours (v021); and three INVALID_INPUT refusals: no cabin altitude, 70,000 ft, and −5 minutes.

## Invariants

- `core/crates/gp-aviation/tests/physiology.rs` `oxygen_requirement_invariants`: the spec's two scenarios (13,500 ft for 45 minutes under Part 91 is (a)(1) with 15 minutes; 11,000 ft for 40 minutes under Part 135 is (a)(1) with 10 minutes); for each part, the minutes on oxygen never fall as the cabin climbs from 8,000 to 19,750 ft in 250 ft steps for a 45-minute time; and a result at a 16,000 ft cabin and 36,000 ft flight altitude carries the notice, "rules as of 2026-10-19", and a pressurized rule that names FL350.
