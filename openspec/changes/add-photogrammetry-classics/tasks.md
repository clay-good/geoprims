# Tasks

- [ ] 1 Add USACE EM 1110-1-1000 (1993) ch. 10 to `data/sources-ledger.json`; confirm the Wolf example numbers at the source → verify: the ledger gate passes with sections named
- [ ] 2 `drone.photogrammetry.photo-scale` (same GSD function as `gsd`), `relief-displacement`, `parallax-height`, `stereo-base` → verify: the Wolf 6-7 and 8-3 vectors, the Washington Monument vector, the Penn State airbase; the round-trip and scaling invariants; GSD bit-identical to the existing tool
- [ ] 3 Diagrams for the four tools; camera profile fields added for them in `data/profiles.json` → verify: `diagrams.test.mjs` and `visual-purpose.test.mjs` reach each; the profile gate passes
- [ ] 4 Explainer "Why overlap changes heights," hubs, glossary, derivation notes, search questions → verify: the full gate sequence passes and each tool's questions rank it first
