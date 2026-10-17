<!-- Derivation note (trust/correctness-program layer A). -->
# Rotor disk area and disk loading (`drone.power.disk-area`)

## Method

Momentum theory treats each rotor as a disk that pushes air down. The total disk area sets how much power hover takes, and the weight carried per unit of disk, the disk loading, is the usual way to compare airframes. The operation uses the same area function as `drone.power.hover-power`.

## Equations

- Area of one rotor = π × D² / 4
- Disk area A = N × π × D² / 4
- Disk loading = m × g₀ / A, with g₀ = 9.80665 m/s²

## Symbols and units

N number of rotors; D propeller diameter in inches, centimeters, or meters; m takeoff mass. Areas in m², disk loading in pascals (N/m²).

## Domain

A whole number of rotors and a positive diameter. A mass, when given, must be positive.

## Approximations

Separate, unobstructed disks. Overlapping, ducted, or coaxial rotors do not act on the full area.

## Worked example

- sourcePublisher: Embry-Riddle Aeronautical University
- sourceTitle: Introduction to Aerospace Flight Vehicles
- sourceEdition: 2026 webbook edition
- sourceLocator: Chapter 67 hover-power example, a 45,000 lb aircraft on two 38 ft rotors (each a π·(19 ft)² disk)
- independent: yes
- inputs: 2 rotors of 38 ft, 45,000 lb
- outputs: 105.36 m² per rotor, 210.73 m² total, disk loading 949.9 Pa (19.84 lb/ft²)
- tolerance: 0.1 m² on the disk area (geometry from the published 38 ft diameter)
- verifiedBy: the `focused_power_operations_match_their_general_tools` integration test, which requires the disk area and loading to equal the stable `drone.power.hover-power` tool's for the same frame
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `disk_area_vectors`: 20 airframes from a 5 in quad to a 2.4 m single rotor, with diameters in inches, centimeters, millimeters, and meters.
- `core/vectors/drone.power.disk-area.jsonl`: those vectors and three refusal cases.
- The integration suite requires the same area and loading as `drone.power.hover-power`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_power_operations_match_their_general_tools`: the disk area and disk loading equal the stable `drone.power.hover-power` tool's for the same frame.
- The area scales with the square of the diameter and linearly with the rotor count.
- Disk loading × disk area = weight.
