## 1. Citations

- [ ] 1.1 Define the citation schema and `data/citations/<domain>.json` layout; verify the schema rejects missing edition or locator (coverage scenario)
- [ ] 1.2 Write citation records for every tool, including sourced assumptions; verify the coverage gate passes with zero exemptions
- [ ] 1.3 Implement the shared citation renderer (page, print, copy-with-reference, calculation sheet, MCP `meta.references`); verify the copy-with-reference scenario
- [ ] 1.4 Replace any reproduced copyrighted table with a cited input field; verify the sight-distance scenario and a lint for table-shaped constants in the flagged tools
- [x] 1.5 Build the inverse source map; verify the edition-rollover scenario (tools/trust/ledger.mjs maps every citation to its ledger row; rolling a row to a new edition fails the build and names the citing tools)

## 2. Freshness

- [x] 2.1 Create `data/sources-ledger.json` with every required row; verify the ledger-completeness gate
- [x] 2.2 Implement the superseded-edition, overdue-verification, and model-expiry gates; verify each scenario with fixture ledgers
- [x] 2.3 Implement monotonic provenance stamps against the base branch; verify the stale-revert scenario
- [ ] 2.4 Add the monthly free-access probe workflow that opens issues; verify against a fixture with a dead link
- [ ] 2.5 Link regulatory reference entries to ledger rows with "Rules as of" rendering; verify the Part 108 status-change scenario

## 3. Correctness program

- [ ] 3.1 Write the derivation template and one derivation per hero tool first; verify the missing-derivation gate
- [ ] 3.2 Extend golden vectors into worked-example fixtures with source fields and tolerance ceilings; verify the independent-example gate
- [ ] 3.3 Implement the dimension lint over manifests and core annotations; verify it catches a deliberate unit-name mismatch
- [x] 3.4 Implement the bounds fuzzer with minimization; verify it finds a seeded trap (tools/fuzz/fuzz.test.mjs: seeded mutations of every tool's example through Wasm in a timed worker, with minimization; it found 16 real defects (non-finite results, a trap, and two hangs), each fixed and pinned as a regression; 1,000 cases per tool pass)
- [ ] 3.5 Implement example parity (page example = button example = MCP example); verify with a deliberately divergent fixture
- [ ] 3.6 Create `docs/review-signoffs.md` and the pending-review disclosure; verify the unreviewed-domain scenario
- [ ] 3.7 Implement the claims-honesty gate; verify the overclaim scenario
- [ ] 3.8 Implement the both-surfaces gate; verify the unreachable-tool scenario

## 4. Proof display

- [ ] 4.1 Add the core `trace` output and MCP `explain: true`; verify byte-identical traces across web and MCP
- [ ] 4.2 Build the "How we got this" panel with substituted formulas, responsive default state, and print expansion; verify the substituted-formula and printed-proof scenarios
- [ ] 4.3 Implement limitation banners shared with MCP; verify the banner-text scenario
- [ ] 4.4 Implement context bands with cited bases; verify the GSD scenario
- [ ] 4.5 Build `/sources`, `/methodology`, `/changelog` with result-change labels, and vector downloads; verify the sources-page and vector-download scenarios (built: /sources from the ledger with citing tools, /methodology stating what is and is not verified, and per-tool /vectors/<id>.jsonl downloads; pending: /changelog)

## 5. Reviewer recruitment

- [ ] 5.1 Recruit and schedule reviewers for aviation, drone, survey, and geodesy hero tools; verify signed records exist in `docs/review-signoffs.md` before those tools are marketed as reviewed
