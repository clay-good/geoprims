## Context

The pattern source is roughlogic.com (`docs/research/06-roughlogic-patterns.md` §1). It already runs this exact loop: a lazy-loaded dialog, a separate Worker on `/api/reports*`, D1 with CHECK limits, a daily keyed reporter hash used only in counters, uniform 202 responses, a cron cleanup, and a runbook of `wrangler d1 execute` queries.

Platform facts come from `docs/research/05` §1:

| Service | Free-plan limits |
|---|---|
| D1 | 100,000 row writes and 5M reads per day; 500 MB per database |
| Turnstile | Unlimited challenges |
| WAF | 1 rate-limit rule |

Turnstile processes client IP, TLS fingerprint, and user agent, which is why it loads only in the dialog. Cloudflare Workers Logs are on by default and record request metadata, so they must be disabled.

## Goals / Non-Goals

**Goals:**
- **Fast repair.** A maintainer can reproduce any report in under a minute.
- **Honest privacy.** The only personal-data-adjacent value (the client address) is used transiently for a keyed hash, kept only in counters, and deleted after 14 days.
- **Robustness.** The calculators never depend on the reporting system.

**Non-Goals:**
- A triage UI. Scripted SQL and a reproduction script are enough at this scale.
- Automated fix suggestions.

## Decisions

### R1. Separate report Worker on the same zone
The Worker is bound to `geoprims.com/api/reports*`. The static site is served as Workers static assets (foundation D7). Only `/api/reports*` runs code; static assets never invoke the Worker. This matches roughlogic's separation and keeps the "only server code" claim auditable.

### R2. Keyed daily hash, counters only
`reporterKey = HMAC-SHA256(secret, utcDate + ":" + clientAddress)`. It exists only in `report_limits`, and counters are deleted after 14 days.

Alternatives considered:
- **Plain hashing:** rejected, because an IPv4 hash can be brute-forced.
- **No per-reporter limit:** rejected, because a single abuser could exhaust the global cap.

### R3. Improvements over roughlogic
- **Build hash and versions in the payload.** Roughlogic lacked these, so a report could not pinpoint the build it came from.
- **A 280-character note** (roughlogic uses 160), to fit "expected X per <source>".
- **A `kind` field** for prioritization.
- **A public known-issues page and a monthly quality summary.** Roughlogic has neither.
- **An offline "Copy report" fallback.**
- **An MCP report-preparation tool.**

### R4. Known issues from a curated file, not live D1
Reading D1 at request time would need a public read endpoint and would expose raw user text. Instead, maintainers write `known-issues.json` entries, reviewed in PRs, and the static build renders them.

### R5. Status model
`open → triaged → confirmed → fixed`, plus the terminal states `wont_fix`, `duplicate`, and `not_a_bug`, each requiring a resolution note. `fixed` requires `fixed_in_version`. Transitions are done via scripted SQL wrappers that enforce these rules.

### R6. D1 schema
```sql
-- 0001_problem_reports.sql
CREATE TABLE problem_reports (
  id TEXT PRIMARY KEY NOT NULL,
  created_at TEXT NOT NULL,
  tool_id TEXT NOT NULL,
  tool_version TEXT NOT NULL,
  core_version TEXT NOT NULL,
  build_hash TEXT NOT NULL,
  asset_versions_json TEXT NOT NULL CHECK (length(asset_versions_json) <= 2000),
  kind TEXT NOT NULL CHECK (kind IN ('wrong-result','broken','confusing','other')),
  page_path TEXT NOT NULL CHECK (length(page_path) <= 8192),
  note TEXT CHECK (note IS NULL OR length(note) <= 280),
  note_has_url INTEGER NOT NULL DEFAULT 0 CHECK (note_has_url IN (0,1)),
  inputs_json TEXT NOT NULL CHECK (length(inputs_json) <= 60000),
  outputs_json TEXT NOT NULL CHECK (length(outputs_json) <= 60000),
  warnings_json TEXT NOT NULL CHECK (length(warnings_json) <= 4000),
  display_json TEXT NOT NULL CHECK (length(display_json) <= 500),
  dedupe_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN
    ('open','triaged','confirmed','fixed','wont_fix','duplicate','not_a_bug')),
  primary_source TEXT CHECK (primary_source IS NULL OR length(primary_source) <= 1000),
  resolution_note TEXT CHECK (resolution_note IS NULL OR length(resolution_note) <= 1000),
  fixed_in_version TEXT,
  resolved_at TEXT
);
CREATE INDEX pr_status_kind_created ON problem_reports (status, kind, created_at);
CREATE INDEX pr_tool_status ON problem_reports (tool_id, status);
CREATE TABLE report_limits (
  bucket TEXT NOT NULL, scope TEXT NOT NULL CHECK (scope IN ('global','reporter')),
  subject TEXT NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('attempt','accepted')),
  count INTEGER NOT NULL CHECK (count >= 0),
  PRIMARY KEY (bucket, scope, subject, kind)) WITHOUT ROWID;
```

### R7. Bot check
Turnstile in managed ("interaction-only") mode, loaded only when the dialog opens, with no pre-clearance cookie. The privacy page names Turnstile, links Cloudflare's Turnstile privacy addendum, and explains when it loads.

Alternative considered: a proof-of-work challenge with no third party. Deferred: it is weaker against targeted spam. It stays as the fallback if Turnstile terms change.

### R8. Security headers and CSP
The site CSP adds `https://challenges.cloudflare.com` to `script-src` and `frame-src` only. `connect-src 'self'` already covers `/api/reports`.

## Risks / Trade-offs

- **[Spam floods the review queue]** → Layered caps, a URL flag in notes, and dedupe. The global daily cap bounds the worst case at 250 rows per day.
- **[The report feature erodes the privacy story]** → Explicit user action, full payload preview, excludable inputs, no identifiers, published retention, and Turnstile only in the dialog.
- **[D1 rows expire before a fix]** → Open reports never expire. Only resolved ones are deleted after 180 days, and the durable record lives in the repository.
- **[Reporters expect replies]** → The dialog's success message says "Thanks. Check /known-issues for updates", with no promise of a reply.

## Migration Plan

Deploy order:
1. Create D1 and apply migrations.
2. Set secrets.
3. Deploy the Worker with reporting disabled.
4. Add the WAF rule.
5. Enable reporting.
6. Run the launch verification checklist: a real report round trip, a duplicate, a quota test, the offline fallback, and the kill switch.

Rollback: disable by configuration. The calculators are unaffected.

## Open Questions

None that affect the specs.
