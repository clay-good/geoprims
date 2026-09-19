# worker

The one piece of server code in geoprims: a Cloudflare Worker bound to `/api/reports*` that receives problem reports into a D1 database (add-problem-reporting). It performs no calculation, keeps request logging off, and stores no address.

| Path | What |
|---|---|
| `src/index.mjs` | Routes: `GET /api/reports/config`, `POST /api/reports`; the daily cleanup |
| `src/report.mjs` | Strict validation, the daily dedupe key, and the keyed reporter hash |
| `src/catalog-tools.json` | Tool ids and versions from the site's catalog build (`node worker/scripts/prepare.mjs`) |
| `migrations/` | D1 migrations; `0001` is generated from `data/report-limits.json` by `tools/codegen/d1-migration.mjs` |
| `wrangler.jsonc` | Route, D1 binding, cron, logging off, and the kill switch (`REPORTS_ENABLED`) |
| `test/` | Tests on Node's built-in SQLite through a D1-shaped shim, so the real migration's CHECKs and batches run |

Every acceptable request gets the same `202 {"ok": true}`, whether stored, duplicate, over quota, or invalid; only non-JSON or oversized bodies get `400`, and other methods `405`.

Deploy (maintainer, with a Cloudflare account):

```bash
npx wrangler d1 create geoprims-reports
```

```bash
npx wrangler d1 migrations apply geoprims-reports --remote
```

```bash
npx wrangler secret put TURNSTILE_SECRET
```

Then set `database_id` in `wrangler.jsonc`, deploy with `npx wrangler deploy`, add the WAF rate rule (10 requests per 10 s), run the launch checklist, and only then set `REPORTS_ENABLED` to `"true"`.
