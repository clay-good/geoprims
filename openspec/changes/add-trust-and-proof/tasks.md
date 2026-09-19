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

- [ ] 3.1 Write the derivation template and one derivation per hero tool first; verify the missing-derivation gate (template and gate done in tools/trust/promotion.mjs; notes written for the twenty stable tools in docs/derivations/, including the pilot hero tools pressure altitude, density altitude, runway components, and the wind triangle, and the developer hero tools geohash, tile from point, tile bounds, H3 grid disk, and haversine; pending: the remaining hero tools, tracked in docs/launch/hero-tools.md)
- [ ] 3.2 Extend golden vectors into worked-example fixtures with source fields and tolerance ceilings; verify the independent-example gate (independent-example gate done: each derivation's worked example records publisher, title, edition, locator, tolerance, and verifier; pending: per-domain tolerance ceilings)
- [x] 3.3 Implement the dimension lint over manifests and core annotations; verify it catches a deliberate unit-name mismatch (tools/trust/dimensions.mjs; it found nT, A, minute, and hour outputs marked dimensionless, now annotated with fixed units)
- [x] 3.4 Implement the bounds fuzzer with minimization; verify it finds a seeded trap (tools/fuzz/fuzz.test.mjs: seeded mutations of every tool's example through Wasm in a timed worker, with minimization; it found 16 real defects (non-finite results, a trap, and two hangs), each fixed and pinned as a regression; 1,000 cases per tool pass)
- [ ] 3.5 Implement example parity (page example = button example = MCP example); verify with a deliberately divergent fixture
- [x] 3.6 Create `docs/review-signoffs.md` and the pending-review disclosure; verify the unreviewed-domain scenario (tools/trust/signoffs.mjs parses the record, with 12-month expiry; each domain page and each tool's "How we got this" panel says "Not yet independently reviewed by a <practitioner>" until a signed row exists)
- [x] 3.7 Implement the claims-honesty gate; verify the overclaim scenario (tools/trust/claims.mjs checks every built page, llms.txt, AGENTS.md, and the READMEs for counts, "all experimental" statements, "checked against" statements without a differential suite, unrecorded reviews, and offline claims without a service worker; it caught the home page still saying everything was experimental and claiming offline use)
- [x] 3.8 Implement the both-surfaces gate; verify the unreachable-tool scenario (every stable tool's title ranks it in the top 5 of the core search that both the web palette and `geoprims_search` use, its example runs, and describe advertises its inputs)

## 4. Proof display

- [ ] 4.1 Add the core `trace` output and MCP `explain: true`; verify byte-identical traces across web and MCP
- [ ] 4.2 Build the "How we got this" panel with substituted formulas, responsive default state, and print expansion; verify the substituted-formula and printed-proof scenarios
- [ ] 4.3 Implement limitation banners shared with MCP; verify the banner-text scenario
- [ ] 4.4 Implement context bands with cited bases; verify the GSD scenario
- [x] 4.5 Build `/sources`, `/methodology`, `/changelog` with result-change labels, and vector downloads; verify the sources-page and vector-download scenarios (/sources from the ledger with citing tools, /methodology, per-tool /vectors/<id>.jsonl downloads, and /changelog from data/changelog.json with result changes labeled; tools/trust/changelog.mjs fails the build when a superseded vector has no result-change entry)

## 5. Reviewer recruitment

- [ ] 5.1 Recruit and schedule reviewers for aviation, drone, survey, and geodesy hero tools; verify signed records exist in `docs/review-signoffs.md` before those tools are marketed as reviewed
