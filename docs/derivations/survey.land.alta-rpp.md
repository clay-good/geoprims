<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ALTA/NSPS relative positional precision (`survey.land.alta-rpp`)

## Method

The 2026 ALTA/NSPS standards define Relative Positional Precision (RPP) as the semi-major axis of the 95 percent error ellipse of the line between adjacent boundary corners, and cap it at a constant plus a share of the distance between them. The tool computes that allowable value for a distance and, when given the ellipse, says whether it is within, near, or beyond the allowable. The ellipse comes from the surveyor's least-squares adjustment, either as its semi-major axis or as the standard errors and covariance of the corners' coordinate difference. A traverse misclosure is refused, because the standard judges RPP from the ellipse, not from a closure.

## Equations

- Allowable: 2 cm + 50 × 10⁻⁶ × distance when the distance is metric; 0.07 ft + 50 × 10⁻⁶ × distance when it is in feet, as the standard writes it. (0.07 ft is 2.13 cm, so the two forms differ by 1.3 mm; the tool uses the one matching the unit entered.)
- From a covariance: the larger eigenvalue λ = (σe² + σn²)/2 + √(((σe² − σn²)/2)² + c²), and the 95% semi-major axis = 2.447746830680816 × √λ.
- Status: the ellipse against the allowable, within, near (inside the shared near margin), or beyond.

## Symbols and units

Distance and the ellipse take any length unit (feet by default). σe and σn are the standard errors of the east and north coordinate differences, and c their covariance in the distance's unit squared. 2.4477 is √(−2 ln 0.05), the 95% scale for two dimensions. The allowable is reported in feet by default.

## Domain

Distance from 0 to 100 km. The ellipse must be zero or larger. A covariance larger than the two variances allow (c² > σe² σn²) is INVALID_INPUT, as is a mix of the semi-major axis with σ values, or a misclosure.

## Approximations

None in the allowable: it is the standard's own formula. The 95% scale assumes the covariance is known, not estimated from few redundant observations, which is what the standard's "95 percent confidence level" is commonly taken to mean. The tool does not judge the adjustment that produced the ellipse.

## Worked example

- sourcePublisher: American Land Title Association and National Society of Professional Surveyors
- sourceTitle: Minimum Standard Detail Requirements for ALTA/NSPS Land Title Surveys
- sourceEdition: 2026 (effective February 23, 2026), official PDF, read 2026-10-09
- sourceLocator: Section 3.E.v: "The maximum allowable Relative Positional Precision for an ALTA/NSPS Land Title Survey is 2 cm (0.07 feet) plus 50 parts per million (based on the direct distance between the two corners being tested)"; Section 3.E.i defines RPP as the semi-major axis of the error ellipse at the 95 percent confidence level
- independent: yes
- inputs: distance 1,000 ft; and distance 1,000 m
- outputs: allowable 0.07 + 0.05 = 0.12 ft; and 0.02 + 0.05 = 0.07 m
- tolerance: 1e-9 ft
- verifiedBy: golden vectors v001 and v019, run by the core on every build
- verifiedOn: 2026-10-09

The standard gives the constant in both units and the 50 ppm; the allowable at a distance is that formula.

## Differential tests

- `tools/vectors/gen_gnss.py`: the allowable and the covariance ellipse worked again in Python at four cases, and a refused misclosure
- `core/vectors/survey.land.alta-rpp.jsonl`: those vectors, and 15 that hold the allowable to the standard's formula at distances from 0 to 10,000 ft and 0 to 2,000 m, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/land.rs` `alta_rpp_invariants`: the allowable is the constant plus 50 ppm in feet and in meters, no verdict is given without an ellipse, an uncorrelated covariance gives 2.4477 times the larger sigma, and the status follows the ellipse against the allowable
