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

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/power, requirement "Hover power" (disk area and disk loading); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 6 rotors of 15 in, 6.2 kg
- outputs: 0.1140 m² each, 0.6841 m² in all, 88.9 Pa
- tolerance: 1e-9
- verifiedBy: golden vector v001 and the `focused_power_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `disk_area_vectors`: 20 airframes from a 5 in quad to a 2.4 m single rotor, with diameters in inches, centimeters, millimeters, and meters.
- `core/vectors/drone.power.disk-area.jsonl`: those vectors and three refusal cases.
- The integration suite requires the same area and loading as `drone.power.hover-power`.

## Invariants

- The area scales with the square of the diameter and linearly with the rotor count.
- Area of one rotor × N = disk area.
- Disk loading × disk area = weight.
