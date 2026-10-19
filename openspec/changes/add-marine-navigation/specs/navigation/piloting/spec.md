## Purpose

Does the chart-table arithmetic of piloting: the current triangle, dead reckoning, fixes from bearings and a run, distance off by vertical angle, and compass conversion with deviation.

## ADDED Requirements

### Requirement: Set and drift
The set-and-drift tool SHALL solve, by selection, the three current-triangle problems of Bowditch §907–908: (1) course and speed with set and drift give track and speed made good; (2) required course made good, set and drift, and speed give course to steer and speed made good; (3) required course and speed made good with set and drift give course to steer and speed. It SHALL use the same vector-triangle function as the aviation wind tools and SHALL return `NO_SOLUTION` when the current exceeds the ship's speed in problem 2 or 3. The diagram SHALL draw the triangle with the current vector labeled.

#### Scenario: Track and speed made good
- **WHEN** course 080°, speed 10 kn, set 140°, drift 2 kn
- **THEN** the track is 089° and speed made good 11.2 kn (within 0.5° and 0.1 kn of Bowditch §908 example 1, with exact values in the machine result)

#### Scenario: Course to steer
- **WHEN** the required course made good is 095°, set 170°, drift 2.5 kn, speed 12 kn
- **THEN** the course to steer is 083.5° and speed made good 12.4 kn (Bowditch §908 example 2, same tolerance)

#### Scenario: Current too strong
- **WHEN** the required speed made good is 3 kn directly into a 5 kn current
- **THEN** the result is `NO_SOLUTION`

### Requirement: Dead reckoning and running fix
The dead-reckoning tool SHALL advance a position by course and speed over a time (or distance) on the Mercator plane for runs under 100 nm, reporting the rhumb-line position for longer runs and `LIMIT_EXCEEDED` above 600 nm. The running-fix tool SHALL advance an earlier line of position by the run between bearings and intersect it with the later bearing, returning the fix, the run, and the two lines of position for the map.

#### Scenario: Running fix
- **WHEN** a light bears 024° at 1130 and 359° at 1140 while the ship makes 050° at 15 kn
- **THEN** the fix is the intersection of the first bearing advanced 2.5 nm along 050° with the second bearing, drawn on the map

### Requirement: Fix by two bearings on the bow
The two-bearing-fix tool SHALL compute distance off at the second bearing and at the beam from two relative bearings and the run between them (Bowditch §1021, Table 18), with named presets for doubling the angle on the bow (distance off equals the run) and the bow-and-beam bearing. It SHALL state that the method assumes no current.

#### Scenario: Doubling the angle
- **WHEN** relative bearings of 30° and 60° are observed 4.0 nm apart
- **THEN** the distance off at the second bearing is 4.0 nm and the predicted distance abeam is 3.46 nm (±0.01 nm)

### Requirement: Distance by vertical angle
The tool SHALL compute distance in nautical miles from the dip-corrected sextant angle a of an object of charted height H above the water, observed from height of eye h (feet), by Bowditch Appendix B: D = √(tan²a / 0.0002419² + (H − h) / 0.7349) − tan a / 0.0002419, with the object-beyond-the-horizon variant. It SHALL accept meters with conversion and SHALL warn when the object's waterline is beyond the horizon.

#### Scenario: Table 16 entry
- **WHEN** H − h = 100 ft and the corrected angle is 0°30'
- **THEN** the distance matches Bowditch Table 16's entry within 0.05 nm

### Requirement: Compass conversion (TVMDC)
The compass-conversion tool SHALL convert among true, magnetic, and compass directions with variation and deviation (east and west as Bowditch signs them), report compass error, interpolate deviation from a pasted deviation table by magnetic heading, and convert relative bearings. It SHALL share one function with `aviation.wind.heading-chain`. Variation MAY come from the magnetic tools at a date, with the chart's printed variation preferred when given.

#### Scenario: Bowditch worked row
- **WHEN** true heading 215°, variation 7° W, deviation 1.5° W
- **THEN** magnetic heading is 222°, compass heading 223.5°, and compass error 8.5° W (Bowditch Vol. II §502)

#### Scenario: Compass bearing to true
- **WHEN** a compass bearing of 306.5° is converted with the same error
- **THEN** the magnetic bearing is 305° and the true bearing 298°, and the relative bearing from heading 215° is 083°
