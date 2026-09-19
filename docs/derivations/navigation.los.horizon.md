<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Distance to the horizon (`navigation.los.horizon`)

## Method

How far the horizon lies from an observer at height h, on a sphere of the Earth's mean radius. Light bends toward the Earth, which pushes the horizon out, and the standard way to carry that is an effective radius R/(1 − k): the ray is then straight over a bigger Earth. The tool reports three, because navigators, surveyors, and radio engineers each use a different k: the geometric horizon (k = 0, no refraction), the visual horizon (k = 0.13, standard optical refraction), and the radio horizon (k = 0.25, the four-thirds Earth). Beside them it prints the rules of thumb sailors and radio operators actually use, and how far each rule sits from the modelled answer.

## Equations

- Effective radius: Re = R / (1 − k), with R = 6,371,000 m by default (the round mean radius navigators use; the radius is an input).
- Distance along the surface: d = Re · arccos(Re / (Re + h)).
- Straight-line (slant) range to the horizon: s = √(2 Re h + h²).
- Rules of thumb: 1.17√h nautical miles and 1.23√h nautical miles for radio, with h in feet; 3.57√h km with h in meters.

## Symbols and units

h height of eye above the surface (any length unit; meters canonically), k the refraction coefficient (0, 0.13, or 0.25), Re the effective radius (m), d the distance along the surface and s the slant range (shown in the chosen unit).

## Domain

Any height from the surface upward, on any radius given. The model is a sphere with a fixed refraction coefficient, so it does not know about terrain, the observer's own horizon obstructions, or the real atmosphere on the day. Refraction is a low-level effect, but the tool applies the same coefficient at any height, so far above the troposphere the visual and radio figures overstate it (at 400 km it puts the horizon 166 km beyond the geometric one); the geometric figure is the one to read there.

## Approximations

The arc formula is exact for the spherical model; only the model is approximate. Refraction is the largest error by far: k = 0.13 is a standard-atmosphere average, and real values range from about 0.07 in a well-mixed afternoon to well above 0.25 in a temperature inversion, when distant objects loom or the horizon appears to lift. The tool says so on the page rather than implying a precision it cannot have. The rules of thumb are the navigator's, and the tool prints their difference from the model (about 2.5% at any height, because 1.17√h corresponds to a slightly stronger refraction than k = 0.13).

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Bowditch, The American Practical Navigator (NGA Pub. 9), Volume II, Table 12, Distance of the Horizon
- sourceEdition: 2024 edition
- sourceLocator: The table, computed with D = 1.17 √h: 1 ft gives 1.2 nautical miles, 100 ft gives 11.7, and 820 ft gives 33.5
- independent: yes
- inputs: seven heights of eye spread across the table, in feet
- outputs: the rule's distance each row prints, in nautical miles
- tolerance: 0.05 nm, the tenth of a mile the table is printed to
- verifiedBy: golden vectors v023 to v029, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-navigation/tests/horizon_parity.rs` `the_rule_matches_bowditch_table_12`: all 126 printed heights (1 to 820 ft) through the public tool, in both the nautical and statute columns, each within the tenth of a mile it is printed to. One row is pinned as a misprint instead: the table gives 29.5 nm at 640 ft where 1.17 √640 is 29.599, and that row's own statute cell (34.1) follows the rule exactly
- `tools/vectors/gen_horizon_diff.py`: regenerates that fixture from the published PDF
- `core/vectors/navigation.los.horizon.jsonl`: 29 vectors, 22 from an independent evaluation of the arc formula in Python across five decades of height, and 7 from the table

## Invariants

- `core/crates/gp-navigation/tests/horizon_parity.rs` `horizon_invariants`: the geometric horizon is always nearer than the visual one and the visual nearer than the radio, the slant range is never shorter than the distance along the surface, the horizon always grows with height, the rule scales exactly as √h, and the arc approaches that scaling for heights small against the Earth's radius without ever exceeding it
