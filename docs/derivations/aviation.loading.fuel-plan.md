<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fuel planning (`aviation.loading.fuel-plan`)

## Method

Arithmetic on the pilot's own times and burns. Each leg's fuel is its time times its burn, and the trip fuel is their sum. Fuel to the alternate and the reserve are each a time at the cruise burn, which defaults to the last leg's burn. The reserve time comes from a dated regulation preset chosen by reserve type and aircraft category, or from the pilot's own custom time. The total adds taxi, climb, trip, alternate, and reserve. When usable fuel is given, the tool reports the margin, the endurance at the cruise burn, and whether the total is within, near, or beyond the usable fuel.

## Equations

- Leg fuel = time × burn; trip = Σ leg fuel
- Cruise burn = the given cruise burn, or the last leg's burn
- Alternate fuel = alternate time × cruise burn
- Reserve fuel = reserve minutes / 60 × cruise burn
- Total = taxi + climb + trip + alternate + reserve
- Margin = usable − total; endurance = max(usable − taxi, 0) / cruise burn
- Status: beyond if total > usable; near if total ≥ 0.9 × usable; otherwise within
- Reserve minutes: airplane VFR day 30 (14 CFR 91.151(a)(1)), VFR night 45 (91.151(a)(2)); rotorcraft VFR 20 (91.151(b)); airplane IFR 45 and helicopter IFR 30 (91.167(a)(3)); custom as entered; none 0

## Symbols and units

Times in hours (reserve in minutes), burns in US gallons per hour, fuel in US gallons by default; other volume and flow units are converted. The reserve rule text carries the citation, its "rules as of" review date, and the eCFR link from data/regulations.json.

## Domain

One to 20 legs, each with a time of zero or more and a burn above zero. Taxi, climb, alternate time, and custom reserve cannot be negative; cruise burn and usable fuel must be above zero. A custom reserve needs its time, and a reserve time without the custom choice is refused (INVALID_INPUT).

## Approximations

None in the arithmetic: the answer is as good as the times and burns entered. The tool does not model wind, climb, or power changes inside a leg. The reserves are the regulatory minimums as of the review date; operators often require more, and the result says so. It is a summary, not legal advice.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite fuel-and-loading spec, scenario "VFR night reserve"
- sourceEdition: openspec/changes/add-aviation-suite/specs/aviation/fuel-and-loading/spec.md
- sourceLocator: Requirement on fuel reserve presets; the spec requires a 45-minute reserve at the entered cruise burn, cited to 14 CFR 91.151(a)(2); the legs and fuel numbers are the project's example, worked by hand
- independent: no
- inputs: legs 1.5 h at 9.5 gal/h and 0.75 h at 9 gal/h; VFR night reserve; taxi 1.4 gal; usable fuel 53 gal
- outputs: trip 21.0 gal (14.25 + 6.75); reserve 45 min, 6.75 gal at 9 gal/h; total 29.15 gal; margin 23.85 gal; endurance 5.73 h
- tolerance: 1e-9 gal
- verifiedBy: golden vector v013 (it supersedes v001, which carried the old eCFR link)
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_fuel.py`: the same arithmetic and reserve table written again in Python: VFR day and night, rotorcraft, IFR with an alternate for airplane and helicopter, a custom reserve, no reserve, a liters-per-hour burn, a shortfall, and four refused inputs (within 1e-9)
- `core/vectors/aviation.loading.fuel-plan.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no invariant test is written for this tool yet. This is the closest test: it lints and runs every aviation tool's golden vectors through the registry, including the ones above, and fails if any tool has fewer than five
