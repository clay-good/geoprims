## Why

A calculator site earns trust by being correct, and it stays correct only if wrong answers get reported and fixed fast. Pilots, surveyors, and drone operators notice when a number looks off. Today they have no channel short of a GitHub account.

A "Report a problem" button on every tool gives them a one-click path. The report carries the exact tool, inputs, outputs, and build, so a maintainer can reproduce the problem in seconds. A public known-issues page and a correction workflow close the loop.

The pattern is proven in the sister project roughlogic.com (`docs/research/06-roughlogic-patterns.md` §1). This change adopts it, with the improvements found in `docs/research/05-feedback-seo-ux-trust-mcp-distribution.md` §1.

Depends on: `establish-platform-foundation`, `build-web-experience`.

## What Changes

- **A "Report a problem" button in the header of every tool page.** It lazy-loads the dialog, so nothing loads until someone clicks.
- **A report dialog** that shows exactly what will be sent. The payload is the tool id, version, build hash, sanitized permalink, inputs, outputs, warnings, and an optional 280-character note. Personal data is never collected.
- **A single small Cloudflare Worker at `/api/reports*`** with bot protection that loads only in the dialog (Turnstile), strict validation, layered rate limits, dedupe, and uniform responses.
  - Reports are stored in **Cloudflare D1**. No IP addresses are stored; a daily keyed hash is used only for rate-limit counters, which are deleted after 14 days.
- **A kill switch.** If reporting is misconfigured or disabled, the button explains that and offers the GitHub issue form. Calculators keep working.
- **A triage and correction workflow:**
  - a runbook and scripted queries
  - response-time targets
  - a "result change" label on fixes that change numbers
  - a regression vector for every confirmed defect
  - an optional GitHub issue mirror of structured fields only, after a maintainer approves
- **A public, static "Known issues" page** generated at build time from a curated file. It lists open and fixed problems with workarounds and fix versions.
- **A GitHub "Wrong answer" issue form** as the second door.
- **An MCP `geoprims_report_problem` tool** that prepares a report link and payload for the human to review and send. The server itself never sends anything.
- **A CI gate** that checks the button is wired on every tool and that size limits agree across the client, Worker, and D1 schema.

## Capabilities

### New Capabilities

- `feedback/problem-reports`: The report button, dialog, payload, Worker endpoint, D1 storage, abuse controls, retention, and privacy rules.
- `feedback/triage-and-corrections`: How reports become fixes: triage targets, reproduction, correction workflow, known-issues page, issue mirroring, and MCP report preparation.

### Modified Capabilities

None (see Impact for the edits to unarchived foundation and MCP specs).

## Non-goals

- No accounts, email collection, or reply threads. Reporters are anonymous. Follow-up happens on the public known-issues page or GitHub.
- No analytics or tracking derived from reports.
- No live read API over reports. The known-issues page is static and curated.
- No general contact form or feature-request inbox. That goes to GitHub Discussions.

## Impact

- **Amends `platform/privacy-and-security`** (unarchived; edited in place):
  - The rule that "user inputs never leave the device" gains one explicit, user-initiated exception: a report the user reviews and sends.
  - The CSP allows the Turnstile origin, but only in `script-src` and `frame-src`.
- **Amends `agent/mcp-server`** with the report-preparation tool.
- **Adds a Cloudflare Worker, a D1 database, and a daily cron.** These are the product's only server-side components, and they compute nothing.
- **Adds `docs/runbooks/problem-reports.md`, `.github/ISSUE_TEMPLATE/wrong-answer.yml`, and `known-issues.json`.**
