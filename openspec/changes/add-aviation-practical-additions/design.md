## Context

Research gathered 2026-10-19 from ecfr.gov (2026-10-01 point in time), the FAA handbooks and orders (PHAK-25C, AFH-3C, IPH-16B, AC 61-107B, Order 8260.3F), NASA NTRS, the EASA AMC & GM to Part-CAT, and the Chevron review. Findings that shape the design:
- No FAA handbook states V_A ∝ √W as a formula; it follows from 14 CFR 23.335(c) (V_A ≥ V_S√n at design weight) and V_S ∝ √W. The tool says so.
- PHAK and AFH disagree on the hydroplaning constant (9√p vs 8.6√p); NASA TN D-2056 gives 9√p. The 7.7√p "spin-down" figure is not in the NASA report and is not used.
- TUC tables differ at 22,000 ft between PHAK Figure 17-1 and AC 61-107B Figure 2-3; the tool shows both, named.
- The banded circling-radius table in the TPP legend is not in Order 8260.3F; the order gives formulas, and a spot check of the formula (category C at 1,000 ft MSL) gives 2.83 NM against the table's 2.7 NM. The tool computes the order's formula and does not reproduce the table until it is read from the legend and its assumptions found.
- ASTM D1250 volume-correction tables are paid; Chevron's coefficient and Measurement Canada's free Jet A VCF table give a cited default and vectors.

## Goals / Non-Goals

**Goals:**
- Exact formulas with the regulation or handbook named and the rule of thumb beside, as the aviation suite does.
- Regulatory values as dated data, never code.

**Non-Goals:**
- Type-specific performance. Medical advice. Paid-source tables.

## Decisions

### A1. Derived rules are labeled derived
`va-for-weight` computes V_A,w = V_A,max × √(W / W_max) and the sentence says "derived from 14 CFR 23.335(c) and V_S ∝ √W; your AFM governs if it publishes V_A by weight." It also shows the PHAK rule V_A ≈ 1.7 V_S as a cross-check when V_S is given.

### A2. Cabin altitude through the ISA engine
`cabin-altitude` takes pressure altitude and the maximum differential (psi or hPa), computes p_cabin = p_ISA(h) + Δp, inverts ISA for the cabin altitude, and clamps at a selected cabin altitude when less differential suffices, reporting the differential in use. The PHAK Figure 7-41 example (28,000 ft, 6.1 psid → 8,000 ft) is the vector at the figure's 0.1 psi rounding; ISA constants come from the existing atmosphere engine (ICAO Doc 7488 and US76, identical to 32 km).

### A3. Hydroplaning with the constant as a named choice
Default V_p = 9√p kt (NASA TN D-2056 eq. 4; PHAK example 36 psi → 54 kt), with the AFH's 8.6√p as a named option and the difference shown. The tool states the report's validity (24–150 psi, water deeper than tread grooves) and returns `OUT_OF_DOMAIN` outside it.

### A4. Circling radius from the order's formula
`circling-radius` implements Order 8260.3F formulas 2-7-1 (true airspeed from indicated at the altitude) and 2-7-2 (circling approach radius with bank and the S term) with Table 2-7-1's category speeds, banks, and S values, minimum 1.30 NM, reporting the inputs the order uses. The TPP legend's banded table is a documentation task: read it from the legend, record the assumptions that reproduce it, and only then show it beside the formula. Pre-2013 fixed radii (1.3, 1.5, 1.7, 2.3, 4.5 NM) are shown as the legacy values, dated.

### A5. Physiology as dated reference data with the FAA's caution
`oxygen-requirement` evaluates the 91.211 thresholds (and 135.89 or 121.329 by selection) against a cabin altitude and duration, returning which paragraph applies, from `data/regulations.json`. `time-of-useful-consciousness` returns the AC 61-107B Figure 2-3 range (with the rapid-decompression column) and PHAK Figure 17-1's range beside it, both labeled, interpolating nothing (altitudes between rows report the two bracketing rows), and always shows the AC's caution about individual variation.

### A6. Fuel density by temperature
`fuel-density-temperature` takes fuel type (Jet A/A-1, avgas 100LL, wide-cut) or a density at 15 °C and a temperature, and returns density and the weight of a volume by ρ(T) = ρ₁₅ / (1 + β (T − 15 °C)) with β = 0.00099 /°C for kerosine-type fuels (Chevron, vendor guidance), avgas 0.715 g/mL and Jet A 0.810 g/mL at 15 °C from the same review labeled typical, and the ASTM D1655 density range (775–840 kg/m³) as the "your batch varies" note. Measurement Canada's Jet A VCF table (base 800 kg/m³) gives vectors at −40, −20, 0, 10, 20, 30, 40 °C with tolerance 0.0015 (the linear model's drift at the extremes is shown). The existing fuel-weight tool gains a link to it.

### A7. Verification
- V_A: the derived example (105 KIAS at 2,550 lb → 93.0 KIAS at 2,000 lb) and the invariant V_A(W_max) = V_A,max.
- Cabin: PHAK Figure 7-41 rows within 0.1 psi; the computed 8.0 psid at 25,000 ft → 2,420 ft (±20 ft) as a second vector.
- Hydroplaning: 36 psi → 54 kt exactly; AFH 24 psi → 42 kt (±0.5 kt).
- Climb requirement: IPH example (3,000 − 1,221) / 5 = 356 ft/NM and 297 ft/NM at 180 kt → 891 fpm (±1 fpm).
- RVR: every row of 91.175(h)(2) exactly; EASA Table 8 factors exactly.
- Category: 97.3 boundaries at 90.9, 91, 120.9, 121, 140.9, 141, 165.9, 166 kt.
- Circling: Table 2-7-1 rows through formulas 2-7-1 and 2-7-2 at 1,000 ft MSL and airport elevations 0, 2,000, 5,000 ft; minimum 1.30 NM.
- Oxygen and TUC: the regulation thresholds and every table row, exactly.
- Fuel: the seven Measurement Canada VCF points within 0.0015; Chevron's 1.000 gal at 0 °C → 1.040 gal at 40 °C.

## Tool inventory

| Group | Operations | Endpoints | Reference | Independent vector |
|---|---|---|---|---|
| `aviation.performance` | va-for-weight, hydroplaning-speed, climb-requirement | 3 | 14 CFR 23.335; NASA TN D-2056; PHAK; IPH | PHAK 36 psi; IPH 356 ft/NM and 892 fpm |
| `aviation.altimetry` | cabin-altitude | 1 | PHAK ch. 7; US76 | Figure 7-41 |
| `aviation.ifr` | rvr-visibility, approach-category, circling-radius | 3 | 14 CFR 91.175, 97.3; Order 8260.3F; EASA AMC10 | the regulation tables; Table 2-7-1 |
| `aviation.physiology` | oxygen-requirement, time-of-useful-consciousness | 2 | 14 CFR 91.211, 135.89, 121.329; AC 61-107B; PHAK ch. 17 | the tables |
| `aviation.loading` | fuel-density-temperature | 1 | Chevron FTR (vendor); Measurement Canada VCF; ASTM D1655 (cited) | VCF table |
| **Total** | **10** | **10** | | |

The aviation rollup moves from 97 to 107 operations.

## Risks / Trade-offs

- **[Readers treat TUC or oxygen output as medical fact]** → The AC's caution is always shown; the sentence quotes the table as a range; no interpolation.
- **[Handbooks disagree (hydroplaning, TUC)]** → Both values shown, each named; the default is the primary source.
- **[The circling table's provenance]** → Formula first; table only after it is read from the legend (task 7).

## Migration Plan

Not applicable. New tools start experimental.

## Open Questions

- Whether EASA amended AMC10 CAT.OP.MPA.110 after Issue 2 Amendment 21 (the UK CAA has a 2025 edit); checked at the source in task 1.
