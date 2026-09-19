<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Wind triangle: heading and groundspeed (`aviation.wind.heading-groundspeed`)

## Method

The E6B wind triangle, solved exactly. The air vector (TAS along the heading) plus the wind vector must lie along the desired course. The crosswind component fixes the wind correction angle, and the along-course components give the groundspeed. Courses and winds must share a reference (true or magnetic). Otherwise, a variation is required.

## Equations

- δ = wind direction − course.
- Wind correction angle: WCA = asin((W / TAS) sin δ).
- Groundspeed: GS = TAS cos WCA − W cos δ.
- Heading = course + WCA (wrapped to 0–360°).

## Symbols and units

TAS true airspeed and W wind speed (kt), course, wind direction, and heading in degrees (true by default). WCA is negative for a correction to the left.

## Domain

TAS greater than zero; any wind. When W |sin δ| > TAS the course cannot be held (NO_SOLUTION), and when GS ≤ 0 the aircraft makes no progress (NO_SOLUTION).

## Approximations

None: this is plane vector geometry over one leg, the same as the flight computer. It assumes a steady wind and TAS along the leg.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 16, figures 16-19 to 16-22 (true course 090°, TAS 120 kt, wind 045° at 40 kt; drawn to scale as GS 88 kt and true heading 076°)
- independent: yes
- inputs: course 090°, TAS 120 kt, wind 045° at 40 kt
- outputs: heading 076°, groundspeed 88 kt (the core gives 76.4° and 88.3 kt)
- tolerance: 1° and 1 kt (a scale drawing)
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: an independent Python implementation of the triangle at 20 cases, including direct headwinds and tailwinds and winds up to 80 kt (within 1e-12 relative)
- `core/vectors/aviation.wind.heading-groundspeed.jsonl`: those vectors plus the FAA drawing, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `wind_invariants`: the triangle closes. TAS along the heading plus the wind vector equals GS along the course in both east and north components (within 1e-9 kt)
