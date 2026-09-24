<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Drone flight time (`drone.power.endurance`)

## Method

Flight time is the energy you let yourself use divided by the power the drone draws. The tool starts from the pack's energy, keeps the usable share (a depth-of-discharge limit), takes off a cold-battery derating, and then holds back the landing reserve. What is left, divided by the hover power, is the hover time; divided by a cruise power, if you give one, it is the cruise time. Range is that time multiplied by the groundspeed.

It does not model the aircraft. The power comes from you: a test flight, the hover-power tool (momentum theory, which is where air density enters), or the calibration tool. That keeps this step exact arithmetic and puts all the physics, and all its uncertainty, in the power figure.

## Equations

- usable = E × s × (1 − d)
- flyable = usable × (1 − r)
- t_hover = flyable / P_hover; t_hover, no reserve = usable / P_hover
- t_cruise = flyable / P_cruise
- range = t_cruise × v_ground (t_hover when no cruise power is given)
- Cold derating heuristic, only when a battery temperature is given and no derating is entered: d = 0 at 20 °C and warmer, 1% per °C below (20% at 0 °C), capped at 50%.
- Peukert, off unless an exponent k is entered: every time is multiplied by (P_rated / P)^(k − 1), with P_rated = E / rated discharge time (1 h by default).

## Symbols and units

E pack energy (Wh, or any energy unit); s usable share (%); d derating (%); r reserve (%, a share of the usable energy); P_hover and P_cruise electrical power (W); v_ground groundspeed (m/s); times in minutes and range in kilometres; k Peukert exponent (dimensionless).

## Domain

Positive energy and powers. Usable share 1% to 100%, reserve 0% to 99%, derating 0% to 95%, Peukert exponent 1 to 1.5, and a positive rated time that is only accepted with an exponent.

## Approximations

- Steady power. A real flight's draw changes with climbs, wind, and manoeuvres, and rises as the pack's voltage sags; the time is an average-power estimate.
- The energy is the pack's rated figure. Age, temperature, and high current all reduce what a pack delivers. The cold derating stands in for temperature and is labeled a rule of thumb (`HEURISTIC_DERATING`); enter your own when you have test data.
- The reserve is a share of the usable energy, not of the whole pack. With 80% usable and a 20% reserve the tool flies on 64% of the pack, where the battery-energy tool's depth of discharge minus reserve would give 60%. Both are stated on their pages; to chain them, feed the battery tool's usable energy in with the usable share and reserve left at their defaults (the invariants test checks this).
- No air density here. Density changes the power, not the energy, so it belongs in the power figure: the hover-power tool uses the ISA troposphere (ρ0 = 101,325 / (287.05287 × 288.15) = 1.2250 kg/m³ at sea level) and scales power by √(ρ0/ρ).
- Peukert's law was fitted to lead-acid cells. Bauersfeld and Scaramuzza (2022, Sec. II and V) note it has been shown to hold for LiPo packs only at medium discharge rates, and only for a steady draw, and they use a battery voltage model instead. So here it is off by default, needs your own exponent, and is labeled (`HEURISTIC_PEUKERT`) when used.
- Range ignores wind: work out the groundspeed first.

## Worked example

- sourcePublisher: IEEE Robotics and Automation Letters (L. Bauersfeld and D. Scaramuzza)
- sourceTitle: Range, Endurance, and Optimal Speed Estimates for Multicopters
- sourceEdition: Vol. 7, No. 2, pp. 2953-2960 (2022), DOI 10.1109/LRA.2022.3145063; free copy arXiv:2109.04741
- sourceLocator: Sec. VII-E, "Algorithm and Example", step 6 (DJI Mavic 3, 4S1P): t_e = C_eff × 3.7 V × N_S × 3600 s / P_mot,e
- independent: yes
- inputs: effective capacity 4.89 Ah at 4 × 3.7 V (72.372 Wh) and 89.5 W for maximum endurance; 4.88 Ah (72.224 Wh) and 107.0 W for maximum range
- outputs: t_e = 2,909 s (48.48 min); t_r = 2,429 s (40.48 min)
- tolerance: 0.06 min (3.6 s), the spread from the capacity being printed to three significant figures (±0.005 Ah is ±3 s)
- verifiedBy: golden vectors v010 (hover time) and v011 (cruise time), run by the core on every build
- verifiedOn: 2026-09-23

The paper's steps 1 to 5 derive the power and effective capacity from its aerodynamic and battery models; step 6 is the energy-over-power division this tool performs, and the tool is checked against step 6's printed inputs and output. From the printed inputs the division gives 2,911 s and 2,430 s, inside the rounding of the capacity. The paper's range, x_r = 32.1 km, is not used as a check: its own t_r × v_r (2,429 s × 13.12 m/s) is 31.9 km, so vector v012 pins the formula instead.

## Differential tests

- `tools/vectors/gen_endurance.py`: a separate Python implementation of the equations above, for vectors v010 to v029: the published example, range, reserve and usable share, the cold heuristic from 25 °C to −40 °C (including the 50% cap), a user derating overriding the temperature, unit conversion (kW, kJ), and five refused inputs
- `tools/vectors/gen_drone.py`: v001 to v009, the spec scenario and the Peukert factor
- `core/vectors/drone.power.endurance.jsonl`: all 29, run through the core on every build

- `core/vectors/drone.power.endurance.jsonl` v030 to v038 (1.1.0): range at an airspeed in a wind, the groundspeed from the wind triangle recomputed in Python from its textbook form, including straight head- and tailwinds, a quartering wind, a crosswind, no wind, and the winds the drone cannot fly against

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `endurance_invariants`: time is linear in energy and inverse in power; usable share, derating, and reserve each scale it by their own factor; the times with and without reserve differ by exactly the reserve share; range is time × groundspeed; colder batteries never fly longer, the heuristic warns only below 20 °C and stops at 50%; and the battery tool's usable energy, fed in, gives that energy over the power
