<!-- Derivation note (trust/correctness-program layer A). -->
# Drone range (`drone.power.range`)

## Method

A drone flying at a steady power uses its energy budget in a known time, and covers ground at its groundspeed for that long. The operation takes the energy budget from the same function as `drone.power.endurance`, divides by the power in forward flight, and multiplies by the groundspeed, which it takes as entered or solves from the airspeed and wind.

## Equations

- Energy to fly on = E × usable share × (1 − derating) × (1 − reserve)
- Flight time t = energy to fly on ÷ P
- Range = t × groundspeed
- Groundspeed from airspeed V, course, and wind W from direction θ: with β = θ − course, groundspeed = √(V² − (W sin β)²) − W cos β

## Symbols and units

E rated energy (Wh); P power in forward flight (W); speeds in m/s, km/h, kt, or mph; course and wind direction in degrees, the wind named by where it blows from.

## Domain

Positive energy, power, and speed. A groundspeed, or an airspeed with a wind, not both. A crosswind stronger than the airspeed or a headwind at least equal to it is refused with NO_SOLUTION.

## Approximations

Steady power and speed in one direction. Climb, hover on task, and the return leg are not included.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/power, requirement "Endurance" (range from cruise time and groundspeed); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 99 Wh, 85% usable, 25% reserve, 180 W, 14 m/s
- outputs: 63.11 Wh to fly on, 21.04 min, 17.67 km
- tolerance: 1e-9
- verifiedBy: golden vector v001 and the `focused_power_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `range_vectors`: 17 random flights, half with a groundspeed and half with an airspeed, course, and wind solved by a separately written wind triangle.
- `core/vectors/drone.power.range.jsonl`: those vectors, a direct headwind and tailwind, and four refusal cases.
- The integration suite requires the same range as `drone.power.endurance` and checks the headwind ratio.

## Invariants

- Range = flight time × groundspeed.
- Doubling the power halves the range.
- A direct headwind of W reduces the groundspeed by W; a direct tailwind adds it.
