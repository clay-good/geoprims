<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Slope stakes (`survey.earthwork.slope-stake`)

## Method

On one side of the centerline, the design surface runs from the shoulder, at the design elevation and the half width out, down or up the side slope. It goes up at the cut slope where the ground at the shoulder is above grade, and down at the fill slope where it is below. The ground is the line through the shots given, straight between them. The catch point is where the two meet. It is found by bracketing a sign change segment by segment outward from the shoulder, then refined with Brent's method. The stake is written as the cut or fill at the catch and its offset, as in C 5.0 / 39.0 R.

## Equations

- Design in cut: z(x) = G + (x − w) / s_cut; in fill: z(x) = G − (x − w) / s_fill
- Ground: straight between shots (x_i, y_i)
- Catch: the first x ≥ w where ground(x) = z(x); cut or fill = |ground(x) − G|

## Symbols and units

G the design (shoulder) elevation and w the half width, in feet or meters; s the slope as horizontal per vertical (2 for 2:1); offsets from the centerline on the chosen side.

## Domain

Ground shots from the shoulder outward past the catch point. A slope that never meets the ground within the shots is refused (DID_NOT_CONVERGE), and a missing slope for the case that applies is refused by name.

## Approximations

The ground is straight between shots, so a break between two shots is invisible. The template is one straight slope from the shoulder on each side, with no rounding, benches, or ditch.

## Worked example

- sourcePublisher: Indiana Department of Transportation
- sourceTitle: Survey Procedures, chapter 6, Slope Stakes
- sourceEdition: retrieved 2026-09-24
- sourceLocator: The worked example with figure 6-2: left side, control elevation 499.0, standard distance 22 ft, HI 497.5, ground rod 3.0 at both trials, 3:1 fill; right side, control elevation 497.0, standard distance 29 ft, HI 503.5, ground rod 1.5 at both trials, 2:1 cut
- independent: yes
- inputs: left, grade 499.0 ft, half width 22 ft, ground at 494.5 ft (HI less the rod); right, grade 497.0 ft, half width 29 ft, ground at 502.0 ft
- outputs: fill 4.5 ft at 35.5 ft left (F 4.5 / 35.5 L); cut 5.0 ft at 39 ft right (C 5.0 / 39.0 R)
- tolerance: 0.05 ft (the example's rounding)
- verifiedBy: golden vectors v007 and v008, run by the core on every build
- verifiedOn: 2026-09-24

The example reads the same rod at both trial offsets on each side, so the ground there is level; the vectors give it level from the shoulder out, which the answer does not depend on beyond the catch.

## Differential tests

- `tools/vectors/gen_survey.py`: catch points solved in closed form, segment by segment, for the spec scenario, a break in the ground, and 13 random grounds with two to four breaks, both sides, and slopes from 1.5:1 to 4:1 (within 1e-6 relative)
- `core/vectors/survey.earthwork.slope-stake.jsonl`: those vectors, the INDOT example, and the refusals, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `slope_stake_invariants`: raising the grade and the ground together leaves the catch and the depth alone; the catch lies on the design slope; and the left and right stakes differ only in their side letter
