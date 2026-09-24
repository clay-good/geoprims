<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Wind components, u and v (`aviation.wind.uv`)

## Method

A wind is an arrow: it has a length (the speed) and a direction. Pilots and forecasts give the direction the wind blows from, measured clockwise from true north. Weather models give the same arrow as two components instead: u, how fast the air moves toward the east, and v, how fast it moves toward the north. This tool turns one form into the other.

Because the direction is where the wind comes from, the air moves the opposite way, which is where the minus signs come from: a wind from the west (270°) moves air toward the east, so u is positive and v is zero. Going back, the direction is the angle of the reversed arrow, and the speed is the arrow's length.

## Equations

- Components: u = −s·sin θ, v = −s·cos θ, with θ the direction the wind blows from (degrees true) and s the speed.
- Back: θ = atan2(−u, −v), taken into [0°, 360°); s = √(u² + v²).

## Symbols and units

Inputs: either `direction` (degrees true, from) and `speed`, or `u` (positive toward the east) and `v` (positive toward the north), in any speed unit (knots by default). Outputs: all four, `u`, `v`, `direction`, and `speed`.

## Domain

Any direction and any speed of zero or more. With u and v both zero there is no direction, and the tool reports 0°, which then carries no meaning.

## Approximations

None: the conversion is exact trigonometry.

## Worked example

- sourcePublisher: Unidata (University Corporation for Atmospheric Research), MetPy
- sourceTitle: metpy.calc.wind_components, wind_direction, and wind_speed
- sourceEdition: MetPy 1.6.3
- sourceLocator: the wind_components documentation example, and 1,000 conversions through the three functions
- independent: yes
- inputs: MetPy's documented example, 10 m/s from 225°; 500 random directions and speeds from 0.5 to 150 kt, and 500 random u and v from −100 to 100 kt
- outputs: u and v for the first, and the direction and speed for the second
- tolerance: 1e-9 kt, with directions compared modulo 360° (MetPy reports a north wind as 360°, this tool as 0°)
- verifiedBy: `core/crates/gp-aviation/tests/uv.rs` `uv_matches_metpy`; golden vector v016 (the documented example) and v017 to v024
- verifiedOn: 2026-09-24

MetPy is Unidata's meteorology library for Python, written independently of this tool; its documentation gives 10 m/s from 225° as u = v = 7.07106781 m/s, which the tool reproduces. Across the 1,000 conversions the two agree to 1e-9 kt both ways.

## Differential tests

- `tools/vectors/gen_uv_metpy.py`: the MetPy fixture and vectors v016 to v024 (appended; v001 to v015 are from the earlier Python generator)
- `core/crates/gp-aviation/tests/uv.rs` `uv_matches_metpy`: all 1,000 conversions

## Invariants

- `core/crates/gp-aviation/tests/uv.rs` `uv_invariants`: for five winds around the compass, the components' length is the speed; converting to u and v and back returns the direction and speed; turning the wind a quarter turn clockwise turns (u, v) into (v, −u); and a wind from the west has a positive u and no v.
