# Tasks

- [ ] 1 Add the sources to `data/sources-ledger.json` (Cochran 1977, EPA QA/G-5S, Osada 2002, Blackman & Vigna 2021, Buckland 2001 and 2015, USFS GTR SRS-80, QGIS and ArcGIS grid docs as vendor) → verify: the ledger gate passes with each section read named
- [ ] 2 xoshiro256** and splitmix64 in `gp-base` behind the sampling feature, with the double conversion → verify: the seed-42 and seed-0 reference vectors; the import lint still passes
- [ ] 3 `geometry.sampling.fishnet` and `hex-grid` (CRS choice, origin, rotation, clip, inside fraction, distortion) → verify: QGIS parity vectors within 1 mm; the 500 km refusal
- [ ] 4 `geometry.sampling.random-points` (triangulate, area-weighted choice, Osada) → verify: all-inside and uniform-by-area scenarios; byte parity with the NumPy reimplementation over 20 seeds
- [ ] 5 `geometry.sampling.systematic-points` and `stratified-points` → verify: triangular spacing, empty-stratum, and reproducibility scenarios; QGIS regular-points parity with a fixed start
- [ ] 6 `geometry.sampling.transects` and `points-along-line` → verify: the 10-transect and last-point scenarios; R `dssd` parity for the systematic parallel design
- [ ] 7 Limits, exports with properties, map drawing of grids, points, and transects → verify: the too-many-points scenario; `visual-purpose.test.mjs` reaches each tool; export tests byte-compare the primary examples
- [ ] 8 Taxonomy group, hubs, glossary, derivation notes, search questions → verify: the full gate sequence passes and each tool's questions rank it first
