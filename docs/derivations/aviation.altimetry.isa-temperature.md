<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ISA temperature and deviation (`aviation.altimetry.isa-temperature`)

## Method

The ICAO Standard Atmosphere defines temperature as a set of straight-line layers in geopotential altitude. A pressure altitude is, by definition, the ISA geopotential altitude with that pressure, so it is used directly as the layer altitude with no geometric conversion. The tool finds the layer that holds the altitude and evaluates its line. When an outside air temperature is given, the ISA deviation is that temperature minus the standard one.

## Equations

- T(H) = T_b + L_b · (H − H_b), for the layer with base H_b ≤ H
- Layers (H_b in m, L_b in K/m, T_b in K): 0, −0.0065, 288.15; 11,000, 0, 216.65; 20,000, +0.001, 216.65; 32,000, +0.0028, 228.65; 47,000, 0, 270.65; 51,000, −0.0028, 270.65; 71,000, −0.002, 214.65
- Below 0 m the troposphere line is extended.
- ISA deviation = OAT − T(H)
- In feet the troposphere line is 15 °C less about 1.98 °C per 1,000 ft, and the tropopause holds −56.5 °C from 36,089 ft.

## Symbols and units

H pressure altitude (geopotential), entered in feet and worked in meters (1 ft = 0.3048 m); T in K, shown in °C (T − 273.15); OAT outside air temperature in °C; deviation in °C (a temperature difference).

## Domain

Pressure altitudes from −5 km to 80 km (about −16,400 ft to 262,500 ft); outside that, OUT_OF_DOMAIN.

## Approximations

None beyond the standard atmosphere's definition; the layers are exact by definition. The ISA is a reference, and the real air at an altitude often differs from it, which is what the deviation measures.

## Worked example

- sourcePublisher: flightcondition (open-source Python library, from PyPI)
- sourceTitle: flightcondition 26.4.20, Atmosphere given a pressure
- sourceEdition: 26.4.20, run 2026-10-10
- sourceLocator: tools/vectors/gen_airspeed_fc.py: the ISA pressure of the pressure altitude goes in, and the library's standard temperature there comes out
- independent: yes
- inputs: pressure altitudes of 0, 12,000, 36,000, 45,000, and 80,000 ft, and nine more from −1,000 ft up, six with an outside air temperature
- outputs: 15.00, −8.77, −56.32, −56.50, and −52.12 °C
- tolerance: 0.0001 °C (the library inverts pressure to height itself and agrees to about 0.00002 K)
- verifiedBy: golden vectors v007 through v020, run by the core on every build
- verifiedOn: 2026-10-10

The spec's own scenario, FL410 at −56.5 °C, is the tropopause temperature the standard defines (vector v005).

## Differential tests

- `tools/vectors/gen_airspeed_fc.py`: flightcondition 26.4.20, a separately written library, at 14 pressure altitudes through four layers of the standard atmosphere; it needs the library installed and replaces only its own rows
- `tools/vectors/gen_aviation.py`: a separate Python implementation of the ICAO Doc 7488/3 layer table, evaluated at 0, 10,000, 30,000, 36,089, 41,000, and 60,000 ft (within 1e-12 relative); it uses the same defining constants, so it checks the code, not the standard
- `core/vectors/aviation.altimetry.isa-temperature.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `altimetry_invariants`: for five field elevations and six altimeter settings, the ISA temperature this tool gives at the field's pressure altitude, used as the outside air temperature in the density-altitude tool, makes the density altitude equal the pressure altitude within 1e-3 ft (a cross-check of the two tools' standard atmospheres)
- `core/crates/gp-aviation/tests/aviation.rs` `isa_temperature_fl410`: FL410 gives exactly −56.5 °C
- `core/crates/gp-aviation/tests/aviation.rs` `isa_temperature_invariants`: the temperature falls 6.5 K per kilometer to the tropopause, holds at −56.5 °C to 65,617 ft, warms above, and the deviation is the outside air temperature less the standard one
