# Tasks

- [ ] 1 Add Ghilani, *Adjustment Computations*, 6th ed., to `data/sources-ledger.json`; confirm the 16th-edition chapter 12 partitioning sections at the source and record them → verify: the ledger gate passes with the sections named
- [ ] 2 `survey.cogo.area-partition` (three cases, Brent's method with declared tolerance and limit, station-and-offset output, refusals) → verify: the textbook partition vectors; the sum-of-areas and on-boundary invariants on 1,000 random simple polygons; `NO_SOLUTION` and `DEGENERATE_GEOMETRY` cases
- [ ] 3 `survey.cogo.helmert-3d-fit` (linearized Bursa-Wolf, both conventions, standard errors, warnings, send-to Helmert) → verify: the textbook data set to printed decimals; the synthetic recovery invariant; `NO_REDUNDANCY` and `SCALE_SUSPECT` cases
- [ ] 4 `survey.reduction.level-network` (weights by length, setups, or sigma; Cholesky; chi-square; equation display up to 12 unknowns; 500 cap) → verify: the textbook network to the millimeter; the one-loop equivalence; the no-fixed-benchmark refusal
- [ ] 5 Calculation sheets, plane sketches (the partition line on the figure; the network as a graph), derivation notes, hubs, glossary, search questions → verify: `visual-purpose.test.mjs` reaches each tool; the full gate sequence passes and each tool's questions rank it first
