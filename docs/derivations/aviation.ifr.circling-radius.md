<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Circling approach radius (`aviation.ifr.circling-radius`)

## Method

The FAA builds a circling approach's protected area from arcs drawn around each runway threshold. Their radius, the circling approach radius (CAR), comes from Order 8260.3G paragraph 2-7-1.a. The category's indicated airspeed is turned into true airspeed at the airport elevation plus the height above the airport, which is never taken as less than 1,000 ft (Formula 2-7-1). The radius is then twice the turn radius at the category's bank, with a 25 kt wind allowance added to the true airspeed, plus a straight segment (Formula 2-7-2). Table 2-7-1 gives each category's speed, bank, and straight segment, and the radius is never less than 1.30 NM. The tool also shows the fixed radii charted before Order 8260.3 Change 21, from the Instrument Procedures Handbook, because older procedures may still be built on them.

## Equations

- h = 0.00198 × (elevation + k), with k = max(1,000 ft, circling altitude − elevation), or 1,000 ft without a circling altitude.
- V_KTAS = V_KIAS × 171233 × √(303 − h) / (288 − h)^2.628 (Formula 2-7-1).
- CAR = 2 × (V_KTAS + 25)² / (tan(bank) × 68625.4) + S (Formula 2-7-2), reported as max(CAR, 1.30 NM).
- Table 2-7-1: A 90 KIAS, 25°, 0.4 NM; B 120, 25°, 0.4; C 140, 20°, 0.5; D 165, 20°, 0.6; E 200, 22°, 0.7.

## Symbols and units

Elevation, circling altitude, and k in feet; speeds in knots; bank in degrees; S and CAR in nautical miles. 68625.4 is g in nautical miles per hour squared (32.174 ft/s² expressed in NM/h²), which turns (V + 25)² / (g tan bank) into a turn radius in NM.

## Domain

Airport elevation from −2,000 to 15,000 ft, and the circling altitude from the airport elevation to 10,000 ft above it; otherwise OUT_OF_DOMAIN. A circling altitude less than 1,000 ft above the airport uses 1,000 ft, as the order says.

## Approximations

Formula 2-7-1 is the order's own standard-atmosphere conversion, not an exact compressible-flow airspeed conversion, and the 25 kt is the order's fixed wind allowance; the tool uses both as written, because the point is the radius the procedure designer used. It reports the design radius; restricted circling sectors and the chart's minimums are not modeled.

With a circling altitude, the tool also reports the radius the Terminal Procedures Publication legend prints for that MDA band (read in the d-TPP cycle 2611 front matter, effective October 29, 2026). The legend's numbers sit up to about 0.4 NM under the formula evaluated at the circling altitude, which looked like a different method until the assumption was found: every one of its 30 cells is Formula 2-7-2 at the band's lowest MDA (0, 1,001, 3,001, 5,001, 7,001, or 9,001 ft MSL, taken as elevation plus height), cut down (not rounded) to a tenth of a mile, and never under 1.3 NM. Rounding instead of cutting misses 12 cells, and taking each band's top misses 25. So the legend is a conservative chart aid for a band of MDAs, and the formula with your own MDA is the design value; the tool shows both.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Procedures Handbook
- sourceEdition: FAA-H-8083-16B
- sourceLocator: Chapter 4, beside Figure 4-2: a sea-level airport with a 1,500 ft HAA has a Category C CAR of 2.86 NM
- independent: yes
- inputs: Category C, airport elevation 0 ft, circling altitude 1,500 ft
- outputs: radius 2.86 NM (the formulas give 2.8644 NM)
- tolerance: 0.005 NM, the handbook's printed rounding
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/approach.rs` `circling_radius_invariants`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_circling.py`: a separate Python evaluation of Formulas 2-7-1 and 2-7-2 for every category at airport elevations of 0, 2,000, and 5,000 ft (the change design's verification set), four cases with a circling altitude (one under 1,000 ft above the airport), and two refusals; with the handbook's example, 22 vectors; then the legend's 30 printed cells, one per band and category; 52 vectors in `core/vectors/aviation.ifr.circling-radius.jsonl`.

## Invariants

- `core/crates/gp-aviation/tests/approach.rs` `circling_legend_is_the_formula_at_each_band_floor`: for every band above the lowest and every category, the legend radius equals the formula at the band's floor cut down to a tenth, and is the same anywhere in the band; no legend radius without a circling altitude.
- `core/crates/gp-aviation/tests/approach.rs` `circling_radius_invariants`: the handbook's 2.86 NM; the Category C scenario shows Table 2-7-1's inputs; Category A at sea level falls under 1.30 NM by the formula and reports the minimum, saying so; the height used is never under 1,000 ft; the radius grows with elevation and from Category A to E; the legacy radii; refusal of a circling altitude below the airport.
