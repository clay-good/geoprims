<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Climb to cruise: time, fuel, and distance (`aviation.performance.climb-plan`)

## Method

Two ways, as a pilot plans a climb. From averages: the height to climb over the average rate of climb gives the time; the climb true airspeed through the wind triangle of `aviation.wind.heading-groundspeed` gives the groundspeed; groundspeed times time is the distance to the top of climb, and burn times time is the fuel. From the POH: the "time, fuel, and distance to climb" table is read at the field and at cruise altitude, and the field's values are subtracted from the cruise values, as the POH instructs. The top of climb is the distance along the first leg from departure.

## Equations

- Height h = cruise altitude − field elevation
- Averages: t = h / ROC; GS = TAS · cos WCA − W · cos(WD − course); d = GS · t; fuel = burn · t
- Table: t = t(cruise) − t(field); fuel = fuel(cruise) − fuel(field); d = d(cruise) − d(field)
- Fuel with taxi = taxi fuel + climb fuel

## Symbols and units

h in feet; ROC in feet per minute; TAS, W, GS in knots; course and wind direction in degrees true; t in minutes; d in nautical miles; burn in US gallons per hour and fuel in US gallons.

## Domain

A cruise altitude at or above the field (below it is a descent, refused with a pointer to the top-of-descent tool). A rate of climb and a climb TAS above zero, or two table rows with every cruise value at least the field's. A wind needs a course. When the field is at the cruise altitude, time, fuel, and distance are zero and the note says no climb is needed.

## Approximations

One average rate of climb and one wind for the whole climb; the rate really falls with height. The POH's distances are for zero wind and standard temperature; the tool reports the table's difference and leaves the POH's temperature and wind notes to the pilot.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 11, Figure 11-25 (fuel, time, and distance to climb chart), read at 6,000 ft (3.5 gal, 6 min, 9 NM) and at 10,000 ft (6 gal, 10.5 min, 15 NM)
- independent: yes
- inputs: field 6,000 ft, cruise 10,000 ft, table rows 6 min, 3.5 gal, 9 NM and 10.5 min, 6 gal, 15 NM
- outputs: 2.5 gal and 6 NM, as printed; 4.5 min (the handbook's text says 4 minutes, but its own readings of 6 and 10.5 minutes subtract to 4.5)
- tolerance: 1e-9 (a subtraction of the printed readings)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-25

## Differential tests

- `tools/vectors/gen_flight_plan.py`: climbs by averages with the groundspeed from the law-of-cosines wind triangle, a different form from the core's; within 1e-9
- `core/vectors/aviation.performance.climb-plan.jsonl`: those vectors, the published example, the field-at-cruise scenario, and the refused cases, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/flight_planning.rs` `climb_plan_groundspeed_is_the_wind_triangles`: the climb's groundspeed equals `aviation.wind.heading-groundspeed` to the last bit, and the distance is groundspeed times time
- `core/crates/gp-aviation/tests/flight_planning.rs` `climb_plan_field_at_cruise_needs_no_climb`: zero time, fuel, and distance, with the note
