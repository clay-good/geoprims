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

- sourcePublisher: Federal Aviation Administration (14 CFR, via eCFR)
- sourceTitle: 14 CFR Part 91, General Operating and Flight Rules
- sourceEdition: eCFR, as in force on 2026-09-01, read 2026-10-09
- sourceLocator: § 91.151(a)(1) and (2): airplanes under VFR, "at least 30 minutes" by day and "at least 45 minutes" at night; § 91.151(b): rotorcraft, "at least 20 minutes"; § 91.167(a)(3): IFR, "45 minutes at normal cruising speed or, for helicopters, ... 30 minutes"
- independent: yes
- inputs: one leg of 2 h at 10 gal/h, for each of airplane and rotorcraft with the VFR day, VFR night, and IFR reserves
- outputs: reserve times 30, 45, and 45 minutes for the airplane and 20, 20, and 30 for the rotorcraft; reserve fuel 5, 7.5, 7.5, 3.33, 3.33, and 5 gal; trip 20 gal each
- tolerance: exact for the reserve times; 1e-9 gal for the fuel
- verifiedBy: golden vectors v014 through v019, run by the core on every build
- verifiedOn: 2026-10-09

The regulation gives the reserve times. The fuel is that time at the cruise burn, which is arithmetic. The spec's own scenario, a VFR night flight of 1.5 h at 9.5 gal/h and 0.75 h at 9 gal/h with 1.4 gal of taxi fuel and 53 gal usable, gives trip 21.0 gal, reserve 6.75 gal, total 29.15 gal, and margin 23.85 gal (vector v013).

## Differential tests

- `tools/vectors/gen_fuel.py`: the same arithmetic and reserve table written again in Python: VFR day and night, rotorcraft, IFR with an alternate for airplane and helicopter, a custom reserve, no reserve, a liters-per-hour burn, a shortfall, and four refused inputs (within 1e-9)
- `core/vectors/aviation.loading.fuel-plan.jsonl`: those vectors, and ten that hold each reserve preset to the minutes 14 CFR 91.151 and 91.167 state, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `fuel_plan_invariants`: for each category and reserve, with and without taxi, climb, and alternate fuel, the reserve time is the regulation's, the reserve and alternate fuel are their times at the cruise burn, and the total is the sum of its parts
