<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Density altitude (`aviation.altimetry.density-altitude`)

## Method

Find the station pressure from the field elevation and the altimeter setting, as in pressure altitude. Compute the air density from the ideal gas law at the observed temperature (the virtual temperature when a dew point is given). Density altitude is the altitude in the ICAO standard atmosphere that has that density. The 118.8 ft/°C and 120 ft/°C rules of thumb are shown with their errors.

## Equations

- p = p_ISA(elevation + PA(QNH)), the station pressure from the altimeter-setting relation (see pressure altitude).
- Dry air: ρ = p / (R T).
- Humid air: e = 610.94 exp(17.625 Td / (Td + 243.04)) Pa (Alduchov and Eskridge 1996), Tv = T / (1 − 0.378 e / p), ρ = p / (R Tv).
- DA = H such that ρ_ISA(H) = ρ.
- Rules of thumb: DA ≈ PA + 118.8 × (T − T_ISA(PA)) or PA + 120 × (T − T_ISA(PA)), with T in °C.

## Symbols and units

p station pressure (Pa), T air temperature (K), Td dew point (°C), e vapor pressure (Pa), Tv virtual temperature (K), ρ density (kg/m³), R = 287.05287 J/(kg·K). Results in feet by default.

## Domain

Temperatures and dew points from −100 °C to +80 °C, the span where the Magnus fit holds. The dew point cannot exceed the temperature, and the vapor pressure must stay below half the station pressure.

## Approximations

The Magnus fit for vapor pressure is within 0.4% from −40 °C to +50 °C. Without a dew point, dry air is assumed and the result says so, because humidity raises density altitude. The rules of thumb ignore pressure changes with temperature, and the result reports their error.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 11, sample problem 1 and figure 11-22 (airport elevation 5,883 ft, OAT 70 °F, altimeter 30.10 inHg; density altitude read from the chart as 7,700 ft)
- independent: yes
- inputs: elevation 5,883 ft, altimeter 30.10 inHg, temperature 70 °F
- outputs: density altitude about 7,700 ft (the core gives 7,718 ft)
- tolerance: 100 ft (reading a chart with 1,000 ft gridlines)
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: an independent Python implementation that finds density altitude by bisection on the standard atmosphere's density profile, at 20 cases dry and humid, from −200 ft to 12,000 ft and −30 °C to +45 °C (within 1e-9 relative)
- `core/vectors/aviation.altimetry.density-altitude.jsonl`: those vectors plus the FAA chart example, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `altimetry_invariants`: dry air at exactly ISA temperature has a density altitude equal to its pressure altitude (within 0.001 ft), and density altitude rises monotonically with temperature
