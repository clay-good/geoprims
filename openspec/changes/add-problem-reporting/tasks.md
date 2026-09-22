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
- [x] 3.2 Add the payload-schema lock (no new fields without a spec change); verify it fails when a field is added (`tools/trust/report-schema.test.mjs` reads the key list from the contracts/report-api spec and requires the Worker's accepted keys, the web dialog's payload with and without inputs, and, through the Worker's list, the MCP report to match it exactly; it fails on an added field in the spec or the payload, and on any identifying name)

## 4. Triage and corrections

- [x] 4.1 Write `docs/runbooks/problem-reports.md` (queries, status transitions, SLAs, launch checklist); verify every query runs against a seeded local D1 (`worker/test/runbook.test.mjs` runs every SQL block in the runbook against the real migration with seeded reports; the D1 is Node's SQLite, the same engine, as `wrangler` is not in CI yet)
- [x] 4.2 Write the reproduction script (report id → recompute on reported and current builds); verify on seeded reports (`worker/scripts/reproduce.mjs` restores the inputs from the report's permalink with the core's decoder, recomputes on the current build and on the reported one (the current one when its module hash matches, or a release's `dist/wasm` given with `--reported-wasm`), and marks each field that differs; `worker/test/reproduce.test.mjs` covers a faithful report, a wrong reported value, and a report sent without inputs)
- [x] 4.3 Write the status-transition wrappers enforcing `primary_source` for confirmed and `fixed_in_version` for fixed; verify rejection of invalid transitions (`worker/src/triage.mjs` and the `worker/scripts/triage.mjs` CLI: only the workflow's own moves, a primary source for confirmed, a semantic version for fixed, a resolution note for each close, and a WHERE on the old status so a stale move changes nothing; `worker/test/triage.test.mjs` walks a report through, refuses ten invalid moves, and checks the triage order and the CLI's SQL quoting)
- [x] 4.4 Create `known-issues.json` schema, the `/known-issues` page, and tool-page banners; verify the banner scenario (data/known-issues.json with a build-time validator, the /known-issues page, and the tool-page banner for confirmed issues; `apps/web/test/known-issues.test.mjs` drives the banner scenario with fixtures: only a confirmed, unfixed issue shows "Known issue: … Workaround: …" linking to its entry, and incomplete entries are refused)
- [x] 4.5 Add the "Result change" changelog label and the 90-day tool-page notice; verify the result-change scenario (the changelog labels result-change entries, which must list the superseded vectors (tools/trust/changelog.mjs), and tool pages show "A published result changed on <date>: …" with a changelog link for 90 days, tested in `apps/web/test/notices.test.mjs` and seen on the fuel-plan page after its 2026-09-22 result change)
- [x] 4.6 Add `.github/ISSUE_TEMPLATE/wrong-answer.yml` and `config.yml`; verify required-field validation (the form carries the `correctness` label and marks the tool or URL, inputs, answer received, answer expected, and published source as required; `config.yml` points security reports to a private advisory; `tools/github/issue-form.test.mjs` checks each required field and that every link to the form names a file that exists. GitHub's own blocking of an incomplete form is verified once the repository is public)
- [ ] 4.7 Write the approval-gated GitHub mirror script with a fine-grained token; verify the structured-mirror scenario against a test repository
- [x] 4.8 Build the monthly `/quality` summary from statuses and the changelog; verify with seeded data (/quality/ shows confirmed defects open, under investigation, and fixed, the median days to fix, result changes and fixes published, and a row per month, all derived from data/known-issues.json and data/changelog.json, both reviewed in pull requests. Reports received is reported as not measured yet rather than as a zero, because only the Worker can count it and reporting is off. Tests check the counts against the changelog, the month rows, the honest absences, and that the page adds no script of its own)

## 5. MCP

- [x] 5.1 Implement `geoprims_report_problem` (payload plus prefilled link, no network); verify the agent-prepared scenario and the network-sandbox audit (the agent-prepared scenario is in `mcp/server.test.mjs`, and `mcp/network.test.mjs` now prepares a report inside the no-network sandbox after every worked example, with no connection attempt)

## 6. Privacy and launch

- [x] 6.1 Update the privacy page (what is sent, Turnstile, retention, no IP storage); verify each statement against the implementation checklist (/privacy/ lists every field a report carries, the x-private and "(withheld)" rules, the note cap read from data/report-limits.json so it cannot drift, Turnstile as the only third-party code and only while the dialog is open, the salted daily counter in place of an address, and the retention rules the cron enforces; a test checks each statement)
- [ ] 6.2 Run the launch verification checklist on production (round trip, duplicate, quota, offline, kill switch); verify and record results in the runbook
