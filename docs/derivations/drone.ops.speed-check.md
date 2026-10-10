<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Part 107 groundspeed check (`drone.ops.speed-check`)

## Method

14 CFR 107.51(a) limits the groundspeed of a small unmanned aircraft, not its airspeed. The tool adds the wind along the track to the airspeed to get the groundspeed and compares it with the limit, which is read from the dated entry `faa-107-groundspeed` in `data/regulations.json` rather than written into the code. It reports the groundspeed, the limit, the margin left, and whether the flight is within or over the limit, with the rule's "as of" date.

## Equations

- Groundspeed: GS = airspeed + tailwind (a headwind is a negative tailwind; no tailwind given means 0). The reported groundspeed is not shown below 0.
- Limit: 87 kt, converted exactly (1 kt = 1852/3600 m/s).
- Margin: limit − GS.
- Status: "over the limit" when GS > limit, otherwise "within the limit". A groundspeed of exactly 87 kt is within the limit, because the rule says "may not exceed".

## Symbols and units

Airspeed and tailwind take any speed unit (mph by default). Outputs are in mph by default. 87 kt is 100.12 mph, which the regulation rounds to 100 miles per hour.

## Domain

Airspeed 0 or more (a negative airspeed is INVALID_INPUT). The tailwind may be negative. Magnitudes beyond any real speed are refused as OUT_OF_DOMAIN by the input check every quantity gets.

## Approximations

One steady wind straight along the track. Crosswind, gusts, and wind that changes with height are not modeled, and the airspeed is taken as given. The check covers only the speed limit of 107.51(a), not waivers or other rules.

## Worked example

- sourcePublisher: Federal Aviation Administration (14 CFR, via eCFR)
- sourceTitle: 14 CFR Part 107, Small Unmanned Aircraft Systems
- sourceEdition: eCFR, as in force on 2026-09-01, read 2026-10-09
- sourceLocator: § 107.51(a): "The groundspeed of the small unmanned aircraft may not exceed 87 knots (100 miles per hour)."
- independent: yes
- inputs: airspeed 87 kt, no wind; and airspeed 87.01 kt, no wind
- outputs: limit 87 kt (shown as 100 mph); 87 kt is within the limit with a margin of 0; 87.01 kt is over the limit
- tolerance: exact for the status; 1e-9 mph for the margin
- verifiedBy: golden vectors v006 and v007, run by the core on every build
- verifiedOn: 2026-10-09

The regulation gives the limit and its own conversion to miles per hour. The margins in the other vectors are arithmetic on that limit.

## Differential tests

- `tools/vectors/gen_drone.py`: the limit and each margin worked in Python from 87 kt, at 20 airspeed and wind pairs in mph, knots, m/s, and km/h, on both sides of the limit and exactly on it (within 1e-9)
- `core/vectors/drone.ops.speed-check.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `speed_check_invariants`: over 35 airspeed and wind pairs, the groundspeed is their sum, the margin is the limit less the groundspeed, and the status turns over exactly at 87 kt
- `core/crates/gp-drone/tests/power_ops.rs` `tailwind_pushes_over_the_limit`: a 15 mph tailwind takes a 90 mph airspeed over the limit, and the limit is 87 kt in mph
