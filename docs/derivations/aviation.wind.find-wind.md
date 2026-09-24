<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Wind triangle: find the wind (`aviation.wind.find-wind`)

## Method

The wind triangle read for the wind. The air vector (heading, true airspeed) plus the wind vector is the ground vector (track, groundspeed), so the wind is the ground vector minus the air vector. Its direction is reported as where the wind blows from, in the same reference as the heading and track.

## Equations

- Air vector a = TAS · (sin H, cos H); ground vector g = GS · (sin T, cos T)
- Wind vector w = g − a, blowing toward atan2(w_x, w_y)
- Wind speed = |w|; wind direction (from) = atan2(−w_x, −w_y), wrapped to [0°, 360°)

## Symbols and units

H heading and T track, in degrees from north (both true or both magnetic); TAS true airspeed and GS groundspeed, in knots or another speed unit. The wind comes back in knots and degrees.

## Domain

Any heading and track, a true airspeed above zero, and a groundspeed of zero or more. A calm has no direction; when the wind comes out under 1e-9 kt its direction is given as 000, the way a report writes calm (00000KT), and a light wind's direction is sensitive to small errors in the groundspeed.

## Approximations

The wind is taken as steady over the time the track and groundspeed were measured; the answer is that period's average wind. There is no other approximation: the triangle is solved exactly with vectors.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 16, figures 16-19 to 16-22 (true course 090°, TAS 120 kt, wind 045° at 40 kt; drawn to scale as groundspeed 88 kt and true heading 076°), read the other way
- independent: yes
- inputs: heading 076°, TAS 120 kt, track 090°, groundspeed 88 kt
- outputs: wind 045° at 40 kt (the core gives 044.4° at 40.6 kt, from the drawing's rounded heading and groundspeed)
- tolerance: 1° and 1 kt (a scale drawing)
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_wind_inverse.py`: the heading, track, and groundspeed of a random wind triangle, in every quadrant, with the wind recovered by an independent vector solution in Python (within 1e-9)
- `core/vectors/aviation.wind.find-wind.jsonl`: those vectors, the published example, and the scenario cases, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `wind_triangle_readings_agree`: the heading and groundspeed from `aviation.wind.heading-groundspeed` give back the course, the airspeed, and the wind through the other three tools, within 1e-9, at four triangles
