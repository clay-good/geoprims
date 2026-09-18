## Context

Motivation is in `proposal.md`. Research: `docs/research/03-aviation-drone-survey-formulas.md`, which contains the formulas, constants, and a list of 20 "precision traps" this design guards against. Key facts:

- ISA constants are defined in ICAO Doc 7488/3. Layer math must use geopotential altitude; mixing in geometric altitude causes about 19 m of error at 11 km.
- Two constant sets exist for pressure altitude: ISA-derived (145,442.16 ft, exponent 0.190263) and NWS (145,366.45 ft, 0.190284). We use the ISA-derived set and document it.
- The cold-temperature equation printed in ICAO Doc 8168 Vol III (2018) contains an error. Transport Canada AC 500-020 §4.8 documents the corrected form from Doc 8168 Vol II, 7th edition (2020), which is the version we implement.
- The FAA Cold Temperature Airports list is republished annually. The current list expired September 3, 2026, so hard-coding it is wrong.

## Goals / Non-Goals

**Goals:**
- Exact formulas by default, with the rule of thumb shown beside them and its error quantified. This teaches, and it catches users who would otherwise trust the approximation.
- Every true/magnetic reference is explicit. Every assumption (ISA temperature, IAS = CAS, dry air) becomes a named warning.

**Non-Goals:**
- Aircraft-type performance models. Operational data.

## Decisions

### AV1. One atmosphere engine, two models
A single layer-table evaluator handles both ICAO (−5 to 80 km geopotential) and US76 (to 86 km geometric, with its upper-atmosphere molecular-weight treatment above 86 km excluded). Non-standard days apply a temperature offset while keeping the ISA pressure-height relation, which is the aviation convention.

### AV2. Airspeed through impact pressure
Every conversion routes through qc and Mach, so there is exactly one compressibility implementation. The supersonic Rayleigh branch is included because the isentropic formula silently gives wrong answers above Mach 1, and some UAM/defense users approach it. It is marked with the branch used.

### AV3. Warnings for common assumptions
Named warnings: `CALIBRATION_ASSUMED`, `ISA_TEMPERATURE_ASSUMED`, `RUNWAY_HEADING_APPROXIMATE`, dry-air density, and true/magnetic mixing. These are surfaced in the UI and carried in MCP `meta`, so agents relay them.

### AV4. Regulatory values are reference data, not code
Fuel reserves, transition altitudes, lowest-usable-flight-level tables, and links to cold-temperature airport lists live in a dated reference-data file with citations. Each result shows "rules as of <date>". A data update changes them; code does not.

### AV5. User aircraft data stays local
Calibration tables, deviation cards, envelopes, and performance tables are user-entered, validated (monotonic axes, closed envelope polygon), stored only locally, and exportable as JSON.

## Tool inventory (targets)

| Group | Operations | Examples |
|---|---|---|
| `atmosphere` | 14 | isa, us76, geometric-geopotential, nonstandard-day, speed-of-sound, viscosity, ratios, rh-dewpoint, vapor-pressure, virtual-temperature, moist-density, cloud-base, freezing-level, profile |
| `airspeed` | 12 | ias-to-cas (table), cas-to-eas, eas-to-cas, cas-to-tas, tas-to-cas, tas-to-mach, mach-to-tas, cas-to-mach, mach-to-cas, impact-pressure, tat-sat, dynamic-pressure |
| `altimetry` | 14 | pressure-altitude, station-pressure, altimeter-setting, density-altitude, isa-temperature, isa-deviation, qnh-qfe, qfe-qnh, flight-level, lowest-usable-fl, cold-temp-correction, cold-temp-segments, true-altitude, pressure-per-height |
| `wind` | 14 | heading-groundspeed, find-wind, tas-from-groundspeed, course-from-heading, runway-components, gust-components, best-runway, crosswind-limit-check, heading-chain, deviation-card, one-in-sixty, drift-angle, uv-to-direction-speed, winds-aloft-interpolation |
| `performance` | 15 | turn-radius, turn-rate, bank-for-rate, load-factor, stall-in-turn, time-to-turn, climb-gradient, descent-gradient, top-of-descent, descent-angle, glide-range, glide-ring, pivotal-altitude, visual-descent-point, specific-range |
| `loading` | 14 | fuel-required, endurance, fuel-weight, reserve-preset, fuel-per-leg, burn-rate, cg, percent-mac, envelope-check, weight-shift, ballast, table-1d, table-2d, table-3d |
| **Operations** | **83** | |
| Generated endpoints | 28 | composed airspeed pairs (e.g. `ias-to-mach`, `eas-to-tas`, `ias-to-tas`), altimeter unit forms (`qnh-hpa-to-pressure-altitude`), and common E6B aliases (`crosswind-calculator`, `headwind-calculator`) |
| **Endpoints** | **111** | |

## Risks / Trade-offs

- **[Users treat outputs as operational]** → Persistent not-for-navigation notice. Dated regulatory values. No aircraft-specific defaults.
- **[Rules of thumb displayed prominently get copied]** → Rule-of-thumb values are visually secondary and always labeled with their error.
- **[Constant-set disagreements with other calculators (e.g. ±12 ft pressure altitude)]** → Docs page explains the two constant sets. An option switches to NWS constants for cross-checking.

## Migration Plan

Not applicable.

## Open Questions

None that affect the specs.
