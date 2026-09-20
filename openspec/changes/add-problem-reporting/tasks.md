## 1. Client

- [x] 1.1 Add the single report button to the tool header component with lazy import; verify the every-tool and nothing-before-click scenarios in end-to-end tests (build test: one button per tool page and no bot-check script in any page's HTML; the dialog is a separate chunk imported on click)
- [ ] 1.2 Build the dialog (disclosure, payload preview, include toggle, 280-character note with counter, kind selector, accessible live status); verify the preview and exclude scenarios and an axe pass in every theme (built: disclosure, full payload preview, include toggle, 280-character counter, kind, live status; preview and exclude verified by tests and in the browser; pending: the axe pass)
- [x] 1.3 Implement payload construction with versions, build hash, sanitized permalink, `x-private` omission, and display class; verify the no-identifying-fields schema check and the private-input scenario
- [x] 1.4 Implement paused, offline (copy report), and success states; verify the offline scenario (the dialog's opening state and its send result are pure functions in src/lib/report.js, tested over offline, the kill switch, an unreachable config, and every send status: only the uniform 202 reads as sent)
- [x] 1.5 Ensure the service worker bypasses `/api/*`; verify with a service-worker test (the worker returns without calling respondWith for any /api/ path, on GET and POST, online and offline; the test fails if the bypass is removed)

## 2. Worker and D1

- [ ] 2.1 Create the D1 database and migration `0001_problem_reports.sql` per design R6; verify with `wrangler d1 migrations apply --local` in CI (built: the migration, generated from data/report-limits.json and applied to Node's SQLite in the Worker tests; pending: wrangler in CI)
- [x] 2.2 Implement the Worker (config endpoint, POST validation order, 24 KB streaming cap, catalog import, uniform 202, security headers, logging disabled); verify the unknown-tool, oversized, and logging scenarios
- [x] 2.3 Implement the keyed daily reporter hash, attempt and accepted caps, dedupe, and the atomic batch insert; verify the uniform-response and no-address scenarios
- [ ] 2.4 Implement Turnstile siteverify with action and hostname checks and timeout; verify with Turnstile test keys (built: siteverify with action and hostname checks and a 5 s timeout, tested with a stubbed siteverify; pending: Cloudflare's test keys against a deployed Worker)
- [x] 2.5 Implement the daily cron cleanup; verify the scheduled-cleanup scenario
- [x] 2.6 Implement the kill switch (503 config when unconfigured); verify the paused state end to end
- [ ] 2.7 Document and apply the WAF rate rule; verify with a scripted burst against staging

## 3. Gates

- [ ] 3.1 Write the feedback-loop gate (one button per tool, lazy import, limits agree across client, Worker, and D1, SW bypass, CSP entries); verify it fails on each targeted bad fixture
- [ ] 3.2 Add the payload-schema lock (no new fields without a spec change); verify it fails when a field is added

## 4. Triage and corrections

- [ ] 4.1 Write `docs/runbooks/problem-reports.md` (queries, status transitions, SLAs, launch checklist); verify every query runs against a seeded local D1
- [ ] 4.2 Write the reproduction script (report id → recompute on reported and current builds); verify on seeded reports
- [ ] 4.3 Write the status-transition wrappers enforcing `primary_source` for confirmed and `fixed_in_version` for fixed; verify rejection of invalid transitions
- [ ] 4.4 Create `known-issues.json` schema, the `/known-issues` page, and tool-page banners; verify the banner scenario (built: data/known-issues.json with a build-time validator, the /known-issues page, and the tool-page banner for confirmed issues)
- [ ] 4.5 Add the "Result change" changelog label and the 90-day tool-page notice; verify the result-change scenario
- [ ] 4.6 Add `.github/ISSUE_TEMPLATE/wrong-answer.yml` and `config.yml`; verify required-field validation
- [ ] 4.7 Write the approval-gated GitHub mirror script with a fine-grained token; verify the structured-mirror scenario against a test repository
- [ ] 4.8 Build the monthly `/quality` summary from statuses and the changelog; verify with seeded data

## 5. MCP

- [ ] 5.1 Implement `geoprims_report_problem` (payload plus prefilled link, no network); verify the agent-prepared scenario and the network-sandbox audit

## 6. Privacy and launch

- [ ] 6.1 Update the privacy page (what is sent, Turnstile, retention, no IP storage); verify each statement against the implementation checklist
- [ ] 6.2 Run the launch verification checklist on production (round trip, duplicate, quota, offline, kill switch); verify and record results in the runbook
