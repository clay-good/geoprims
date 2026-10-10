<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Calibrated airspeed from true airspeed, Mach, or EAS (`aviation.airspeed.tas-to-cas`)

## Method

The cas-to-tas chain run backward. The input speed is turned into a Mach number: true airspeed divided by the speed of sound at the outside air temperature, Mach as given, or equivalent airspeed divided by a0·√(p/P0), which needs no temperature. Mach and the ISA static pressure at the pressure altitude give the impact pressure. Calibrated airspeed is the speed that gives that impact pressure at sea level in the standard atmosphere. Below Mach 1 the isentropic pitot relation is used; above Mach 1 the Rayleigh pitot formula, solved for Mach by fixed-point iteration.

## Equations

- Static pressure: p = p_ISA(pressure altitude), floored at 1 Pa.
- Speed of sound: a = √(1.4 · R · T); a0 = a at T0.
- Mach from the input: M = TAS / a(T), or M as given, or M = EAS / (a0 · √(p/P0)).
- Impact pressure ratio: qc/p = (1 + 0.2 M²)^3.5 − 1 for M ≤ 1; qc/p = 1.2^3.5 · 6^2.5 · M⁷ / (7M² − 1)^2.5 − 1 for M > 1.
- Impact pressure: qc = p · (qc/p).
- CAS: Mc = √(5 · ((qc/P0 + 1)^(2/7) − 1)); if Mc > 1, iterate Mc ← K · √((qc/P0 + 1) · (1 − 1/(7Mc²))^2.5) with K = √(7^2.5 / (1.2^3.5 · 6^2.5)) until the step is within 1e-12 relative (at most 200 steps); CAS = a0 · Mc.
- Also reported: TAS = M · a (only when a temperature is known), EAS = M · a0 · √(p/P0), dynamic pressure 0.7 · p · M², and the compressibility correction CAS − EAS.

## Symbols and units

P0 = 101,325 Pa, T0 = 288.15 K, R = 287.05287 J/(kg·K), γ = 1.4. T is the static outside air temperature in kelvin (or the ISA temperature at the pressure altitude when the user picks it, with ISA_TEMPERATURE_ASSUMED). Speeds are in knots by default; pressure altitude in feet.

## Domain

Pressure altitude from −5 km to 30 km (−16,404 ft to 98,425 ft); Mach input from 0 to 5; one positive speed (TAS, Mach, or EAS), not more than one. TAS input needs a temperature; Mach or EAS alone leave TAS out.

## Approximations

The air is a dry ideal gas with γ = 1.4, and the static pressure is the ICAO standard atmosphere at the pressure altitude. The answer is calibrated airspeed: there is no calibration table, so the position and instrument error to indicated airspeed is not modeled. The supersonic branch is iterated to 1e-12 relative.

## Worked example

- sourcePublisher: geoprims (round trip of the add-aviation-suite FL350 airspeed scenario, computed)
- sourceTitle: add-aviation-suite airspeed spec, scenario "High altitude", run backward with the Gracey (NASA RP-1046) relations
- sourceEdition: add-aviation-suite change; NASA RP-1046 (1980)
- sourceLocator: openspec/changes/add-aviation-suite/specs/aviation/airspeed/spec.md, scenario "High altitude"; tool example "primary"
- independent: no
- inputs: Mach 0.78, pressure altitude 35,000 ft, temperature −54.3 °C
- outputs: CAS 264.42 kt, TAS 449.65 kt (from the independent Python generator, not a published table)
- tolerance: 1e-9 relative
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: a separate Python implementation of the Gracey relations that finds Mach from qc/p by bisection rather than the core's fixed point, at six Mach inputs (one supersonic, Mach 1.6) and four EAS inputs (within 1e-9 relative)
- `core/vectors/aviation.airspeed.tas-to-cas.jsonl`: those ten vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `round_trip_and_compressibility_sign`: for CAS from 40 to 900 kt and pressure altitudes from −1,000 ft to 60,000 ft, the TAS and the EAS from `aviation.airspeed.cas-to-tas` come back to the starting CAS through this tool (within 1e-8 relative), and EAS input returns no TAS
- `core/crates/gp-aviation/tests/slice2.rs` `supersonic_branch_round_trips`: 600 KCAS at 40,000 ft is supersonic, and its Mach comes back to 600 KCAS through this tool (within 1e-7 kt)
