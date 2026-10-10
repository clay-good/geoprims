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

- sourcePublisher: geoprims (tool example, hand check)
- sourceTitle: aviation.wind.aloft-interpolate primary example
- sourceEdition: 2026
- sourceLocator: Example "7,500 ft between the 6,000 and 9,000 ft winds" (u = (20 + 25.98)/2 = 22.99 kt, v = (0 − 15)/2 = −7.5 kt, so 288° at 24.2 kt; temperature 0 °C)
- independent: no
- inputs: 6,000 ft 270° 20 kt 3 °C; 9,000 ft 300° 30 kt −3 °C; altitude 7,500 ft
- outputs: u 22.99 kt, v −7.5 kt, wind 288.07° at 24.18 kt, temperature 0 °C, between 6,000 and 9,000 ft
- tolerance: 1e-9
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aloft.py`: a Python evaluation of the u/v convention and linear interpolation at six level sets, including a wind veering through north, opposite winds cancelling to calm, levels given out of order, and a query on a level (within 1e-9), plus a refused altitude above the levels
- `core/vectors/aviation.wind.aloft-interpolate.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool directly; this is the closest test that exercises it, and it runs every vector in the tool's file through the core and fails on any value outside its tolerance
- `core/crates/gp-aviation/tests/uv.rs` `uv_invariants`: exercises `aviation.wind.uv`, which shares this tool's u/v conversion functions, not this tool itself
