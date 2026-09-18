## Context

This change exists because the second independent review (September 18, 2026) listed the questions an implementer would otherwise have to ask mid-build. It answers each at the seam where two changes meet. Pattern references are in `docs/research/06` (roughlogic.com's hash-state versioning, report Worker, head parity, and mobile viewports) and `docs/research/05` (Cloudflare Workers static assets, Turnstile, and D1).

## Goals / Non-Goals

**Goals:**
- One authoritative answer per seam, with shared test vectors where two runtimes must agree byte for byte.

**Non-Goals:**
- Re-specifying behavior already owned by another capability.

## Decisions

### B1. Cloudflare topology
A **single Worker project** serves the static site as Workers static assets and handles the report API:

- **Routing:** `assets.run_worker_first: ["/api/reports", "/api/reports/*"]`. Every other request is served by the static-asset layer without invoking code.
- **Custom domain:** `geoprims.com` is attached to this Worker.
- **Observability off:** `observability.enabled: false` and invocation logs disabled.
- **Bindings:** one D1 database (`REPORTS_DB`) and the secrets `TURNSTILE_SECRET_KEY` and `REPORT_HASH_SECRET`.
- **Cron:** a daily trigger (`17 8 * * *`) for cleanup.
- **Large assets:** served from an R2 bucket on the custom domain `assets.geoprims.com`, with CORS allowing only `https://geoprims.com` and exposing `Content-Range`.
- **Zone settings:** Bot Fight Mode and Super Bot Fight Mode disabled, one WAF rate-limit rule on `/api/reports*`, HSTS preload.

Fallback, if the `run_worker_first` array patterns do not match as documented: a separate Worker bound to the route `geoprims.com/api/reports*` (roughlogic's pattern). Worker routes take precedence over the static-asset custom domain for that path. Task 2.1 verifies which applies.

### B2. The CSP, fully enumerated
```
default-src 'self';
script-src 'self' 'wasm-unsafe-eval' https://challenges.cloudflare.com 'sha256-<island-bootstrap>';
style-src 'self';
img-src 'self' blob: data: https://assets.geoprims.com;
connect-src 'self' https://assets.geoprims.com;
worker-src 'self' blob:;
frame-src https://challenges.cloudflare.com;
font-src 'self';
object-src 'none';
base-uri 'none';
frame-ancestors 'none';
form-action 'none';
manifest-src 'self'
```

Astro is configured with `build.inlineStylesheets: 'never'`. Any inline bootstrap script Astro emits is allowed by its build-computed SHA-256 hash, written to both `_headers` and a meta tag by the build and checked by the header smoke test. No `unsafe-inline` is used for scripts or styles.

### B3. Why a formal fragment grammar
roughlogic versions its hash state (`v=` key) and learned that renamed fields need migration. geoprims compresses state, so an explicit grammar and a byte-exact shared vector file are the only way to guarantee that MCP-generated report links open identically on the site.

### B4. Codes as data
The codes registry is `data/codes.json`, the source for message templates in the web UI, MCP results, and docs (`/methodology#codes`). Severity drives ordering and collapsibility in the answer card.

## Risks / Trade-offs

- **[Contracts drift from implementations]** → Each contract has a gate (route map, anatomy, meta-schema, codes completeness, limits parity, fragment vectors, CSP smoke test).
- **[Workers static-assets routing semantics change]** → The fallback topology is documented, and the header and route smoke tests catch regressions.

## Migration Plan

Not applicable.

## Open Questions

None.
