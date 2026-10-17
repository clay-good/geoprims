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

- sourcePublisher: IEEE Robotics and Automation Letters (L. Bauersfeld and D. Scaramuzza)
- sourceTitle: Range, Endurance, and Optimal Speed Estimates for Multicopters
- sourceEdition: Vol. 7, No. 2 (2022), DOI 10.1109/LRA.2022.3145063; free copy arXiv:2109.04741
- sourceLocator: Sec. VII-E max-range case (DJI Mavic 3): 72.224 Wh at 107.0 W gives t_r = 2,429 s (40.48 min), and t_r × v_r at 13.12 m/s is 31.9 km
- independent: yes
- inputs: 72.224 Wh, 100% usable, 0% reserve, 107.0 W, 13.12 m/s
- outputs: 40.50 min of flight and 31.88 km of range (the paper's t_r × v_r is 31.9 km; the spread is the capacity's three-figure rounding)
- tolerance: 0.1 km (the capacity rounding, as the endurance tool's note explains)
- verifiedBy: the `focused_power_operations_match_their_general_tools` integration test, which requires the range and usable energy to equal the stable `drone.power.endurance` tool's at the same power and groundspeed
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `range_vectors`: 17 random flights, half with a groundspeed and half with an airspeed, course, and wind solved by a separately written wind triangle.
- `core/vectors/drone.power.range.jsonl`: those vectors, a direct headwind and tailwind, and four refusal cases.
- The integration suite requires the same range as `drone.power.endurance` and checks the headwind ratio.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_power_operations_match_their_general_tools`: the range and usable energy equal the stable `drone.power.endurance` tool's at the same power and groundspeed, and a headwind reduces the range by the share it takes off the groundspeed.
- Range = flight time × groundspeed.
- Doubling the power halves the range.
