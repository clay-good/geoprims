## 1. Routes and permalinks

- [x] 1.1 Implement the route-map gate and the redirects file; verify the out-of-map and alias-canonical scenarios (apps/web/scripts/routes.mjs runs in the web build: it classifies all 289 pages against the route map, fails naming anything outside it, enforces the canonical-to-parent and noindex rules per class, and writes dist/_redirects from data/redirects.json. The out-of-map scenario is verified; the canonical half of the alias scenario is verified on generated endpoints, since alias slug routes are not built yet)
- [x] 1.2 Implement the fragment encoder/decoder in the core with the shared vector file used by web and MCP tests; verify the shared-encoding and unknown-version scenarios

## 2. Platform topology

- [ ] 2.1 Configure the single Worker with static assets, `run_worker_first` for `/api/reports*`, D1 binding, secrets, cron, and observability off; verify that static requests never invoke the Worker and API requests do (fall back to a route-bound Worker if not)
- [ ] 2.2 Configure R2 on `assets.geoprims.com` with CORS limited to the site origin; verify a cross-origin request from another origin is refused
- [ ] 2.3 Configure the enumerated CSP with build-computed script hashes and `inlineStylesheets: 'never'`; verify the header smoke test and a zero-violation CSP report in end-to-end runs (done: the enumerated CSP with build-computed hashes for Astro's two island bootstrap scripts and its one island style, in _headers and a meta tag, and the smoke test; a manual run had zero violations. Pending: the automated end-to-end CSP report, with the Playwright suites)
- [ ] 2.4 Disable Bot Fight Mode features and add the WAF rule; verify the no-cookie scenario

## 3. Page chrome

- [x] 3.1 Implement the canonical anatomy and the anatomy gate; verify the anatomy-gate scenario (apps/web/test/anatomy.test.mjs holds every tool page to the contract's region order, requires the non-optional regions on every stable page, and pins exactly one report button, in the answer card. The developer block moved below related tools to match the contract; the header regions and the report button's home are now recorded as superseded by redesign-minimal-shell, and "Terms on this page" is listed as the region it is)
- [ ] 3.2 Implement the 320 px header and overflow menu; verify the 320 px header scenario
- [ ] 3.3 Implement notice ranking, the two-visible limit, and "N more notes"; verify the many-notices scenario at 390 × 844
- [ ] 3.4 Add the chrome copy lint (sentence case, no all caps); verify the no-all-caps scenario

## 4. Manifest extensions

- [x] 4.1 Add every extension to the meta-schema, closed to unknown fields; verify the unknown-extension and too-many-core-inputs scenarios (tools/trust/metaschema.mjs checks the built catalog: only the contract's tool and field extensions, well-formed x-comparison, x-clock-default, and x-primary-example, x-core on inputs only and at most 5 with a list of rows counting once; tested with x-color and six-core fixtures)
- [x] 4.2 Implement the sentence-template renderer in the core with readability lint; verify the conditional and unit-profile scenarios
- [x] 4.3 Create the glossary schema and missing-term gate; verify the RPP scenario (data/glossary.json with 92 entries, each a plain definition of at most 40 words citing a sources-ledger row, plus reasoned exclusions for symbols and example values; tools/trust/glossary.mjs fails the build naming the tool and term, keeps relatedTools in step with the catalog, and is tested with the RPP fixture)
- [ ] 4.4 Implement primary-example designation and the parity gate across page, button, OG, hero card, MCP, and explainer; verify the example-parity scenario (done: x-primary-example is designated and validated, and the parity gate covers the page's shipped answer, the printed agent call, the home page's featured card, and the MCP default run for every tool, with divergent fixtures. Pending: OG images and explainers, neither of which the build emits yet)
- [x] 4.5 Implement the decimal-separator rule in the core parser shared by all surfaces; verify both scenarios

## 5. Codes, profiles, and report API

- [x] 5.1 Create `data/codes.json` from the seed registry with message templates and the completeness gate; verify the caution-first and completeness scenarios
- [ ] 5.2 Encode the reference profile and viewports in the shared test configuration and `docs/performance.md`; verify every performance and mobile gate names the profile version
- [x] 5.3 Create the shared limits constants file (`data/report-limits.json`) imported by client, Worker, migration generator, and MCP; verify the worst-case payload scenario (MCP imports it now; client, Worker, and migration generator pending)
- [ ] 5.4 Implement the config and submit endpoints and client submission behavior per the contract; verify the disabled, wrong-method, and slow-note scenarios
