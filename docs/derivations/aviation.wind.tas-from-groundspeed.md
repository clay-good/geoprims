<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Wind triangle: airspeed for a groundspeed (`aviation.wind.tas-from-groundspeed`)

## Method

The wind triangle read for the airspeed. The ground vector (course, groundspeed) less the wind vector is the air vector, whose length is the true airspeed to fly and whose direction is the heading. The wind correction angle is the heading less the course.

## Equations

- Ground vector g = GS · (sin C, cos C); wind vector w = WS · (sin(WD + 180°), cos(WD + 180°))
- Air vector a = g − w
- TAS = |a|, heading = atan2(a_x, a_y), wind correction angle = heading − C, in (−180°, 180°]

## Symbols and units

C course and WD the direction the wind blows from, in degrees; GS and WS in knots or another speed unit.

## Domain

Any course and wind, and a groundspeed above zero. The triangle always has a solution, so the airspeed returned should be checked against what the aircraft can fly.

## Approximations

The wind is taken as steady over the leg, and the airspeed is true airspeed, not calibrated or indicated. There is no other approximation: the triangle is solved exactly with vectors.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 16, figures 16-19 to 16-22 (true course 090°, TAS 120 kt, wind 045° at 40 kt; drawn to scale as groundspeed 88 kt and true heading 076°), read the other way
- independent: yes
- inputs: course 090°, groundspeed 88 kt, wind 045° at 40 kt
- outputs: TAS 120 kt on heading 076° (the core gives 119.7 kt and 076.3°, from the drawing's rounded groundspeed)
- tolerance: 1° and 1 kt (a scale drawing)
- verifiedBy: golden vector v009, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_wind_inverse.py`: a random course, groundspeed, and wind in every quadrant, solved independently in Python (within 1e-9)
- `core/vectors/aviation.wind.tas-from-groundspeed.jsonl`: those vectors, the published example, and the scenario cases, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `wind_triangle_readings_agree`: the heading and groundspeed from `aviation.wind.heading-groundspeed` give back the course, the airspeed, and the wind through the other three tools, within 1e-9, at four triangles
