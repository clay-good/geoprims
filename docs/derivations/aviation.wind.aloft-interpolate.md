<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Winds aloft between levels (`aviation.wind.aloft-interpolate`)

## Method

Each forecast level's wind is turned into east (u) and north (v) components by the meteorological convention. The levels are sorted by altitude, and the two that bracket the requested altitude are found. The u and v components, and the temperature when both levels give one, are interpolated linearly in altitude. The wind is read back from the interpolated u and v. Working with components means a wind that veers or backs between levels turns smoothly, instead of averaging two directions.

## Equations

- For a wind from θ (true) at speed s: u = −s · sin θ, v = −s · cos θ
- f = (h − h_below) / (h_above − h_below)
- u = u_below + f · (u_above − u_below); v likewise; T = T_below + f · (T_above − T_below)
- Speed = √(u² + v²); direction (from) = atan2(−u, −v), wrapped to [0°, 360°); a speed under 1e-9 kt reads as calm, direction 0°

## Symbols and units

h altitude in feet; θ direction the wind blows from, degrees true; s, u, v in knots; T in °C.

## Domain

2 to 20 levels with distinct altitudes and speeds of zero or more; the altitude must lie between the lowest and highest level, or the tool returns OUT_OF_DOMAIN (no extrapolation). Temperature is returned only when both bracketing levels give one.

## Approximations

Linear in altitude between two levels; the forecast's own error is larger. When the wind turns sharply between levels, the vector blend can give a lower speed than either level (opposite winds cancel to calm). Directions are true, as forecasts give them.

## Worked example

- sourcePublisher: Unidata MetPy and NumPy
- sourceTitle: MetPy 1.6.3, metpy.calc wind_components, wind_direction, and wind_speed; NumPy 1.26.4, numpy.interp
- sourceEdition: MetPy 1.6.3 and NumPy 1.26.4, run 2026-10-10
- sourceLocator: tools/vectors/gen_aloft_metpy.py, second case: five levels from 3,000 to 18,000 ft, read at 7,500 ft
- independent: yes
- inputs: 6,000 ft 260° at 20 kt, 9 °C; 9,000 ft 280° at 35 kt, 3 °C (with levels at 3,000, 12,000, and 18,000 ft that do not bracket the altitude); altitude 7,500 ft
- outputs: wind from 272.8° at 27.1 kt (u 27.08 kt, v −1.30 kt), temperature 6 °C, between the 6,000 and 9,000 ft levels
- tolerance: 1e-9 in every field
- verifiedBy: golden vector v009, run by the core on every build
- verifiedOn: 2026-10-10

MetPy and NumPy give 15 more cases, v008 and v010 through v023, over five sets of levels from 3,000 to 39,000 ft: winds that veer, back, and swing through north, levels given top down, and levels without temperatures.

## Differential tests

- `tools/vectors/gen_aloft_metpy.py`: u, v, speed, direction, temperature, and the bracketing levels at 16 altitudes, with the components from MetPy and the interpolation from NumPy (within 1e-9); no case sits on a level or has a resultant under 3 kt
- `tools/vectors/gen_aloft.py`: a Python evaluation of the u/v convention and linear interpolation at six level sets, including a wind veering through north, opposite winds cancelling to calm, levels given out of order, and a query on a level (within 1e-9), plus a refused altitude above the levels
- `core/vectors/aviation.wind.aloft-interpolate.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `aloft_invariants`: at a level the answer is that level's wind and temperature; at 11 altitudes between two levels u, v, and temperature lie on the straight line between them, the speed is the length of (u, v), a wind from 350° to 040° swings through north, the order of the levels and a level outside the bracket change nothing; and an altitude a foot past either end is refused
