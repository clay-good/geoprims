<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Cloud base and freezing level (`aviation.atmosphere.cloud-base`)

## Method

A parcel of surface air lifted without mixing cools at the dry adiabatic rate until it saturates at the lifting condensation level (LCL), where convective cloud forms. The tool finds the temperature there with Bolton's (1980) equation 15 and divides the cooling by the dry adiabatic rate to get the height. The pilot's rule of thumb, 400 ft per degree Celsius of temperature–dew point spread, is shown beside it. The freezing level is the surface temperature divided by a steady lapse rate, the standard 1.98 °C per 1,000 ft by default.

## Equations

- Rule: base_rule = (T − T_d) × 400 ft
- Bolton eq. 15: T_L = 1 / (1/(T_dK − 56) + ln(T_K / T_dK)/800) + 56, all in kelvins
- LCL height above the field: z = (T_K − T_L) / (g/c_p), with g/c_p = 9.80665 / 1,004 K/m (about 9.77 K per km)
- Cloud base above sea level = z (in ft) + field elevation, when the elevation is given
- Freezing level = T / Γ × 1,000 ft, plus the field elevation when given; only when T > 0 °C

## Symbols and units

T surface temperature and T_d dew point in °C (T_K, T_dK in kelvins, +273.15); Γ lapse rate in °C per 1,000 ft (0.1 to 5.0, default 1.98); g = 9.80665 m/s²; c_p = 1,004 J/(kg·K); heights in feet (1 ft = 0.3048 m).

## Domain

Surface temperature from −60 °C to 60 °C, dew point at or below the temperature and not below −80 °C. A dew point above the temperature is refused. The freezing level is left out when the surface is at or below 0 °C.

## Approximations

Both bases assume well-mixed surface air forming convective cloud. Bolton's fit gives the LCL temperature within about 0.1 K. The 400 ft rule ignores that the dew point also falls as the parcel rises; for the example it reads 123 ft below the LCL. The freezing level uses one steady lapse rate. Neither estimate says anything about stratus, fog, or layers that move in.

## Worked example

- sourcePublisher: Unidata (University Corporation for Atmospheric Research), MetPy
- sourceTitle: MetPy 1.6.3, metpy.calc.lcl and metpy.constants.dry_adiabatic_lapse_rate
- sourceEdition: MetPy 1.6.3, run 2026-10-10
- sourceLocator: tools/vectors/gen_cloud_base_metpy.py, first case: lcl(1013.25 hPa, 25 °C, 15 °C), and the dry-adiabatic climb from 25 °C to its temperature
- independent: yes
- inputs: temperature 25 °C, dew point 15 °C
- outputs: MetPy puts the LCL 4,121.8 ft above the surface; the tool gives 4,123.3 ft, beside 4,000 ft by the rule
- tolerance: 0.1% of the LCL height
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-10-10

MetPy finds the LCL by iterating on the parcel's saturation, where the tool uses Bolton's closed fit, and it carries its own g ÷ c_p (9.761 K per km against 9.77). Over 16 surface readings from −20 °C to 45 °C with spreads from 0.5 °C to 50 °C the two heights agree within 0.08%. MetPy's LCL temperature is the same at 1,013.25, 850, and 700 hPa, so leaving the surface pressure out of the tool costs nothing. The 400 ft per degree rule and the freezing level are plain arithmetic with no library behind them; the invariants below hold them.

## Differential tests

- `tools/vectors/gen_cloud_base_metpy.py`: the LCL height above the field, and above sea level where an elevation is given, from MetPy at 16 surface readings (within 0.1%)
- `tools/vectors/gen_heading.py`: a Python evaluation of the rule, Bolton's eq. 15, and the freezing-level division at six surface readings with and without field elevation and lapse rate (within 1e-6 ft); it restates the same formulas, so it checks the code's arithmetic and unit handling, not Bolton's fit
- `core/vectors/aviation.atmosphere.cloud-base.jsonl`: those vectors and a refused dew point above the temperature, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `cloud_base_invariants`: at five temperatures and seven spreads the rule is 400 ft per degree, saturated air has its base on the ground, the LCL rises with the spread and stays within 7% of the rule, the height above sea level is the LCL plus the elevation, and the freezing level is the surface temperature over 1.98 °C per 1,000 ft above the field, left out at or below freezing
