<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Height per hectopascal and per inch of mercury (`aviation.altimetry.pressure-per-height`)

## Method

In still air, pressure falls with height because each layer carries the weight of the air above it. The hydrostatic equation gives the rate, and the ideal gas law turns the air's density into its pressure and temperature, so the height one unit of pressure spans depends only on the temperature and the pressure there. The tool takes the pressure from the ICAO Standard Atmosphere at the given pressure altitude (that is what a pressure altitude means) and the temperature from the outside air temperature when one is given, or the standard temperature otherwise. It reports the height for one hectopascal and for one inch of mercury, beside the 27 ft per hPa rule of thumb and that rule's error.

## Equations

- Hydrostatic balance: dp = −ρ g₀ dh, with h geopotential height.
- Ideal gas: ρ = p / (R T).
- Together: dh/dp = −R T / (g₀ p), so one unit of pressure spans R T / (g₀ p) of height.
- Per hectopascal: 100 × R T / (g₀ p) meters; per inch of mercury: 3,386.38864 × R T / (g₀ p) meters.
- Rule of thumb error: 27 ft minus the answer per hPa.

## Symbols and units

R = 287.05287 J/(kg·K), g₀ = 9.80665 m/s² (ICAO Doc 7488 defining constants). p is the standard-atmosphere pressure at the pressure altitude, in pascals; T is the outside air temperature, or the standard temperature, in kelvin. Pressure altitude and the answers are in feet by default; 1 inHg = 3,386.38864 Pa.

## Domain

Pressure altitudes across the ICAO Standard Atmosphere, −5 km to 80 km (about −16,400 ft to 262,500 ft). An outside air temperature colder than 150 K is refused as no real air column.

## Approximations

The answer is the local rate at the given altitude. Over a large pressure difference the rate itself changes, so a big altimeter-setting error is better worked as two pressure altitudes than as a product of this rate and the difference. The temperature of a real column varies with height; using the outside air temperature at the altitude is exact for the rate there. Water vapor makes air lighter, raising the effective temperature by about 0.06% per gram of vapor per kilogram of air (virtual temperature, T(1 + 0.61q)): up to about 1% in warm, humid air. The tool leaves it out and says so.

## Worked example

- sourcePublisher: airinnova
- sourceTitle: ambiance, a full implementation of the ICAO Standard Atmosphere 1993
- sourceEdition: version 1.3.1
- sourceLocator: Atmosphere(h).pressure, differenced over geopotential height (converted with Atmosphere.geop2geom_height) at sea level, one-sided upward: 8.3242 m (27.31 ft) per hPa and 924.8 ft per inHg; tools/vectors/gen_pressure_height.py
- independent: yes
- inputs: pressure altitude 0 ft, no temperature (ISA)
- outputs: 27.31 ft per hPa, 924.8 ft per inHg
- tolerance: 1e-5 relative (the finite difference's own error is far smaller)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-07

## Differential tests

- `tools/vectors/gen_pressure_height.py`: ambiance, a separate implementation of the standard atmosphere, giving pressure as a function of height; the generator differences it rather than using the closed form, one-sided within a layer near the layer bases, where ambiance's pressure steps by a fraction of a pascal (found at the 11 km tropopause, where a centred difference read 2% low)
- `core/vectors/aviation.altimetry.pressure-per-height.jsonl`: 22 vectors from that reference, from −1,000 ft to 60,000 ft and with outside air temperatures from −60 °C to 35 °C, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `pressure_per_height_invariants`: the height per hPa grows at every 1,000 ft step from −1,000 ft to 60,000 ft; the height per inHg is always 33.8639 times the height per hPa; and a non-standard temperature scales the answer by exactly T / T_ISA
