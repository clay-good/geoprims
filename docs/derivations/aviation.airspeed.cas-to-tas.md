<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# True airspeed from indicated or calibrated airspeed (`aviation.airspeed.cas-to-tas`)

## Method

The exact compressible-flow path from Gracey (NASA RP-1046). Calibrated airspeed fixes the impact pressure through the sea-level relation. That impact pressure, over the static pressure of the ISA at the pressure altitude, gives the Mach number. Mach times the speed of sound at the outside air temperature is the true airspeed. Below Mach 1 the isentropic relation is used; above it, the Rayleigh pitot formula, solved for Mach by fixed-point iteration. Indicated airspeed is turned into calibrated airspeed by linear interpolation in the POH table the pilot enters, or taken as calibrated with a warning when there is no table. The 2% per 1,000 ft rule of thumb is shown beside the exact answer with its error.

## Equations

- Impact pressure: qc = p0 · f(CAS / a0), where f(M) = (1 + 0.2 M²)^3.5 − 1 for M ≤ 1 and f(M) = 1.2^3.5 · 6^2.5 · M⁷ / (7 M² − 1)^2.5 − 1 above
- Mach: M solves f(M) = qc / p, with p = p_ISA(pressure altitude). Subsonic: M = √(5 ((qc/p + 1)^(2/7) − 1)). If that comes out above 1, iterate M ← K · √((qc/p + 1)(1 − 1/(7 M²))^2.5), K = √(7^2.5 / (1.2^3.5 · 6^2.5)), up to 200 times, until the step is under 1e-12 relative.
- Speed of sound: a = √(γ R T); a0 = √(γ R T0)
- TAS = M · a; EAS = M · a0 · √(p / p0); dynamic pressure q = 0.7 · p · M²; compressibility correction = CAS − EAS
- IAS to CAS (with a table): CAS = y0 + (y1 − y0)(IAS − x0)/(x1 − x0) between the bracketing rows
- Rule of thumb: CAS × (1 + 0.02 × pressure altitude in ft / 1,000); rule error = rule − TAS

## Symbols and units

p0 = 101,325 Pa, T0 = 288.15 K, R = 287.05287 J/(kg·K), γ = 1.4. p is static pressure in Pa and T the static (outside) air temperature in K. Speeds are in m/s inside and knots by default out; qc and q in hPa by default. The temperature is measured, or the ISA temperature at the pressure altitude when `temperature_source` is `isa`.

## Domain

An airspeed above zero; pressure altitude from −5 km to 30 km (−16,404 ft to 98,425 ft); a temperature above absolute zero, or the ISA choice (not both). A calibration table needs at least two rows in increasing order of indicated airspeed, and an airspeed outside the table is refused with OUT_OF_DOMAIN rather than extrapolated. Optional V-speeds must be positive and ordered as an indicator's arcs; they only raise warnings.

## Approximations

Air is a perfect gas with γ = 1.4, and the static pressure is the standard atmosphere's at the pressure altitude, which is how a pressure altitude is defined. The temperature must be static air temperature, not a probe reading with ram rise. The calibration table is interpolated linearly. The 2% rule is an approximation the tool shows only beside the exact value; at 250 KCAS at 10,000 ft and −5 °C it is 11.4 kt high.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite airspeed spec, scenario "CAS 250 kt at FL100"
- sourceEdition: openspec/changes/add-aviation-suite/specs/aviation/airspeed/spec.md
- sourceLocator: Requirement "CAS ↔ TAS ↔ Mach via impact pressure"; numbers computed from the Gracey relations, not taken from a published table
- independent: no
- inputs: CAS 250 kt, pressure altitude 10,000 ft, OAT −5 °C
- outputs: Mach 0.4523, TAS 288.6 kt, EAS 248.1 kt; rule of thumb 300 kt
- tolerance: 0.1 kt for TAS and EAS; 0.00005 for Mach
- verifiedBy: golden vector v001 and `cas_250_at_fl100` in core/crates/gp-aviation/tests/slice2.rs
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: a separate Python implementation of the same Gracey relations and ISA, which finds Mach by bisection on the monotonic qc/p curve instead of the core's closed form and fixed point; seven cases from sea level to 40,000 ft, one supersonic (within 1e-9 relative)
- `core/vectors/aviation.airspeed.cas-to-tas.jsonl`: those seven vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `round_trip_and_compressibility_sign`: at nine CAS values from 40 to 900 kt and seven pressure altitudes from −1,000 to 60,000 ft (ISA temperature), TAS and EAS each convert back to the same CAS through `aviation.airspeed.tas-to-cas` within 1e-8 relative, and the compressibility correction is never negative at or above sea level below Mach 1
- `core/crates/gp-aviation/tests/slice2.rs` `supersonic_branch_round_trips`: 600 KCAS at FL400 is above Mach 1, reports the Rayleigh branch, and its Mach converts back to 600 kt within 1e-7
- `core/crates/gp-aviation/tests/slice2.rs` `calibration_table`: no table gives CAS = IAS with CALIBRATION_ASSUMED; 130 KIAS interpolates to 129.5 KCAS; 180 KIAS past a table ending at 160 kt is OUT_OF_DOMAIN naming both ends
