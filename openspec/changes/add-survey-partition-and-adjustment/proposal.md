## Why

Three survey office jobs still need a desktop package or a spreadsheet after the survey suite: cutting a parcel into pieces of a required area, fitting a 3D transformation between two sets of control, and adjusting a leveling network with more than one loop. Each has a textbook method with published worked examples, each is asked for daily, and each is easy to get subtly wrong by hand (a cutoff line computed from the wrong side, a rotation sign flipped, misclosure distributed without weights).

Depends on: `add-survey-suite` (COGO, areas, leveling), `add-practitioner-essentials` (2D localization), `add-geodesy-suite` (Helmert conventions).

## What Changes

Three operations:

| Tool | Question it answers | Source |
|---|---|---|
| `survey.cogo.area-partition` | Where does the line go that cuts off exactly A acres: parallel to a side, through a fixed corner, or between two chosen sides? | Ghilani & Wolf, *Elementary Surveying*, 16th ed., chapter 12 (Area), the partitioning sections |
| `survey.cogo.helmert-3d-fit` | What 3D conformal (7-parameter, Bursa-Wolf) transformation best fits these common points, and how well? | Ghilani, *Adjustment Computations*, 6th ed., the 3D conformal coordinate transformation chapter; IOGP Guidance Note 7-2 §4.3 for the conventions |
| `survey.reduction.level-network` | Given height differences over several loops with lengths or setups, what are the adjusted elevations and their standard errors? | Ghilani, *Adjustment Computations*, 6th ed., the differential leveling adjustment chapter |

All three show the full computation (the quadratic for the cutoff, the normal equations, the residuals) and export a calculation sheet.

## Capabilities

### New Capabilities

- `survey/partition-and-adjustment`: Area partitioning, 3D conformal fit from control, and least-squares leveling networks.

### Modified Capabilities

None.

## Non-goals

- Parcels with curved sides (circular-arc boundaries) in the partition tool; the result says so and refuses them.
- Full 2D/3D network adjustment of angles and distances beyond the existing experimental `least-squares-2d`.
- Legal subdivision design. The tool computes geometry from the figure it is given.

## Impact

- `core/gp-survey` grows by three operations and a small dense linear-algebra helper (Cholesky on normal equations, already used by `least-squares-2d`).
- New sources-ledger rows: Ghilani, *Adjustment Computations*, 6th ed. (2017); IOGP 373-7-2 is already `iogp-g7-2`.
