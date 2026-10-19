## Why

The practitioner review's second tier (`docs/research/07`, Tier 2) left a handful of aviation questions unanswered that pilots ask every week and answer today from memory or from ad-heavy pages: fuel weight when the ramp is hot or cold, the RVR-to-visibility conversion, the oxygen rule, how long useful consciousness lasts, maneuvering speed at today's weight, the cabin altitude a pressurization system will hold, the hydroplaning speed for these tires, whether the airplane can make the departure procedure's climb gradient with this wind, and the approach category and circling radius for this approach speed. Each has a dated regulation or a handbook formula with a published example. None needs data beyond small reference tables that are US Government works.

Depends on: `add-aviation-suite` (atmosphere, airspeed, altimetry, performance, loading), `add-practitioner-essentials` (IFR group, regulations data file), `add-trust-and-proof` (dated reference data).

## What Changes

Ten operations across existing aviation groups and one new group, `physiology`:

| Tool | Question | Source |
|---|---|---|
| `aviation.performance.va-for-weight` | What is maneuvering speed at this weight? | 14 CFR 23.335(c) (pre-2017 text) with V_S ∝ √W (PHAK-25C ch. 5); derived rule, labeled |
| `aviation.altimetry.cabin-altitude` | What cabin altitude does this differential hold at this flight level? | PHAK-25C ch. 7 (Figure 7-41 example); ISA from US Standard Atmosphere 1976 |
| `aviation.performance.hydroplaning-speed` | At what speed do these tires hydroplane? | Horne & Dreher, NASA TN D-2056 (1963), eq. 4; PHAK-25C ch. 11 example; AFH-3C variant as an option |
| `aviation.performance.climb-requirement` | Can I meet this departure's ft/NM with my rate of climb and this wind? | AIM 5-2-9; Instrument Procedures Handbook FAA-H-8083-16B ch. 1 example |
| `aviation.ifr.rvr-visibility` | What visibility does this RVR equal? | 14 CFR 91.175(h)(2) table; EASA AMC10 CAT.OP.MPA.110 Table 8 for met-visibility-to-RVR factors |
| `aviation.ifr.approach-category` | Which approach category is my Vref (or 1.3 V_S0)? | 14 CFR 97.3 |
| `aviation.ifr.circling-radius` | How big is the circling protected area for this category and altitude? | FAA Order 8260.3F §2-7-1 (formulas 2-7-1 and 2-7-2, Table 2-7-1) |
| `aviation.physiology.oxygen-requirement` | When must I use or provide oxygen? | 14 CFR 91.211 (and 135.89, 121.329 by selection), dated |
| `aviation.physiology.time-of-useful-consciousness` | How long at this altitude? | AC 61-107B Change 1 Figure 2-3 (with its caution), PHAK Figure 17-1 shown beside |
| `aviation.loading.fuel-density-temperature` | What does this fuel weigh at this temperature? | Chevron Aviation Fuels Technical Review (vendor, coefficient 0.00099 /°C for kerosine); Measurement Canada Jet A VCF table as vectors; ASTM D1655 cited |

The physiology and approach-reference tools are reference tools in the sense of the drone operations references: dated, cited, "rules as of" shown, never advice.

## Capabilities

### New Capabilities

- `aviation/performance-and-cabin`: Maneuvering speed by weight, cabin altitude, hydroplaning speed, climb requirement versus available.
- `aviation/approach-references`: RVR and visibility conversions, approach category, circling radius.
- `aviation/physiology-references`: Oxygen rules and time of useful consciousness as dated references.
- `aviation/fuel-density`: Fuel density and weight versus temperature.

### Modified Capabilities

None.

## Non-goals

- Aircraft-specific values (actual V_A, cabin differential, tire pressure). The reader enters them from the POH or AFM.
- The pilot-facing banded circling table from the TPP legend; the tool computes the order's formula and labels the table values unverified until read from the legend (design A4).
- Medical advice. The physiology tools quote the FAA's own tables and cautions.
- ICAO PANS-OPS circling values (paid source); noted only.

## Impact

- `core/gp-aviation` grows by ten operations; `data/taxonomy.json` adds `physiology` under `aviation`.
- `data/regulations.json` gains the 91.211, 135.89, 121.329, 91.175(h), 97.3, and TERPS entries, each with its eCFR or order citation and review date; the sources ledger gains NASA TN D-2056, AC 61-107B, FAA Order 8260.3F, FAA-H-8083-16B, EASA AMC & GM to Part-CAT, the Chevron review (vendor), and Measurement Canada's VCF table.
- Three explainers: maneuvering speed and weight, RVR and visibility, hypoxia and the oxygen rules.
