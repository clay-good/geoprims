## 1. Routes and permalinks

- [ ] 1.1 Implement the route-map gate and the redirects file; verify the out-of-map and alias-canonical scenarios
- [ ] 1.2 Implement the fragment encoder/decoder in the core with the shared vector file used by web and MCP tests; verify the shared-encoding and unknown-version scenarios

## 2. Platform topology

- [ ] 2.1 Configure the single Worker with static assets, `run_worker_first` for `/api/reports*`, D1 binding, secrets, cron, and observability off; verify that static requests never invoke the Worker and API requests do (fall back to a route-bound Worker if not)
- [ ] 2.2 Configure R2 on `assets.geoprims.com` with CORS limited to the site origin; verify a cross-origin request from another origin is refused
- [ ] 2.3 Configure the enumerated CSP with build-computed script hashes and `inlineStylesheets: 'never'`; verify the header smoke test and a zero-violation CSP report in end-to-end runs
- [ ] 2.4 Disable Bot Fight Mode features and add the WAF rule; verify the no-cookie scenario

## 3. Page chrome

- [ ] 3.1 Implement the canonical anatomy and the anatomy gate; verify the anatomy-gate scenario
- [ ] 3.2 Implement the 320 px header and overflow menu; verify the 320 px header scenario
- [ ] 3.3 Implement notice ranking, the two-visible limit, and "N more notes"; verify the many-notices scenario at 390 × 844
- [ ] 3.4 Add the chrome copy lint (sentence case, no all caps); verify the no-all-caps scenario

## 4. Manifest extensions

- [ ] 4.1 Add every extension to the meta-schema, closed to unknown fields; verify the unknown-extension and too-many-core-inputs scenarios
- [ ] 4.2 Implement the sentence-template renderer in the core with readability lint; verify the conditional and unit-profile scenarios
- [ ] 4.3 Create the glossary schema and missing-term gate; verify the RPP scenario
- [ ] 4.4 Implement primary-example designation and the parity gate across page, button, OG, hero card, MCP, and explainer; verify the example-parity scenario
- [ ] 4.5 Implement the decimal-separator rule in the core parser shared by all surfaces; verify both scenarios

## 5. Codes, profiles, and report API

- [ ] 5.1 Create `data/codes.json` from the seed registry with message templates and the completeness gate; verify the caution-first and completeness scenarios
- [ ] 5.2 Encode the reference profile and viewports in the shared test configuration and `docs/performance.md`; verify every performance and mobile gate names the profile version
- [ ] 5.3 Create the shared limits constants file imported by client, Worker, migration generator, and MCP; verify the worst-case payload scenario
- [ ] 5.4 Implement the config and submit endpoints and client submission behavior per the contract; verify the disabled, wrong-method, and slow-note scenarios
