## Context

Research gathered 2026-10-19 from the 2019 Bowditch PDFs (NGA), NOAA CO-OPS pages and API, Schureman SP 98 and Parker's manual, and the USACE squat notes. Facts that shape the design:
- Bowditch 2019 Vol. I chapter 12 gives every sailing with worked examples; Appendix B gives the formulas; Vol. II chapter 5 gives TVMDC worked rows. All public domain.
- NOAA lists 37 constituents per station with amplitude (ft or m), phase (local standard time or GMT), and speed; its prediction equation is h = H₀ + Σ f·H·cos(a·t + (V₀+u) − κ), with f and V₀+u from Schureman. NCEP's `calctide.f` is a public-domain reference. pytides (MIT) implements the same Schureman set.
- Bowditch's sailings are spherical or Mercator-on-the-spheroid; the geodesic tools already give the ellipsoidal truth to compare against.
- Barrass's squat formulas are restated with parameter ranges in free USACE notes (Briggs 2006, 2009), so they can be cited without the paid PIANC report.

## Goals / Non-Goals

**Goals:**
- Bowditch's numbers, to Bowditch's rounding, with full precision underneath and the ellipsoidal answer beside the spherical one.
- Tide predictions that match NOAA's own to 0.01 ft from pasted constituents, with the method visible.

**Non-Goals:**
- Tidal currents (follow-up). Celestial. Operational data.

## Decisions

### M1. Sailings as Bowditch states them, with the ellipsoid beside
Plane, traverse, parallel, mid-latitude, and Mercator sailings use the formulas of Bowditch Appendix B exactly (meridional parts on the WGS 84 spheroid as Table 6 does), and return course to 0.1° and distance to 0.1 nm in the display with full precision in the machine value. Each result also reports the rhumb-line answer from `navigation.rhumb` (the exact ellipsoidal rhumb) and the difference, so the reader sees how much the short-form sailing differs. Great-circle sailing returns the spherical initial course, distance, vertex, and points along the track (per Bowditch §1208–1211), the Karney geodesic answer and its difference, and, when a limiting latitude is given, the composite track (two great-circle legs and a parallel leg, §1213).
- *Alternative: alias the sailings to the existing rhumb and spherical tools.* Rejected: the forms (latitude difference l, departure p, DLo, meridional difference m) are what the reader is checking, and the worked examples are in those terms.

### M2. Current triangle as the three Bowditch problems
`navigation.piloting.set-and-drift` solves, by selection: (1) course and speed plus set and drift → track and speed made good; (2) course made good required, set and drift, speed → course to steer and speed made good; (3) course and speed made good required, set and drift → course to steer and speed to use (Bowditch §907–908, three worked examples). It is the marine form of the wind triangle; the two share one vector-triangle function in the core, so the aviation and marine answers cannot disagree. The diagram draws the triangle with the current vector labeled set and drift.

### M3. Fixes and runs are plane geometry on the Mercator chart
Running fix, two-bearing fix (including doubling the angle on the bow and the bow-and-beam cases as named presets), and dead reckoning work on the Mercator plane at the DR latitude for runs under 100 nm, with the rhumb-line result shown for longer runs and `LIMIT_EXCEEDED` above 600 nm. Bearings are entered true or with variation and deviation applied by the compass tool.

### M4. Compass conversion as a view of the heading chain
`navigation.piloting.compass-conversion` runs the existing `aviation.wind.heading-chain` function with Bowditch's vocabulary (true, variation, magnetic, deviation, compass; compass error = variation + deviation, east and west signed as Bowditch does) and a pasted deviation table interpolated by magnetic heading. The two tools share one function. Variation can come from the magnetic tools at a date, with the chart's printed variation preferred when given (as with VOR station variation).

### M5. Tide prediction from pasted constituents
The paste parser accepts the NOAA station page table rows and the `harcon.json` array, in feet or meters, with phases in local standard time or GMT, and the datum sheet table or `datums.json`. The engine evaluates h(t) = H₀ + Σ f·H·cos(a·t + (V₀+u) − κ) with Schureman's node factors f and equilibrium arguments V₀+u computed from the lunar and solar elements (SP 98 ¶136–141, Tables 14–16), the constituent speeds from Table 2, and f and u evaluated at the middle of the prediction year as NOAA does (selectable: at each instant, as XTide does, with the difference shown). Highs and lows are found by bracketing the derivative. Output is a height series at the chosen interval, the extremes with times, in the station's datum (MLLW unless the sheet moves it) and time zone (the station's local standard time, with daylight time applied only when the reader asks, by the tzdb). Every result states the constituents used, the year of f and u, and that NOAA's published predictions are authoritative.

### M6. Datum conversion from a pasted sheet
`navigation.tide.datums` converts a height between the datums on a pasted NOAA datum sheet (STND, MHHW, MHW, MTL, MSL, MLW, MLLW, NAVD88, LAT, HAT) by subtraction, and draws the datum stack. It is the marine twin of the height-stack diagram. It refuses to guess a datum that the sheet lacks.

### M7. Under-keel clearance with squat as an input-driven estimate
UKC = charted depth + tide height − (draft + squat + margin), with squat by Barrass's simplified formula (S = C_B·V²/100 in open water with h/T between 1.1 and 1.4; 2·C_B·V²/100 in a confined channel) or the general form with blockage factor, as restated in USACE CHETN-I-72 and CHETN-IX-19 with their parameter ranges. Inputs outside the published ranges return `OUT_OF_DOMAIN` naming the range. The result shows the stack (depth, tide, draft, squat, margin, clearance) and never calls a clearance "safe": it reports the number against the user's own margin and names the master's and the port's authority. Tide height can be sent from the tide tool at the arrival time.

### M8. Verification
- Sailings: Bowditch chapter 12 worked examples (great circle 22°S 116°E → 20°S 31°E: 4,693.8 nm, 253°; mid-latitude 1,253 nm on 070° from 15°17.0'N 151°37.0'E → 22°25.6'N 172°21.2'E; Mercator 32°14.7'N 66°28.9'W → 36°58.7'N 75°42.2'W: 301.8°, 538.9 nm; and the rest) to Bowditch's printed precision; the spherical great circle differentially against `navigation.geodesic.spherical-inverse`; Mercator sailing against `navigation.rhumb.inverse` within the stated spheroid-vs-table difference.
- Piloting: Bowditch §908's three current examples (track 089°, 11.2 kn; CTS 083.5°, 12.4 kn; CTS 276°, 14.8 kn) to plotting precision (0.5°, 0.1 kn, with the exact values in the machine result); Vol. II §502's TVMDC rows exactly; vertical-angle distances against Bowditch Table 16 entries.
- Tides: NOAA's predictions API output for at least 20 stations and 2 years, hourly and extremes, within 0.02 ft (the documented f/u epoch difference); f and V₀+u differentially against pytides and `calctide.f` for every constituent and year 1990–2040 within 1e-6.
- UKC: the Panama Canal measured comparisons in CHETN-IX-19 reproduce the note's predicted squat to its printed precision; invariants: UKC is linear in tide and margin.

## Tool inventory

| Group | Operations | Endpoints | Reference | Independent vector |
|---|---|---|---|---|
| `navigation.sailing` | plane, traverse, parallel, mid-latitude, mercator, great-circle | 6 | Bowditch 2019 ch. 12, App. B | ch. 12 examples |
| `navigation.piloting` | set-and-drift, dead-reckoning, running-fix, two-bearing-fix, distance-by-vertical-angle, compass-conversion | 6 | Bowditch ch. 9–11, App. B; Vol. II ch. 5 | §908, §502, Table 16, Table 18 |
| `navigation.tide` | predict, datums | 2 | Schureman SP 98; Parker 2007; NOAA CO-OPS | NOAA predictions API; pytides |
| `navigation.vessel` | under-keel-clearance | 1 | Barrass via USACE CHETN-I-72, IX-19/20 | CHETN-IX-19 comparisons |
| **Total** | **15** | **15** | | |

The navigation rollup moves from 46 to 61 operations. Alias slugs (`tide-calculator`, `set-and-drift-calculator`, `mercator-sailing`) route to their tools and are not counted.

## Risks / Trade-offs

- **[Tide predictions treated as official]** → Every result names NOAA's predictions as authoritative, states the f/u year, and the tool refuses to run without a complete 37-row paste or an explicit "partial constituents" acknowledgment with the missing ones listed.
- **[Navigation module budget]** → The tide engine's astronomical arguments are small; if the module passes 400 KB, `tide` becomes its own Wasm module behind the same ids. Measured 2026-10-20 with the tide engine and both tide tools linked: `navigation.wasm` is 723,812 bytes, 255,608 Brotli against the 409,600-byte domain budget (62%), so `tide` stays in the navigation module.
- **[Readers use UKC as a go/no-go]** → No verdict wording; margin is the reader's; the port authority and master are named as governing.

## Migration Plan

Not applicable. New tools start experimental. The MCP `navigation` toolset splits into `navigation` and `marine` (each ≤ 40 stable tools) when the marine tools are promoted.

## Open Questions

- Whether the ESA-free spheroid meridional-parts table (Bowditch Table 6) and the exact ellipsoidal formula differ beyond 0.1' at high latitudes; measured in task 2 and recorded in the derivation note.
