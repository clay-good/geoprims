## Purpose

Specifies the exact HTTP contract and limits of the problem-report endpoint, so the client, Worker, D1 schema, and MCP report preparation agree byte for byte.

## ADDED Requirements

### Requirement: Config endpoint
`GET /api/reports/config` SHALL return `200` with `{"enabled": true, "sitekey": "<turnstile sitekey>", "limits": {...}, "apiVersion": 1}`, with `Cache-Control: max-age=300`, when reporting is available. It SHALL return `503` with `{"enabled": false}` when it is disabled or misconfigured.

#### Scenario: Disabled
- **WHEN** the Worker lacks its secrets
- **THEN** the config endpoint returns 503 `{"enabled": false}` and the dialog shows the paused state

### Requirement: Submit endpoint
`POST /api/reports` SHALL accept `application/json` bodies with exactly these keys:
- `apiVersion` (1)
- `toolId`, `toolVersion`, `coreVersion`, `buildHash`
- `assetVersions` (object)
- `kind`
- `pagePath`
- `inputs`: `[{field, label, value, unit}]`
- `outputs`: `[{field, label, value, unit}]`
- `warnings`: `[code]`
- `display`: `{theme, unitProfile, viewportClass}`
- `note` (string or null)
- `token`

It SHALL respond `202 {"ok": true}` for every syntactically acceptable request, whether stored or not. It SHALL respond `400 {"ok": false}` only for requests that are not JSON or exceed the size cap, and `405` for other methods.

#### Scenario: Wrong method
- **WHEN** a GET is sent to `/api/reports`
- **THEN** the response is 405

### Requirement: Authoritative limits table
These limits SHALL be defined once, in a shared constants file imported by the client, the Worker, the D1 migration generator, and the MCP report tool:

| Limit | Value |
|---|---|
| Body | 32,768 bytes |
| Input rows | ≤ 32 |
| Output rows | ≤ 32 |
| Field name | ≤ 40 characters |
| Label | ≤ 80 characters |
| Value | ≤ 160 characters |
| Unit | ≤ 16 characters |
| Note | ≤ 280 characters |
| `pagePath` | ≤ 2,048 characters |
| Warnings | ≤ 30 codes |
| `inputs_json` / `outputs_json` columns | ≤ 12,000 characters each |
| `warnings_json` | ≤ 2,000 characters |
| `display_json` | ≤ 300 characters |
| `asset_versions_json` | ≤ 2,000 characters |

#### Scenario: Worst-case payload fits
- **WHEN** a payload uses every row and character limit at maximum
- **THEN** its serialized size is at most 32,768 bytes (worst case ≈ 28.2 KB), and every D1 column fits its CHECK constraint (worst case ≈ 11,100 characters per rows column)

### Requirement: Client submission behavior
The client SHALL:
1. obtain the bot-check token at submit time, and refresh it if it is older than 240 s
2. post once
3. show "Report sent. Thanks. Updates appear on the known-issues page." on `202`

On a network failure it SHALL show "Couldn't send. Copy the report instead?" with the copy action. It SHALL never retry automatically more than once.

#### Scenario: Slow note
- **WHEN** a user spends 6 minutes writing a note
- **THEN** the client fetches a fresh token before posting, and the report is stored
