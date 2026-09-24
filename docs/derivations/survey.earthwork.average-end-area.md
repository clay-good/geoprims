<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Average end area volume (`survey.earthwork.average-end-area`)

## Method

The average end area method: the volume between two parallel cross-sections is the mean of their areas times the distance between them. Each span is computed on its own, and a job's total is the sum of its spans, with cut and fill kept apart. The result is given in cubic yards and cubic feet from any area and length units.

## Equations

- V = L · (A₁ + A₂) / 2
- Cubic yards = cubic feet / 27; 1 yd³ = 0.9144³ m³ exactly

## Symbols and units

A₁ and A₂ are the end areas (ft², m², or another area unit) and L is the distance between the sections (ft, m, or another length). V is reported in yd³ and ft³.

## Domain

Areas of zero or more and a length greater than zero. A zero end area is allowed (a cut or fill running out); a negative area or a zero length is refused.

## Approximations

The method assumes the section changes linearly between the ends. That is exact for a prism, but it overstates a pyramid-like span, where one end is much smaller than the other, by up to half the prismoidal correction. `survey.earthwork.prismoidal` uses a measured middle section to remove that error.

## Worked example

- sourcePublisher: LibreTexts Engineering
- sourceTitle: Fundamentals of Transportation, section 7.3 Earthwork, Example 1 (computing volume)
- sourceEdition: retrieved 2026-09-24
- sourceLocator: Sections at 0, 50, 100, and 150 m with areas 40, 42, 19, and 34 m²: spans of 2,050, 1,525, and 1,325 m³, 4,900 m³ in all
- independent: yes
- inputs: 40 and 42 m² sections 50 m apart (and the next two spans)
- outputs: 2,050 m³ (2,681.3 yd³), 1,525 m³, and 1,325 m³
- tolerance: 1e-12 relative (the published values are exact)
- verifiedBy: golden vectors v006 to v008, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_survey.py`: an independent Python evaluation at ten random spans in square feet and two in square meters, including two zero end areas (within 1e-11 relative)
- `core/vectors/survey.earthwork.average-end-area.jsonl`: those vectors, the published example, the spec scenario, and the zero-length and negative-area refusals, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `average_end_area_invariants`: the end areas may be swapped, the volume scales with the length, meters and feet give the same answer, and splitting a span at a section whose area is the average of its ends gives the same total
