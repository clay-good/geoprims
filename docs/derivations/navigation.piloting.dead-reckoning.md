<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Dead reckoning position (`navigation.piloting.dead-reckoning`)

## Method

The run D (a distance, or speed × time) along the course C from a known position. Under 100 NM it is worked as on a Mercator chart: Mercator sailing with meridional parts on WGS 84. From 100 to 600 NM it is the exact rhumb line on WGS 84 (Karney). The two are compared and the gap reported.

## Equations

- L₂ = L₁ + D cos C / 60; DLo = (M(L₂) − M(L₁)) tan C, or D sin C / cos L₁ due east or west.
- M(L) = (10800/π)[atanh(sin L) − e·atanh(e sin L)].

## Symbols and units

Latitude and longitude in degrees; course degrees true; speed knots; time hours; D nautical miles.

## Domain

Runs from 0 to 600 NM (`LIMIT_EXCEEDED` above) that stay out of the polar cap (`OUT_OF_DOMAIN` past 89°).

## Approximations

The Mercator plot's distance counts a minute of latitude as one nautical mile; the exact rhumb line on the ellipsoid differs by a few hundredths of a mile over 100 NM, and the result reports the gap.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9
- sourceEdition: 2024 edition
- sourceLocator: Chapter 10, Dead Reckoning, and §1306 Mercator sailing
- independent: no (Bowditch works no numeric dead-reckoning example; the method is its Mercator sailing)
- inputs: 36°55'N 76°00'W, 050° at 15 kn for 2 h
- outputs: A 30 NM run; the Mercator plot and the exact rhumb line agree within 0.05 NM
- tolerance: 0.05 NM
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/piloting.rs`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_piloting.py dead-reckoning`: Python Mercator sailing at 20 random runs under 100 NM (1e-10° latitude, 1e-9° longitude) and the over-600 NM refusal.
- `core/vectors/navigation.piloting.dead-reckoning.jsonl`: those vectors, run through the core on every build.

## Invariants

- `dead_reckoning_runs`: the run is speed × time; the two methods agree within 0.05 NM under 100 NM; 300 NM uses the rhumb line; 700 NM is refused.
