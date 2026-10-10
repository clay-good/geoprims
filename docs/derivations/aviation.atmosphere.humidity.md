<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Humidity and moist air (`aviation.atmosphere.humidity`)

## Method

The saturation vapor pressure over water comes from the Magnus form with the Alduchov and Eskridge (1996) coefficients. The vapor pressure is the saturation pressure at the dew point, or the relative humidity times the saturation pressure at the air temperature. The dew point, when not given, inverts the same form. From the vapor pressure and the station pressure come the mixing ratio, the virtual temperature, and the density of the moist air, which the tool sets beside the density of dry air at the same temperature and pressure.

## Equations

- Saturation vapor pressure: e_s(T) = 610.94 Pa · exp(17.625 T / (T + 243.04)), T in °C
- Vapor pressure: e = e_s(T_d), or e = (RH / 100) · e_s(T)
- Dew point: g = ln(e / 610.94); T_d = 243.04 g / (17.625 − g)
- Relative humidity: RH = 100 · e / e_s(T)
- Mixing ratio: w = 1,000 · 0.622 · e / (p − e) g/kg
- Virtual temperature: T_v = T_K / (1 − 0.378 e / p)
- Densities: ρ_moist = p / (R T_v); ρ_dry = p / (R T_K)

## Symbols and units

R = 287.05287 J/(kg·K) for dry air; ε = Rd/Rv = 0.622. T and T_d in °C (T_K = T + 273.15); p and e in Pa inside, hPa by default out; densities in kg/m³; mixing ratio in g/kg. The pressure defaults to 1013.25 hPa.

## Domain

Air temperature from −40 °C to 50 °C. Give either a dew point or a relative humidity, not both. The dew point must be at or below the temperature and at least −60 °C; the relative humidity must be above 0% and at most 100%. Pressure from 100 to 1,100 hPa.

## Approximations

The Magnus form is a fit: within 0.4% from −40 °C to 50 °C over liquid water, per Alduchov and Eskridge. Below freezing over ice it reads high. Air and water vapor are treated as ideal gases. The densities describe the air you enter; they are not an aircraft performance number.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite atmosphere spec, scenario "Humid density"
- sourceEdition: openspec/changes/add-aviation-suite/specs/aviation/atmosphere/spec.md
- sourceLocator: Requirement "Humidity and moist-air effects"; the spec states only that moist density is below dry density, and the numbers are the equations above worked in Python by tools/vectors/gen_humidity.py
- independent: no
- inputs: temperature 30 °C, dew point 24 °C, pressure 1,000 hPa
- outputs: moist density 1.1362 kg/m³, dry density 1.1492 kg/m³; RH 70.3%; vapor pressure 29.78 hPa; saturation 42.37 hPa; mixing ratio 19.09 g/kg; virtual temperature 33.45 °C
- tolerance: 1e-9 relative
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_humidity.py`: the same moist-air relations written again in Python, at six cases from −20 °C to 45 °C and 700 to 1,013.25 hPa, from both the dew point and the relative humidity, plus four refused inputs. It checks the arithmetic, not the choice of formula.
- `core/vectors/aviation.atmosphere.humidity.jsonl`: those ten vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/src/humidity.rs` `agrees_with_the_nws_tetens_form`: from 0 °C to 50 °C the saturation vapor pressure agrees within 0.5% with the Tetens form the NWS uses, a check on the constants by a different formula
- `core/crates/gp-aviation/src/humidity.rs` `dew_point_inverts_the_vapor_pressure`: the dew point of the saturation pressure at T_d gives back T_d within 1e-9 °C at four dew points from −30 °C to 24 °C
