## Purpose

Turns a route, the winds, and an aircraft's numbers into the plan a pilot flies: a nav log leg by leg, the climb, and the points that decide whether to press on or turn back.

## ADDED Requirements

### Requirement: Navigation log
The nav log tool SHALL take waypoints (latitude and longitude rows, or true course and distance rows), TAS, a wind per leg or one wind for all legs, magnetic variation per leg or a WMM date, optional compass deviation, and fuel burn per hour. It SHALL return, per leg: true course (geodesic initial course), distance (NM), wind correction angle, true heading, magnetic heading, compass heading when deviation is given, groundspeed (kt), time en route, and fuel. It SHALL also return the totals and cumulative time and fuel. Headings and groundspeed SHALL come from the same function as `aviation.wind.heading-groundspeed`, and SHALL match it to the last bit. Headings SHALL be reported to 1°, groundspeed to 1 kt, time to 1 minute, and fuel to 0.1 of the fuel unit, with full precision in the machine result.

#### Scenario: Leg heading matches the wind triangle
- **WHEN** a leg has true course 090°, TAS 120 kt, and wind 030° at 20 kt
- **THEN** the leg's true heading is 81.7° and groundspeed 108.7 kt, equal to `aviation.wind.heading-groundspeed` on the same inputs

#### Scenario: Wind stronger than TAS
- **WHEN** a leg's wind speed exceeds the TAS
- **THEN** that leg reports `WIND_EXCEEDS_TAS` with its index, and the totals are not given

#### Scenario: Leg across the antimeridian
- **WHEN** a leg runs from 50° N 170° E to 50° N 170° W
- **THEN** the leg's course and distance are the geodesic's across the antimeridian, not the long way around

### Requirement: Climb plan
The climb plan tool SHALL return time, fuel, and ground distance to climb from field elevation to cruise altitude, from an average rate of climb (ft/min), climb TAS, the wind in the climb, and climb fuel burn. It SHALL also accept "time, fuel, and distance to climb" values read from the POH at both altitudes, and use the difference. It SHALL report top of climb as a distance along the first leg.

#### Scenario: Field at cruise altitude
- **WHEN** the field elevation equals the cruise altitude
- **THEN** time, fuel, and distance are zero, and the note says no climb is needed

### Requirement: Equal time point and point of no return
The tool SHALL compute the equal time point's distance from departure, D × GSback / (GSon + GSback). It SHALL compute the point of no return's time and distance from safe endurance E (usable fuel minus reserve, divided by burn): T = E × GSback / (GSout + GSback). Groundspeeds SHALL come from the wind triangle when wind is given. Distances SHALL be in NM to 0.1, and times to 1 minute.

#### Scenario: No wind
- **WHEN** there is no wind
- **THEN** the equal time point is exactly halfway along the leg

#### Scenario: Headwind outbound
- **WHEN** the wind is a headwind on the way out
- **THEN** the equal time point lies past the midpoint, toward the destination
