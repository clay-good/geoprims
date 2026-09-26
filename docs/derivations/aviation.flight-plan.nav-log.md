<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Navigation log (`aviation.flight-plan.nav-log`)

## Method

The planning sheet's chain, TC ± WCA = TH ± V = MH ± D = CH, worked for every leg of a route at once. Each leg's true course and distance come from the waypoints by the Karney geodesic inverse on WGS 84, or from the chart when the legs are given by course and distance. The wind correction, true heading, and groundspeed come from the same function `aviation.wind.heading-groundspeed` calls, so a leg's numbers are that tool's numbers to the last bit. The variation is the leg's own, the route's, or WMM2025 at the leg's start on a given date; the deviation is read from the card the way `aviation.wind.heading-chain` reads it. Time is distance over groundspeed, fuel is time times burn, and the totals add the legs.

## Equations

- True course TC and distance s: geodesic inverse from the leg's start to its end (initial azimuth)
- WCA = asin(W · sin(WD − TC) / TAS)
- TH = TC + WCA; GS = TAS · cos WCA − W · cos(WD − TC)
- MH = TH − V (east positive); CH = MH − D(MH), D read from the card by linear interpolation around the circle
- time = s / GS; fuel = time × burn; totals = Σ over the legs

## Symbols and units

TC, TH, MH, CH, WD, V, D in degrees (TC, TH, and WD true); TAS, W, GS in knots; s in nautical miles; time in minutes; burn in US gallons per hour, fuel in US gallons.

## Domain

Two to 100 waypoints (latitudes within ±90°, any longitudes, the antimeridian included), or 1 to 99 legs with a distance above zero. A true airspeed above zero. A leg whose crosswind is stronger than the TAS, or whose headwind is at least the TAS, has no heading, time, or fuel: it is reported with `WIND_EXCEEDS_TAS` and its index, and the route has no totals. WMM2025 variation needs waypoints and a date from 2025.0 to 2030.0.

## Approximations

Each leg is flown at cruise TAS in one steady wind, with no climb or descent. The course from waypoints is the geodesic's course leaving the waypoint; on a long leg the course changes along it, and a rhumb line would hold one course. The wind triangle is solved flat over the leg, as a flight computer does.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 16, Charting the Course and Figure 16-26 (Chickasha to Guthrie: TC 031°, TAS 115 kt, wind 360° at 10 kt, variation 7° E, deviation +2° added to the magnetic heading, checkpoint legs of 11, 10, 10.5, 13, and 8.5 NM)
- independent: yes
- inputs: the five checkpoint legs at 031°, TAS 115 kt, wind 360° at 10 kt, variation 7° E, a card of −2° (2° W, which adds 2° to the magnetic heading), 8 gal/h
- outputs: WCA 3° left, TH 28°, MH 21°, CH 23°, GS 106 kt, checkpoint times 6, 6, 6, 7, and 5 min, 53 NM in 30 min (the core gives WCA −2.57°, TH 28.43°, MH 21.43°, CH 23.43°, GS 106.31 kt, 29.9 min). The handbook's 35 min and 4.7 gal include 5 min for the climb, which the nav log leaves to the climb plan.
- tolerance: 0.5 in the printed unit (the handbook rounds to whole degrees, knots, and minutes)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-25

## Differential tests

- `tools/vectors/gen_flight_plan.py`: legs from GeographicLib's GeodSolve and the wind triangle by the law of cosines, GS = √(TAS² − (W sin d)²) − W cos d, a different form from the core's; within 1e-9
- `core/vectors/aviation.flight-plan.nav-log.jsonl`: those vectors, the published example, the spec scenario, the antimeridian leg, and the wind-exceeds-TAS case, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/flight_planning.rs` `nav_log_legs_match_heading_groundspeed_bit_for_bit`: every leg's true heading, groundspeed, and wind correction equal `aviation.wind.heading-groundspeed` on the same inputs to the last bit, over 5 winds, 12 chart courses, and 5 geodesic legs
- `core/crates/gp-aviation/tests/flight_planning.rs` `nav_log_crosses_the_antimeridian_the_short_way`: a leg from 170° E to 170° W is the short geodesic, eastbound
- `core/crates/gp-aviation/tests/flight_planning.rs` `nav_log_single_leg_and_wmm_variation`: MH = TH − the WMM variation, and the total fuel is the total time times the burn
