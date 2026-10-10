<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Lidar flight plan (`drone.sensors.lidar-plan`)

## Method

A scanning lidar spreads its pulses across a swath set by its field of view and the height above ground, and along the track at the aircraft's speed. The pulses per square meter are the pulse rate divided by the area swept each second. Overlapping lines add to it. The tool gives the swath, the line spacing for a side overlap, the density for one line and for all lines together, the point density for an average number of returns, and the USGS 3DEP quality level whose minimum the aggregate density meets.

## Equations

- Swath = 2 × height × tan(FOV ÷ 2).
- Line spacing = swath × (1 − side overlap).
- Pulse density, one line = pulse rate ÷ (speed × swath).
- Aggregate density = pulse rate ÷ (speed × line spacing).
- Point density = aggregate density × returns per pulse.
- Quality level: QL1 when the aggregate density is at least 8.0 pulses per m², QL2 at least 2.0, QL3 at least 0.5, otherwise below QL3.

## Symbols and units

Pulse rate in hertz (kHz by default), FOV the full scan angle across track in degrees, height above ground in meters, speed over the ground in m/s, side overlap in percent of the swath (0 by default), returns the average recorded per pulse (1 by default). Densities are per square meter.

## Domain

Pulse rate, height, and speed above zero; field of view above 0° and under 180°.

## Approximations

The densities are nominal averages over flat ground with every pulse returned. Oscillating and non-repetitive scan patterns crowd pulses toward the swath edges or center, and water, dark surfaces, and range limits drop returns. A USGS quality level also sets accuracy requirements that density alone does not meet, and QL0 has QL1's density with tighter accuracy, so the level named is a density check, not a compliance finding.

## Worked example

- sourcePublisher: U.S. Geological Survey, 3D Elevation Program
- sourceTitle: Lidar Base Specification, tables
- sourceEdition: 2025 rev. A, read on the USGS tables page 2026-10-09
- sourceLocator: Table 1, aggregate nominal pulse spacing and density: QL0 and QL1 at least 8.0 pulses per m² (spacing at most 0.35 m), QL2 at least 2.0 (0.71 m), QL3 at least 0.5 (1.41 m)
- independent: yes
- inputs: plans whose aggregate density lands just above and just below each minimum: 8.4 and 7.6, 2.1 and 1.9, 0.55 and 0.45 pulses per m²
- outputs: QL1 and QL2; QL2 and QL3; QL3 and below QL3
- tolerance: exact for the level; 1e-9 for the densities
- verifiedBy: golden vectors v007 through v020, run by the core on every build
- verifiedOn: 2026-10-09

The table gives the minimums. The densities themselves are the geometry above, worked again in Python; the spec's own scenario, 240 kHz with a 70° field of view at 100 m and 10 m/s, gives a 140.0 m swath and 171 pulses per m² (vector v001).

## Differential tests

- `tools/vectors/gen_sensing.py`: the swath, spacing, and densities worked again in Python for five plans, 14 more placed either side of each USGS minimum, and a refused 180° field of view
- `core/vectors/drone.sensors.lidar-plan.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/planning.rs` `lidar_plan_invariants`: density halves when speed or height doubles, side overlap raises only the aggregate, points are pulses times returns, and the quality level turns over at 8, 2, and 0.5 pulses per m²
