<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Wind triangle: course from heading (`aviation.wind.course-from-heading`)

## Method

The wind triangle read forward from the heading. The air vector (heading, true airspeed) plus the wind vector gives the ground vector, whose direction is the course made good and whose length is the groundspeed. The drift angle is the course less the heading.

## Equations

- Air vector a = TAS · (sin H, cos H); wind vector w = WS · (sin(WD + 180°), cos(WD + 180°))
- Ground vector g = a + w
- Course = atan2(g_x, g_y), groundspeed = |g|, drift = course − H, in (−180°, 180°]

## Symbols and units

H heading, WD the direction the wind blows from, in degrees; TAS and WS in knots or another speed unit.

## Domain

Any heading and wind, and a true airspeed above zero. A groundspeed of zero (a wind equal and opposite to the airspeed) has no course, and is refused.

## Approximations

The wind is taken as steady over the leg. There is no other approximation: the triangle is solved exactly with vectors.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 16, figures 16-19 to 16-22 (true course 090°, TAS 120 kt, wind 045° at 40 kt; drawn to scale as groundspeed 88 kt and true heading 076°), read the other way
- independent: yes
- inputs: heading 076°, TAS 120 kt, wind 045° at 40 kt
- outputs: course 090° at 88 kt (the core gives 089.5° and 88.2 kt, from the drawing's rounded heading)
- tolerance: 1° and 1 kt (a scale drawing)
- verifiedBy: golden vector v009, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_wind_inverse.py`: a random heading, airspeed, and wind in every quadrant, solved independently in Python (within 1e-9)
- `core/vectors/aviation.wind.course-from-heading.jsonl`: those vectors, the published example, and the scenario cases, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `wind_triangle_readings_agree`: the heading and groundspeed from `aviation.wind.heading-groundspeed` give back the course, the airspeed, and the wind through the other three tools, within 1e-9, at four triangles
