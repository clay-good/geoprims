## Purpose

Answers the questions between a drone mission's path and flying it: how many batteries, where to swap them, whether the wind allows it, whether the pilot can see the aircraft, and where to put ground control.

## ADDED Requirements

### Requirement: Sorties and battery swaps
Given a waypoint path, a home point, usable flight time (or battery energy and power), cruise and transit groundspeed, the reserve, and an optional return headwind, the tool SHALL split the path into sorties. Each sortie SHALL end at the last waypoint from which the trip home, computed as in `drone.power.rth-budget`, still lands within the reserve. The tool SHALL return the sorties (first and last waypoint, flying time, return time and energy), the swap points, and the battery count. Times SHALL be to 0.1 minute.

#### Scenario: Return trip grows with distance
- **WHEN** a grid's far end is 2 km from home and each battery gives 20 minutes
- **THEN** sorties that end far from home are shorter than those that end near it, and each one's return time is shown

#### Scenario: A leg that cannot be flown
- **WHEN** a single waypoint lies beyond the out-and-back range of a full battery
- **THEN** the tool reports `WAYPOINT_OUT_OF_RANGE` with that waypoint's index and distance

### Requirement: Wind against the drone's limit
Given the reported wind (speed, gust, report height, 10 m by default), the flying height, the terrain class, and the drone's wind rating (or maximum airspeed and a margin), the tool SHALL estimate the wind at flying height with the power law and the exponent for the terrain class. It SHALL name the exponent in the result, and return the sustained wind and gust at height, the margin to the rating, and the groundspeed into the wind. It SHALL warn that gusts near buildings, trees, and terrain are not modeled.

#### Scenario: At the report height
- **WHEN** the flying height equals the report height
- **THEN** the wind at height equals the reported wind

#### Scenario: Gust over the rating
- **WHEN** the gust at height exceeds the drone's rating while the sustained wind does not
- **THEN** the result reports `GUST_EXCEEDS_RATING` and states both values

### Requirement: Visual line of sight over a mission
Given the pilot's position, a mission's waypoints, and a visual range (entered, or from `drone.sensors.vlos`), the tool SHALL return the geodesic distance to the farthest waypoint, the count and indexes of waypoints beyond the range, and a map layer of the range ring and those waypoints. It SHALL cite 14 CFR 107.31 and state that visual line of sight is judged on the day, not guaranteed by a distance.

#### Scenario: Waypoint on the ring
- **WHEN** a waypoint lies exactly at the visual range
- **THEN** it counts as inside

### Requirement: Ground control and checkpoints
Given an area polygon and an accuracy class, the tool SHALL return the checkpoint count from the ASPRS Positional Accuracy Standards, Edition 2, for the product area. It SHALL also return a suggested GCP count and layout as lat/lon rows (the polygon's corners plus interior points at a stated spacing), labeled a rule of thumb and cited. Every suggested point SHALL lie inside the polygon.

#### Scenario: Concave area
- **WHEN** the area is L-shaped
- **THEN** no suggested point lies in the notch outside the polygon
