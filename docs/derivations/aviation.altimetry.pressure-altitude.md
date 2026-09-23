<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Pressure altitude (`aviation.altimetry.pressure-altitude`)

## Method

An altimeter set to QNH reads PA(p) − PA(QNH): its pressure altitude less the pressure altitude of the setting itself. On the field it reads the elevation, so the field's pressure altitude is the elevation plus PA(QNH). The station pressure is the ICAO standard-atmosphere pressure at that altitude. This is the relation the NWS altimeter-setting formula and the FAA handbook's correction table both encode. The 1,000 ft per inch rule of thumb is shown next to it with its error.

## Equations

- Station pressure: p = p_ISA(elevation + PA(QNH)), where PA(QNH) is the ISA altitude whose pressure is QNH.
- Scaling QNH by the ratio p_ISA(elevation) / p0 instead, as version 1.0.0 did, agrees only at sea level or at 29.92 inHg: at an 8,000 ft field with 28.20 inHg it is 90 ft low (see the changelog).
- Pressure altitude: PA = H such that p_ISA(H) = p, using the closed-form inverse of the troposphere layer, T0/L × ((p/p0)^(−R L/g0) − 1), and the matching layer formula above 11 km.
- Rule of thumb: PA ≈ elevation + (29.92 − QNH in inHg) × 1,000 ft.

## Symbols and units

p0 = 101,325 Pa, T0 = 288.15 K, L = −0.0065 K/m, R = 287.05287 J/(kg·K), g0 = 9.80665 m/s². QNH accepts inHg, hPa, and METAR `A2992` or `Q1013` groups. Elevation and PA are in feet by default.

## Domain

Field elevations from −1,500 ft to 20,000 ft; settings from about 25 to 33 inHg (outside 27.5 to 31.5 inHg a SUSPECT_VALUE warning asks the pilot to check the value).

## Approximations

None beyond the standard atmosphere. This is the same definition altimeters use. The rule of thumb is off by up to about 100 ft at extreme settings, and the result shows that error.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 11, figure 11-3 (field at sea level; altimeter 29.7 gives a PA of 205 ft; 28.2 gives 1,630 ft; 31.0 gives −983 ft)
- independent: yes
- inputs: elevation 0 ft, altimeter 29.7 inHg
- outputs: pressure altitude 205 ft
- tolerance: 1 ft (the table prints whole feet)
- verifiedBy: golden vectors v020–v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: an independent Python implementation that inverts the standard atmosphere by bisection rather than the core's closed forms, at 19 elevation and setting pairs in inHg and hPa (within 1e-9 relative)
- `core/vectors/aviation.altimetry.pressure-altitude.jsonl`: those vectors plus the three FAA table rows, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `altimetry_invariants`: at 1013.25 hPa the pressure altitude equals the field elevation (within 1e-6 ft), and it falls monotonically as the setting rises
