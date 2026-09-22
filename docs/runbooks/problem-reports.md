# Runbook: problem reports

How to triage what users report, move a report through its statuses, and launch reporting. The rules come from `openspec/changes/add-problem-reporting` (triage-and-corrections); the Worker itself is described in [worker/README.md](../../worker/README.md).

## Targets

| Report | Triage within | Then |
|---|---|---|
| `wrong-result` | 24 hours | If confirmed in an aviation, drone, navigation, or height/datum tool, a `data/known-issues.json` entry within 24 hours of confirming, before the fix ships |
| Anything else | 72 hours | |

A report is `confirmed` only after its claimed value is checked against a primary source (a standard, a reference implementation, or a published worked example). If the source contradicts the report, close it `not_a_bug`, and if the confusion was reasonable, improve the tool's text.

## Statuses

`open → triaged → confirmed → fixed`, or closed from `open` or `triaged` as `wont_fix`, `duplicate`, or `not_a_bug` (and from `confirmed` as `wont_fix`).

| Move to | Must record |
|---|---|
| `confirmed` | `--source`: the primary source that settles it |
| `fixed` | `--fixed-in`: the tool version with the fix, like `1.1.0` |
| `wont_fix`, `duplicate`, `not_a_bug` | `--note`: the resolution |

Every move goes through the wrapper, which refuses any other move before writing SQL. Leave out `--remote` to print the statement instead of running it.

```bash
node worker/scripts/triage.mjs list --remote
```

```bash
node worker/scripts/triage.mjs show REPORT_ID --remote
```

```bash
node worker/scripts/triage.mjs set REPORT_ID --from triaged --to confirmed --source "ICAO Doc 7488/3, Table 1" --remote
```

Before confirming, reproduce the report. The script restores the inputs from its permalink and prints what the report said, the result on the reported build, and the result on the current build, marking each field that differs (`≠`). When the reported build is an older release, pass that release's `dist/wasm` with `--reported-wasm`.

```bash
node worker/scripts/reproduce.mjs REPORT_ID --remote
```

`list` shows open reports with wrong results first, oldest first, and `overdue = 1` on wrong results older than 24 hours and anything older than 72. A fix for a confirmed defect adds a golden vector with the reported inputs and its source, bumps the tool version, and, if results move beyond tolerance, adds a `result-change` entry to `data/changelog.json` with the old and new values.

## Queries

Run these with `npx wrangler d1 execute geoprims-reports --remote --command "…"`. Each one is tested against a seeded database (`worker/test/runbook.test.mjs`).

Open reports by tool, to spot a cluster:

```sql
SELECT tool_id, COUNT(*) AS open_reports FROM problem_reports WHERE status IN ('open', 'triaged') GROUP BY tool_id ORDER BY open_reports DESC;
```

Confirmed defects in tools that need a known-issues entry within 24 hours:

```sql
SELECT id, tool_id, primary_source FROM problem_reports WHERE status = 'confirmed' AND (tool_id LIKE 'aviation.%' OR tool_id LIKE 'drone.%' OR tool_id LIKE 'navigation.%' OR tool_id LIKE 'geodesy.%') ORDER BY created_at;
```

Notes that contain a link, for review before anything is quoted:

```sql
SELECT id, tool_id, note FROM problem_reports WHERE note_has_url = 1 AND status = 'open';
```

Median-age check for the monthly `/quality` page (fixed reports this month):

```sql
SELECT id, tool_id, created_at, resolved_at, fixed_in_version FROM problem_reports WHERE status = 'fixed' AND resolved_at >= date('now', 'start of month');
```

## Retention

The Worker's daily cron deletes resolved reports 180 days after they close and the rate-limit counters after 14 days. Open, triaged, and confirmed reports are kept until they close.

## Launch checklist

Run on production with `REPORTS_ENABLED` still `"false"` until the last step, and record the date and result of each line here.

| Check | How | Result |
|---|---|---|
| Paused state | Open the dialog: it offers the copy-report and "Wrong answer" form paths | |
| Round trip | Enable, send a report from a tool page, and find it with `list` | |
| Duplicate | Send the same report again: 202, and no second row | |
| Quota | Send past the daily cap: still 202, no new rows | |
| Offline | Turn the network off and open the dialog: the copy-report path works | |
| Kill switch | Set `REPORTS_ENABLED` to `"false"`: the config endpoint returns 503 and the dialog pauses | |
