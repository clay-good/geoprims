<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# DME slant range to ground distance (`aviation.ifr.dme-slant-range`)

## Method

DME measures the straight-line distance from the station to the aircraft. Over flat ground that distance is the hypotenuse of a right triangle whose legs are the ground distance and the height above the station. The tool solves that triangle for the ground distance, reports how much the reading overstates it, and gives the angle up from the station to the aircraft.

## Equations

- h = height in ft / (1,852 / 0.3048) ft per NM
- Ground distance g = √(DME² − h²)
- Slant-range error = DME − g
- Elevation angle = asin(h / DME), or 90° when DME = 0

## Symbols and units

DME reading and ground distance in nautical miles (1,852 m); h height above the station, entered in feet (altitude minus station elevation); angle in degrees.

## Domain

DME and height zero or more. When the height exceeds the DME reading the aircraft is essentially overhead and the tool returns NO_SOLUTION.

## Approximations

Flat-earth right triangle. The Earth's curvature is ignored, which the manifest puts under 0.01 NM inside 60 NM at 10,000 ft. The answer depends on the height entered, so a wrong station elevation moves it.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: United States Standard for Terminal Instrument Procedures (TERPS)
- sourceEdition: Order 8260.3G, effective 2024-07-01
- sourceLocator: paragraph 16-4-1 and Formula 16-14-7
- independent: yes
- inputs: DME reading 5 NM, height 13,000 ft above the NAVAID
- outputs: 4.52 NM horizontally; the tool gives 4.5191 NM before rounding, a 0.4809 NM slant error, and a 25.33° elevation angle
- tolerance: 0.1% relative on horizontal distance, covering the source's hundredth-nautical-mile rounding; 1e-9 on the computed companion outputs
- verifiedBy: golden vector v008
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_dme.py`: the FAA's printed 5 NM/13,000 ft example plus 18 successful right-triangle cases evaluated separately in Python, followed by three refusal cases. The generated cases cover varied readings, heights, equivalent units, zero height, and overhead; they restate the formula, so only the FAA row is an independent check
- `core/vectors/aviation.ifr.dme-slant-range.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `dme_slant_range_invariants`: the distances close the right triangle, ground distance plus slant error recovers the reading, and the reported angle recovers the height ratio; scaling both legs scales both distances but not the angle, equivalent metric and aviation units agree, and zero height has zero slant error
