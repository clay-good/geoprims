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

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite wind-and-navigation spec, scenario "Ranking"
- sourceEdition: openspec/changes/add-aviation-suite/specs/aviation/wind-and-navigation/spec.md
- sourceLocator: Requirement "Best runway selection": with runways 09/27 and 18/36 and the wind 200° at 12 kt, runway 18 ranks first and every runway is listed; the components are computed
- independent: no
- inputs: runways 09/27, 18/36; wind 200° at 12 kt
- outputs: order 18, 27, 09, 36; runway 18 headwind 11.28 kt and crosswind 4.10 kt from the right; 27 headwind 4.10 kt; 09 tailwind 4.10 kt; 36 tailwind 11.28 kt
- tolerance: 1e-9 kt
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_runway.py`: components and ranking written again in Python, sorting on (beyond, −headwind, crosswind, input order), at six airports including parallel runways and crosswind and tailwind limits, plus the variable-wind scenario and two refused inputs (within 1e-9)
- `core/vectors/aviation.wind.best-runway.jsonl`: those nine vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `wind_invariants`: checks the shared runway-wind calculation through `aviation.wind.runway-components`, not this tool: at five runways and six wind angles, headwind² + crosswind² equals the wind speed² within 1e-9, and a wind mirrored about the runway gives the same components from the other side. No test checks this tool's ranking beyond its golden vectors.
- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: runs this tool's nine golden vectors, among every aviation tool's, through the registry
