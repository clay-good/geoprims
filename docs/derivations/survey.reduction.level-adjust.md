<!-- Derivation note (trust/correctness-program layer A). -->
# Level run adjustment (`survey.reduction.level-adjust`)

## Method

A level run that closes on a known elevation, either back on its starting benchmark or on a second benchmark, shows its accumulated error as a misclosure. The usual adjustment assumes that error grew steadily along the run and gives each turning point a share in proportion to how far along the run it is. The operation reduces the level book, requires a closure, and reports the misclosure and the adjusted elevations. It shares one calculation path with `survey.reduction.level-run`.

## Equations

- HI = elevation + BS; elevation = HI − FS.
- Misclosure m = observed closing elevation − known closing elevation.
- Adjusted elevation at station i = observed elevation − m × (Dᵢ / D), with Dᵢ the cumulative distance to the station and D the total.
- Without distances, Dᵢ / D is the count of setups to the station over the total setups.
- Allowable closure = C × √K, K the run length in kilometers.

## Symbols and units

BS backsight, FS foresight, HI height of instrument, C the allowable constant for the chosen standard. All lengths share one unit: meters, international feet, or US survey feet.

## Domain

At least two stations, starting with a backsight and ending with a foresight. The run must close: give the closing benchmark's elevation, or end on the station it started from. An open run is refused, since it has no misclosure. The allowable needs a distance on every leg.

## Approximations

The proportional distribution is the conventional one for a single line or loop, not a least-squares adjustment. It assumes error accumulates evenly with distance, or with setups. Side shots are reduced but not adjusted. Curvature and refraction are taken as cancelled by balanced sights.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/instrument-reductions, requirement "Differential leveling"; the arithmetic is checked by hand, not taken from a published table
- independent: no
- inputs: BM A 250.000 m; BS 1.234, FS 0.987 and BS 2.110 at 120 m, FS 3.004 and BS 0.542 at 95 m, FS 1.876 at 150 m; BM B 247.975 m
- outputs: observed 250.247, 249.353, 248.019 m; misclosure +0.044 m; adjusted 250.2325, 249.3271, 247.975 m
- tolerance: 1e-9 m
- verifiedBy: golden vector v001 and the `level_adjustment_matches_the_level_run` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `level_adjust`: 18 random runs of 2 to 8 legs in all three length units, alternating loops with benchmark-to-benchmark runs, with and without leg distances, reduced by a separate Python accumulation.
- `core/vectors/survey.reduction.level-adjust.jsonl`: those vectors, two hand-checked examples, and three refusal cases.
- The integration suite requires the same misclosure, stations, sums, and check as `survey.reduction.level-run`.

## Invariants

- The adjusted closing elevation equals the known one, and the starting elevation does not move.
- Each station's correction is the misclosure times its share of the run.
- A run with no misclosure is returned unchanged.
