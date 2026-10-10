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

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite, aviation/atmosphere spec, "Cloud base and freezing level estimates"
- sourceEdition: 2026
- sourceLocator: Scenario "Cloud base approximation" (25 °C, dew point 15 °C: about 4,000 ft AGL by the rule, LCL alongside)
- independent: no
- inputs: temperature 25 °C, dew point 15 °C
- outputs: rule 4,000 ft (from the spec); LCL 4,123 ft and freezing level 12,626 ft (computed in Python from the equations above)
- tolerance: 1e-6 ft
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_heading.py`: a Python evaluation of the rule, Bolton's eq. 15, and the freezing-level division at six surface readings with and without field elevation and lapse rate (within 1e-6 ft); it restates the same formulas, so it checks the code's arithmetic and unit handling, not Bolton's fit
- `core/vectors/aviation.atmosphere.cloud-base.jsonl`: those vectors and a refused dew point above the temperature, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool; this is the closest test that exercises it, and it runs every vector in the tool's file through the core and fails on any value outside its tolerance
