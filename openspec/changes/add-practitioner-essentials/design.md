## Context

Research: `docs/research/07-practitioner-gap-analysis.md` (gaps, hero tools, cuts, journeys, correctness traps), with sources.

Constraints from the foundation:
- Tools may not read the clock or host time-zone data. Dates, offsets, and zones are inputs, and the tzdb is a versioned asset.
- No operational data is fetched. Weather text, almanacs, and navaid data are pasted or typed.

## Goals / Non-Goals

**Goals:**
- Cover the daily practitioner questions with the same rigor as the core math.
- Encode the known traps: the four "nights", FB wind encoding, VOR station variation, and PLSS not being a formula.

**Non-Goals:**
- Live data. Celestial sight reduction. Legal interpretation.

## Decisions

### P1. A new `time` domain
Sun and time tools serve every other domain, so they get a domain rather than being scattered across aviation or survey.

### P2. NREL SPA as the solar engine, with NOAA shown
SPA gives ±0.0003°, which survey solar azimuths need. NOAA's algorithm is what most websites use, so showing it alongside explains small differences users see elsewhere.

### P3. Aviation text decoders as grammars
METAR, TAF, and FB are parsed with explicit grammars (PEG-style) in the core, with a group-by-group trace shown in the UI ("30015G25KT → wind 300° at 15 kt, gusts 25"). The trace doubles as the "show your work" for decoding. Test corpora:
- FAA JO 7900.5 examples
- NWS/AWC documentation examples
- a curated set of real reports with hand-verified decodes, including edge cases (M1/4SM, SLP rollover, VRB, CAVOK, a TAF crossing month-end)

### P4. PLSS in two steps
- **v1:** parser plus nominal aliquot math, which needs no data.
- **v1.1:** BLM CadNSDI township and section polygons, simplified and tiled per state (public domain), as an on-demand asset. Lookups use polygon containment and never idealized geometry.

The per-state tile granularity (at least one township) satisfies the privacy tile rule.

### P5. Cuts applied in place
- Remove `aviation.performance.koch-estimate`. The limitation it represents is covered by the density-altitude explainer.
- Remove `survey.earthwork.angle-of-repose-reference`: unclear sources and slope-design liability.
- Make the survey sight-distance design K a cited input (per `trust/citations`).
- Drone terrain-following export requires acknowledging that GLO-30 is a surface model, plus a minimum user margin (default 15 m).

## Tool inventory (targets)

| Domain | Group | Operations | Examples |
|---|---|---|---|
| time | `sun` | 10 | solar-position, rise-set-twilight, solar-noon, aviation-nights, night-currency, mapping-window, hotspot, shadow-length, slope-incidence, sun-path |
| time | `scale` | 9 | local-to-utc, utc-to-local, zulu, decimal-hours, block-time, gps-week, gnss-offsets, julian-date, day-of-year |
| aviation | `weather` | 4 | metar-decode, taf-decode, fb-winds-decode, weather-handoff |
| aviation | `ifr` | 12 | hold-entry, hold-wind-timing, hold-speed-limit, dme-ground-distance, dme-arc-lead, time-distance-to-station, intercept-angle, glidepath-vs, vdp, tch-geometry, vasi-papi-height, notam-area |
| survey | `land` | 8 | deed-parse, deed-plot-closure, legacy-units, plss-parse, plss-aliquot-area, plss-lookup (v1.1), plss-reverse (v1.1), basis-rotation |
| survey | `gnss` | 7 | dop-skyplot, rtk-budget, opus-plan, antenna-height, alta-rpp, localization-similarity, localization-affine |
| drone | `sensors` | 8 | vlos, part107-twilight, lidar-plan, lidar-density, dataset-size, link-budget, thermal-footprint, thermal-max-distance |
| **Added** | | **58** | plus 12 high-intent search forms (e.g. `zulu-time-converter`, `metar-decoder`, `sunset-civil-twilight`) |
| **Removed** | | **−2** | koch-estimate, angle-of-repose-reference |

Net effect on the rollup: +56 operations. Endpoint totals are restated in `plan-launch-and-value-proof`, which also applies the page-versus-endpoint rule.

## Risks / Trade-offs

- **[Decoder misreads a report]** → Grammar-based parsing with an undecoded-group list, a group-by-group trace, a corpus gate, and the "get an official briefing" framing.
- **[Night-definition errors carry legal weight]** → Dated regulatory text, four explicit windows, and practitioner (CFI) review required before launch.
- **[PLSS data size and accuracy]** → Per-state tiles on demand, stated reliability, and v1.1 timing.
- **[tzdb updates]** → The tzdb is a ledger-tracked asset. The freshness gate forces updates when IANA releases.

## Migration Plan

Not applicable.

## Open Questions

- Exact EASA ALOS coefficients for fixed-wing aircraft: verify against the current EASA AMC/GM text during implementation (task 6.1). Data only.
