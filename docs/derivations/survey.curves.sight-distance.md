<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Vertical curve length for sight distance (`survey.curves.sight-distance`)

## Method

The tool computes the shortest parabolic vertical curve that preserves a supplied sight distance. A crest uses the line from an eye to an object. A sag uses the upward headlight beam. Each geometry has one equation when the sight distance lies within the curve and another when it extends beyond the curve. The tool evaluates the within-curve equation first; if that length is at least the sight distance, that case is self-consistent. Otherwise, it uses the beyond-curve equation.

## Equations

Let `C = (sqrt(h1) + sqrt(h2))²` for a crest and `C = H + S tan(beta)` for a sag.

- Sight line within the curve (`S < L`): `L = A S² / (200 C)`
- Sight line beyond the curve (`S > L`): `L = max(0, 2S - 200C / A)`
- Curve rate: `K = L / A`
- Optional design-table comparison: `Ldesign = Kdesign A`

## Symbols and units

`L` is minimum curve length, `S` is sight distance, and `A` is the absolute algebraic grade difference in percent. For a crest, `h1` is eye height and `h2` is object height. For a sag, `H` is headlight height and `beta` is the upward beam angle. All lengths in one calculation use the same unit. `K` has that length unit per percent, though the interface reports its numeric value beside the common length unit used by the calculation.

## Domain

Sight distance and grade change must be positive. Eye and headlight heights must be positive. Object height may be zero, as in the FHWA table 20 example, but cannot be negative. The upward headlight divergence must be strictly between 0° and 90°. Crest inputs require both eye and object heights; sag inputs require both headlight height and divergence. A design K is accepted only with the source the user took it from.

## Approximations

The equations are exact for the standard parabolic-curve model apart from floating-point rounding. The result is only as applicable as the supplied sight distance, heights, grade change, and design criteria. The tool does not reproduce a design table and does not check sag-curve comfort, drainage, overhead clearance, or horizontal sight distance.

## Worked example

- sourcePublisher: Federal Highway Administration
- sourceTitle: Characteristics of Emerging Road and Trail Users and Their Safety
- sourceEdition: FHWA-HRT-04-103, October 2004
- sourceLocator: Chapter 7, table 20, AASHTO bicycle row
- independent: yes
- inputs: 38.7 m sight distance, 10% grade change, 1.5 m eye height, 0 m object height
- outputs: 49.8 m minimum crest vertical-curve length
- tolerance: 0.2 m because the source prints its inputs to 0.1 m
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-10-13

The rounded source inputs give `10 × 38.7² / (200 × 1.5) = 49.923 m`, which is 0.123 m from the source's 49.8 m table value. The same chapter publishes the equation and explains that table 20 uses a 10% algebraic grade difference and a zero-height object.

## Differential tests

- `tools/vectors/gen_survey.py` independently evaluates both crest equations and both headlight-sag equations. Its 28 vectors cover both unit systems, both geometric cases, a zero-height object, an optional cited design K, all required-input failures, physical bounds, and mixed-unit refusal.
- `core/vectors/survey.curves.sight-distance.jsonl` runs those vectors through the shared core on every build.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `sight_distance_invariants` checks that scaling every length scales the result by the same factor, swapping crest eye and object heights leaves the symmetric formula unchanged, and the FHWA zero-height object remains valid.
- The same test checks that zero eye and headlight heights and a 90° headlight divergence are refused before they can produce a non-finite result.
