<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# International Standard Atmosphere (`aviation.atmosphere.isa`)

## Method

Direct evaluation of the defining equations of the ICAO Standard Atmosphere (Doc 7488/3), or the US Standard Atmosphere 1976 to 86 km: temperature is linear in geopotential altitude within each layer, and pressure follows from hydrostatic balance and the ideal gas law. Geometric and geopotential altitude convert with the standard's radius r0 = 6,356,766 m.

## Equations

- H = r0 z / (r0 + z) (geopotential from geometric altitude).
- Gradient layers: T = Tb + L (H − Hb), p = pb (T/Tb)^(−g0/(R L)).
- Isothermal layers: p = pb exp(−g0 (H − Hb)/(R Tb)).
- ρ = p/(R T), a = √(γ R T), μ = β T^1.5/(T + S), ν = μ/ρ, g = g0 (r0/(r0 + z))².

## Symbols and units

H geopotential altitude (m′), z geometric altitude (m), T temperature (K), p pressure (Pa), ρ density (kg/m³), L lapse rate (K/m′), R = 287.05287 J/(kg·K), g0 = 9.80665 m/s², γ = 1.4, β = 1.458e-6 kg/(m·s·K^0.5), S = 110.4 K, a speed of sound, μ dynamic and ν kinematic viscosity.

## Domain

ICAO: -5 km to 80 km geopotential. US 1976: to 86 km geometric. A temperature deviation of up to ±150 °C shifts temperature (ISA + ΔT) at standard pressure, so density follows p/(R(T + ΔT)).

## Approximations

None within the standard: it is a definition. Layer base pressures are computed from sea level with the defining constants, which reproduces the printed tables within their rounding.

## Worked example

- sourcePublisher: International Civil Aviation Organization
- sourceTitle: Manual of the ICAO Standard Atmosphere (Doc 7488/3)
- sourceEdition: Third edition, 1993
- sourceLocator: Table 1 at 3,048 m geopotential (10,000 ft)
- independent: yes
- inputs: altitude 10,000 ft
- outputs: 268.338 K, 696.82 hPa, 0.904637 kg/m³
- tolerance: the table's printed rounding
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `tools/vectors/gen_aviation.py`: 8 vectors from ambiance 1.3.1, a separately written implementation of the ICAO atmosphere, from -2 km to 78 km (pressure and density within 3e-6 relative)
- `core/vectors/aviation.atmosphere.isa.jsonl`: the defining equations in Python, US 1976 printed table rows, and ambiance, run through the core on every build

- `core/crates/gp-aviation/tests/us76_table.rs`: the US 1976 model at every tabulated kilometer from 0 to 81 km (geometric) against the ambiance package, an independent implementation of the standard (`tools/vectors/gen_us76_ambiance.py`), each value within half a unit of the table's last printed digit: temperature to 0.001 K (to 80 km; above it the table's kinetic temperature carries M/M0, which ambiance leaves out), pressure and density to five significant figures

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `isa_invariants`: the ideal gas law at every level, hydrostatic balance dp/dH = -ρg0, and monotonic pressure and density across the ICAO range
