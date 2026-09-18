## Purpose

Lets any visitor report a wrong answer, broken tool, or confusing result in one click, sending exactly the context maintainers need to reproduce it and nothing that identifies the reporter.

## ADDED Requirements

### Requirement: One report button on every tool
Every tool page SHALL show exactly one "Report a problem" button in the tool header, beside the tool title, visible without scrolling on a 320 px wide screen, with a touch target of at least 48 × 48 CSS px. Clicking it SHALL lazy-load the report module; no reporting code, bot-check script, or network request related to reporting SHALL load before the click.

#### Scenario: Button present on every tool
- **WHEN** the CI wiring gate renders every tool route
- **THEN** each has exactly one report button, and the build fails listing any route with zero or more than one

#### Scenario: Nothing loads before the click
- **WHEN** a user opens a tool and does not click the report button
- **THEN** the page makes no request to the report dialog module, the bot-check origin, or `/api/reports` (the service worker's precache of the app shell excludes the dialog module; a tiny "copy report as text" fallback lives in the shell so offline copy still works)

### Requirement: The dialog shows exactly what will be sent
The report dialog SHALL be a native modal dialog that states in plain language what will be attached. It SHALL show the full payload in a readable preview before sending. Preview rows SHALL let the user exclude inputs and outputs. It SHALL include an optional "What did you expect instead?" note of at most 280 characters with a live remaining-count, and a line reading "Please don't include names, addresses, or other personal information."

#### Scenario: Payload preview
- **WHEN** a user opens the dialog on the density-altitude tool
- **THEN** the dialog lists the tool name, version, the inputs and outputs with units, and the permalink, exactly as they will be sent

#### Scenario: Exclude inputs
- **WHEN** a user unticks "Include my inputs and results"
- **THEN** the preview removes them, `pagePath` is sent without its fragment, `inputs`, `outputs`, and `warnings` are empty arrays, and the payload keeps only the tool id, versions, kind, display class, note, and bot-check token

### Requirement: Report payload
A report SHALL contain only these fields:
- `toolId`, `toolVersion`, `coreVersion`, `buildHash`, and `assetVersions`
- `pagePath`: the route plus the permalink fragment, restricted to known input keys
- `inputs` and `outputs`: label, value, and unit, in machine form
- `warnings`: codes
- `display`: `theme`, `unitProfile`, and `viewportClass` (`phone`, `tablet`, or `desktop`)
- `note`
- `kind`: `wrong-result`, `broken`, `confusing`, or `other`
- the bot-check token

It SHALL NOT contain a user-agent string, screen dimensions, language, time zone, referrer, or any identifier.

#### Scenario: No identifying fields
- **WHEN** the payload schema is validated in CI
- **THEN** it contains only the fields listed, and the build fails if any other field is added

#### Scenario: Private inputs never sent
- **WHEN** a tool contains an input marked `x-private` (for example a free-text field for a person's name on a calculation sheet)
- **THEN** that input is omitted from the payload and preview, and outputs derived from it are replaced by a placeholder

### Requirement: Endpoint validation
The report endpoint SHALL accept only `POST` with content type `application/json` and no content encoding. It SHALL accept only an `Origin` in the allowlist, and SHALL cancel reading any body over 32 KB. It SHALL validate the body strictly:
- the exact key set is enforced
- `toolId` must exist in the catalog bundled with the Worker, and the tool name is re-derived on the server
- `pagePath` must belong to that tool
- at most 32 input rows and 32 output rows; field names ≤ 40 characters; labels ≤ 80 characters; values ≤ 160 characters; note ≤ 280 characters; `pagePath` ≤ 2,048 characters (the single limits table in `contracts/report-api` is authoritative)
- control characters and bidirectional override characters are rejected
- URLs in the note are flagged for review

#### Scenario: Unknown tool rejected silently
- **WHEN** a POST names a tool id that does not exist
- **THEN** the report is not stored, and the response is identical to a successful submission

#### Scenario: Oversized body
- **WHEN** a request body exceeds 32 KB
- **THEN** the Worker stops reading and does not store anything

### Requirement: Abuse controls without tracking
The endpoint SHALL layer these controls:
1. An edge rate-limit rule of at most 10 requests per 10 s per client, enforced by the host. geoprims stores nothing from it; Cloudflare records blocked requests in its security event log under its own retention policy, which the privacy page states.
2. Bot-check verification on the server, with action name `problem-report` and hostname checks. The widget loads only inside the dialog and uses no pre-clearance cookie.
3. Attempt caps per reporter per day (10) and globally per day (500).
4. Accepted-report caps per reporter per day (5) and globally per day (250), which configuration cannot raise above hard-coded ceilings.
5. Dedupe on a daily hash of the canonical payload.

The per-reporter key SHALL be a keyed hash, rotated daily with a secret, of the client address. It SHALL be stored only in the counter table and never in the reports table. Counters SHALL be deleted after 14 days. Every outcome (accepted, duplicate, over quota, invalid) SHALL return the same `202` response.

#### Scenario: Uniform response
- **WHEN** a sixth report from the same client arrives on the same day
- **THEN** it is not stored and the response is the same `202` as an accepted report

#### Scenario: No address stored
- **WHEN** the D1 reports table is inspected
- **THEN** it contains no IP address, address hash, or reporter key column

### Requirement: Storage schema and integrity
Reports SHALL be stored in a D1 table with at least these columns:
- `id`, `created_at`, `tool_id`, `tool_version`, `core_version`, `build_hash`, `kind`
- `page_path`, `note`, `inputs_json`, `outputs_json`, `warnings_json`
- `dedupe_key` (unique)
- `status` (`open`, `triaged`, `confirmed`, `fixed`, `wont_fix`, `duplicate`, or `not_a_bug`)
- `resolution_note`, `fixed_in_version`, `resolved_at`

CHECK constraints SHALL enforce the same size limits as the client and Worker. Counter increments, cleanup, and the conditional insert SHALL run as one atomic batch. Schema changes SHALL be applied as numbered migrations in CI before deployment.

#### Scenario: Limits agree across layers
- **WHEN** the CI feedback-loop gate compares the note, row, and body limits in the client, Worker, and D1 CHECK constraints
- **THEN** all three agree, or the build fails naming the mismatch

### Requirement: Retention
Reports SHALL be kept while open and deleted 180 days after resolution. A daily scheduled job SHALL perform deletions and counter cleanup. The privacy page SHALL state the retention periods. The durable record of a fix SHALL be the regression vector, changelog entry, and known-issues entry, not the D1 row.

#### Scenario: Scheduled cleanup
- **WHEN** the daily job runs
- **THEN** resolved reports older than 180 days and counters older than 14 days are deleted

### Requirement: Kill switch and graceful failure
If the Worker's secrets or bot-check configuration are missing, or reporting is disabled by configuration, the config endpoint SHALL return 503. The dialog SHALL then say "Reporting is paused right now" and link to the GitHub "Wrong answer" issue form. When offline, the dialog SHALL say that reporting needs a connection and offer to copy the report text to the clipboard. The service worker SHALL never cache `/api/*`.

#### Scenario: Offline report
- **WHEN** a user in the field opens the dialog without connectivity
- **THEN** the dialog offers "Copy report" so the text can be sent later, and nothing is queued silently

### Requirement: Isolated server component
The report Worker SHALL be the only server-side code in the product. It SHALL be bound only to `/api/reports*`, SHALL perform no calculation, and SHALL set response headers `Content-Security-Policy: default-src 'none'; sandbox`, `Cache-Control: no-store`, and HSTS. It SHALL have request logging disabled, so request metadata is not retained by the host. It SHALL import the tool catalog from the same build as the site.

#### Scenario: Logging disabled
- **WHEN** the Worker configuration is linted in CI
- **THEN** invocation logs and observability sampling are disabled, or the build fails

### Requirement: No host-injected cookies or challenges
The Cloudflare zone SHALL keep Bot Fight Mode, Super Bot Fight Mode, and any challenge or cookie-setting bot features disabled, so no `__cf_bm` or `cf_clearance` cookie is ever set on site pages. The deployment smoke test SHALL fail if any response sets a cookie.

#### Scenario: Cookie check
- **WHEN** the smoke test loads the home page, a tool page, and submits a test report
- **THEN** no response contains a `Set-Cookie` header
