<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Level run reduction and closure (`survey.reduction.level-run`)

## Method

Differential leveling notes are reduced row by row. The height of instrument is the elevation of the point sighted back to plus the backsight, and each point sighted forward is the height of instrument less its foresight. The arithmetic check compares the sum of the backsights less the sum of the foresights with the change in elevation from first to last, which catches slips in the column arithmetic. When the run ends on a benchmark of known elevation, or loops back to the start, the misclosure is the observed closing elevation less the known one. It is distributed back along the run in proportion to cumulative distance, or evenly by setups when no distances are given, and compared with the allowable C√K.

## Equations

- HI = elevation + BS; elevation = HI − FS (or HI − IS for a side shot)
- Check: ΣBS − ΣFS = last elevation − first elevation
- Misclosure e = observed closing elevation − known closing elevation
- Adjusted elevation at a point = elevation − e × (distance to the point ÷ total distance), or × (setups to the point ÷ total setups)
- Allowable = C × √K, with K the run's length in kilometers

## Symbols and units

BS backsight, FS foresight, and IS intermediate (side) sight, rod readings in feet or meters; HI and elevations in the same unit; C the allowable constant in that unit, per √km.

## Domain

A book that starts with a backsight on the starting benchmark and alternates foresights and backsights through turning points. A book without that shape is refused and the problem row named.

## Approximations

Balanced backsight and foresight distances are assumed to cancel curvature, refraction, and collimation error. The distribution treats the misclosure as having built up steadily along the run; it is not a least-squares adjustment and does not locate a blunder.

## Worked example

- sourcePublisher: Indiana Department of Transportation
- sourceTitle: Survey Procedures, chapter 2, Use and Care of Level
- sourceEdition: retrieved 2026-09-24
- sourceLocator: Figure 2-4, closed form level notes: benchmark A (820.00) to benchmark K (831.15) through three turning points
- independent: yes
- inputs: start 820.00 ft; BS 8.42, 11.56, 6.15, 4.39; FS 1.20, 1.35, 10.90, 5.94; known closing elevation 831.15 ft
- outputs: elevations 827.22, 837.43, 832.68, and 831.13; ΣBS 30.52 and ΣFS 19.39; misclosure −0.02 ft
- tolerance: 0.005 ft (the notes' rounding)
- verifiedBy: golden vector v007, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_survey.py`: the notes reduced by hand-style arithmetic in Python for a loop, a run, runs with and without distances, and 14 random books in feet and meters, with the adjustment computed independently (within 1e-10 relative)
- `core/vectors/survey.reduction.level-run.jsonl`: those vectors, the published notes, the allowable, and a refused book, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `level_run_invariants`: raising the starting and closing elevations by a constant raises every elevation and adjusted elevation by it and leaves the misclosure alone; the arithmetic check holds; the adjusted closing benchmark is the known one; and a book that closes exactly is left unadjusted
