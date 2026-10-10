<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Rank runways for the wind (`aviation.wind.best-runway`)

## Method

The runway-components calculation, repeated for every runway end and then sorted. Each end's heading is taken as ten times its number (36 for 0°). The wind is resolved along and across that heading, after converting between true and magnetic when the wind and runway use different references and a variation is given. Runways within every limit entered come first; within each group, the most headwind ranks first, and ties go to the least crosswind. A variable wind (a range, or VRB) is judged by its worst case over its directions, and gusts are resolved the same way as the steady wind.

## Equations

- θ = wind direction − runway heading (in the runway's reference)
- Headwind h = W · cos θ (negative is a tailwind); crosswind x = |W · sin θ|, from the right when W · sin θ > 0 and from the left when it is below 0
- Components under 1e-9 kt are set to exactly zero
- Gust components: the same with the gust speed in place of W
- Worst case used against limits: crosswind = max(steady, gust); tailwind = max(−h, −h_gust, 0)
- Beyond limits when worst crosswind > max crosswind or worst tailwind > max tailwind; each limit also gets a within, near (≥ 90% of the limit), or beyond phrase
- Sort key: (beyond limits, −h, x), stable for ties

## Symbols and units

W is the wind speed in knots; directions in degrees, where the wind blows from. Runway headings and ATIS winds are magnetic by default; a METAR group is true; a `T` suffix on a runway marks it true. Up to 24 runway ends.

## Domain

At least one runway end, no designator listed twice, and at most 24 ends. Gusts at least the steady speed, and speeds not negative. Mixed true and magnetic references need a variation. A METAR wind group with a magnetic reference is refused.

## Approximations

Each runway is taken as ten times its number, so components can be off by up to W · sin 5°, and a close call between two runways may flip with the published headings; the RUNWAY_HEADING_APPROXIMATE warning says so on every result. The tool knows nothing of runway length, surface, slope, obstacles, noise rules, or traffic. The runway in use is the one ATC or local procedures assign.

## Worked example

- sourcePublisher: Unidata MetPy
- sourceTitle: MetPy 1.6.3, metpy.calc.wind_components
- sourceEdition: MetPy 1.6.3, run 2026-10-10
- sourceLocator: tools/vectors/gen_runway_metpy.py, first case: wind_components(12 kt, 200°), resolved along and across headings 090°, 270°, 180°, and 360°
- independent: yes
- inputs: runways 09/27, 18/36; wind 200° at 12 kt
- outputs: order 18, 27, 09, 36; runway 18 headwind 11.28 kt and crosswind 4.10 kt; 27 headwind 4.10 kt; 09 tailwind 4.10 kt; 36 tailwind 11.28 kt
- tolerance: 1e-9 kt
- verifiedBy: golden vector v010, run by the core on every build
- verifiedOn: 2026-10-10

MetPy gives the wind as eastward and northward parts, and the generator takes each runway's headwind and crosswind from them by two dot products with the runway's heading; the tool works from the angle between wind and runway. The ranking in the generator follows the rule stated above, so the library checks the components and the rule is checked by being written twice and by the invariants below. 13 more airports follow, v011 through v023.

## Differential tests

- `tools/vectors/gen_runway_metpy.py`: 14 airports of one to three strips with winds from every quarter at 7 to 40 kt, with and without crosswind and tailwind limits; components from MetPy (within 1e-9 kt), the full order, and the count beyond limits. No two ends are within 0.01 kt on the ranking keys
- `tools/vectors/gen_runway.py`: components and ranking written again in Python, sorting on (beyond, −headwind, crosswind, input order), at six airports including parallel runways and crosswind and tailwind limits, plus the variable-wind scenario and two refused inputs (within 1e-9)
- `core/vectors/aviation.wind.best-runway.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `best_runway_invariants`: for three strips, 16 wind directions, and three speeds, every end is listed once with the most headwind first, the two ends of a strip have opposite headwinds, the best end's headwind² + crosswind² is the wind speed², it keeps at least cos 35° of the wind, a crosswind limit above every crosswind changes nothing, and one just under the best end's crosswind puts at least one end beyond limits
- `core/crates/gp-aviation/tests/aviation.rs` `wind_invariants`: the shared runway-wind calculation through `aviation.wind.runway-components`: headwind² + crosswind² equals the wind speed² within 1e-9, and a wind mirrored about the runway gives the same components from the other side
