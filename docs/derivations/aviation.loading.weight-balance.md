<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Weight and balance (`aviation.loading.weight-balance`)

## Method

The loaded weight is the sum of the station weights, and the CG is the total moment divided by that weight, all measured from the POH datum. With a fuel burn, the landing state removes that fuel at the fuel arm and recomputes the CG. With a CG envelope from the POH, each state is checked against the polygon, and the result says inside or outside, never safe or legal. With LEMAC and MAC, the CG is also given in percent of MAC.

## Equations

- W = Σ wᵢ, M = Σ wᵢ aᵢ, CG = M / W.
- Landing: W' = W − b, M' = M − b a_fuel, CG' = M' / W'.
- %MAC = (CG − LEMAC) / MAC × 100.
- Envelope: even-odd rule on the (CG, weight) polygon, with points on an edge counted inside.

## Symbols and units

wᵢ station weight (lb by default), aᵢ arm from the datum (in, negative forward of it), M moment (lb·in), b fuel burn, a_fuel fuel arm (defaults to the arm of a station named Fuel), LEMAC and MAC in inches.

## Domain

One or more stations with a name, a non-negative weight, and an arm. The total weight must be positive. The envelope needs at least three corner points in order around it.

## Approximations

None: exact arithmetic on the data entered. The answer is only as good as the POH/AFM data and the aircraft's current weight and balance record, which govern.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aircraft Weight and Balance Handbook (FAA-H-8083-1B)
- sourceEdition: 2016
- sourceLocator: Figures 6-1 to 6-4 (light twin loaded to 5,064 lb, moment 215,093 lb·in, CG 42.47 in, inside the CG range); also figure 3-5 (2,006 lb, 65,756 lb·in, CG 32.8 in) and figure 3-19 (CG at station 161 with MAC 144 to 206 is 27.4% MAC)
- independent: yes
- inputs: airplane 3,404 lb at 35.28 in, fuel 840 lb at 61 in, front seat 320 lb at 37 in, row 2 310 lb at 75 in, forward baggage 100 lb at −15 in, aft baggage 90 lb at 113 in, and the figure 6-1 CG range (closed at the 3,404 lb empty weight)
- outputs: 5,064 lb, CG 42.47 in, inside the envelope
- tolerance: the printed rounding (0.005 in for the CG, 0.5 lb·in for the moment)
- verifiedBy: golden vectors v021 to v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: separate Python weight-and-balance arithmetic (FAA-H-8083-1B chapter 2) at 20 loadings of 1 to 7 stations with arms from −20 in to 120 in (within 1e-12 relative)
- `core/vectors/aviation.loading.weight-balance.jsonl`: those vectors plus the three handbook examples, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `weight_balance_invariants`: moving the datum moves the CG by the same amount, station order does not matter, a station added at the CG leaves it unchanged, and shifting weight w by d moves the CG by w·d/W
