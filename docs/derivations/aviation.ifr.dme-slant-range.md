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

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-practitioner-essentials, aviation/instrument-procedures spec, "DME geometry"
- sourceEdition: 2026
- sourceLocator: Scenario "Slant range" (DME 5.0 NM at 6,000 ft above the station gives about 4.902 NM, ±0.001 NM)
- independent: no
- inputs: DME 5.0 NM, height 6,000 ft
- outputs: ground distance 4.902 NM (4.901519873468377); slant error 0.0985 NM and elevation angle 11.39° computed from the equations above
- tolerance: 1e-9 NM and 1e-9° (the spec allows ±0.001 NM)
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_dme.py`: a Python evaluation of the same right triangle at six readings and heights (within 1e-9), plus the spec's overhead case; it restates the formula, so it checks the code's unit handling and arithmetic
- `core/vectors/aviation.ifr.dme-slant-range.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool; this is the closest test that exercises it, and it runs every vector in the tool's file, including the overhead NO_SOLUTION case, through the core and fails on any value outside its tolerance
