<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Angular closure of a loop traverse (`survey.cogo.angular-closure`)

## Method

The tool parses each measured traverse angle, computes the geometric sum for a closed polygon, and subtracts that required sum from the measured sum. A positive misclosure means the measurements total too much. It removes the misclosure in equal shares, which is the stated model for angles observed with equal weight. When the user enters a standard's coefficient `K`, the tool also compares the absolute misclosure with `K√n`.

## Equations

- Interior required sum: `S = (n − 2) × 180°`
- Reflex-exterior required sum: `S = (n + 2) × 180°`
- Misclosure: `f = Σθᵢ − S`
- Equal correction per angle: `c = −f / n`
- Adjusted angle: `θᵢ′ = θᵢ + c`
- Allowable magnitude: `A = K√n`

The result is within the entered allowable when `|f| ≤ A`.

## Symbols and units

`n` is the number of angles, `θᵢ` is one measured angle, `S` is the geometric sum, `f` is the signed angular misclosure, `c` is the equal correction, `K` is the user-entered allowable coefficient in arcseconds, and `A` is the allowable misclosure in arcseconds. Inputs accept decimal degrees or degrees-minutes-seconds text. Misclosure, correction, and allowable outputs use arcseconds; angle sums and adjusted angles use degrees-minutes-seconds.

## Domain

A loop needs 3 through 500 angles. Every angle must be greater than 0° and less than 360°. The default convention is interior angles. The exterior option means the reflex angle at each vertex, whose polygon sum is `(n + 2) × 180°`; it does not mean the smaller turning angle whose sum is 360°. `K` is optional and may range from 0 through 3,600 arcseconds.

## Approximations

The equal correction assumes every angle has the same weight. It is not a least-squares adjustment and cannot identify a blunder. The displayed adjusted angles are rounded to 0.1 arcsecond, so their displayed sum can differ from the exact required sum by accumulated display rounding. The allowable is not selected by the tool: the survey's governing contract, agency, or jurisdiction determines `K`. Washington's boundary-survey rule, for example, uses `10″√n` or `30″√n` for the two linear-closure classes it names.

## Worked example

- sourcePublisher: Walla Walla University
- sourceTitle: Surveying - Traverse Closure
- sourceEdition: June 27, 2019
- sourceLocator: Section 3.1 and table 1
- independent: yes
- inputs: seven interior angles of 128.5°, 129°, 211°, 70.5°, 80.5°, 194°, and 89.5°
- outputs: 903° measured sum, 900° required sum, 3° misclosure, and a −0.42857° equal correction
- tolerance: each adjusted angle agrees within 0.0005°, half the source table's 0.001° display unit
- verifiedBy: golden vector v007 and `core/crates/gp-survey/tests/survey.rs`
- verifiedOn: 2026-10-13

The tool returns a +10,800 arcsecond misclosure and a −1,542.857142857 arcsecond correction. Its seven adjusted angles agree with every value in the published table at the table's printed precision.

## Differential tests

- `tools/vectors/gen_survey.py`: 25 vectors independently evaluate interior and reflex-exterior sums, polygon sizes, decimal and DMS notation, positive and negative misclosures, exact/inside/outside allowable boundaries, zero and maximum `K`, omitted `K`, and invalid angles and list sizes

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `angular_closure_matches_the_walla_walla_worked_example`: reproduces the published sum, correction, and seven adjusted angles
- `core/crates/gp-survey/tests/survey.rs` `angular_closure_adjusted_angles_reach_the_geometric_sum`: adjusted angle sums reach the required polygon sum after display rounding
- `core/crates/gp-survey/tests/survey.rs` `angular_closure_equal_bias_is_removed_by_the_adjustment`: adding the same bias to every observation leaves the adjusted angles unchanged
- `core/crates/gp-survey/tests/survey.rs` `angular_closure_reflex_exteriors_complement_interiors`: corresponding adjusted interior and reflex-exterior angles sum to 360°
- `core/crates/gp-survey/tests/survey.rs` `angular_closure_allowable_boundary_and_invalid_row_are_explicit`: equality is within the allowable, and a bad row identifies its exact field
