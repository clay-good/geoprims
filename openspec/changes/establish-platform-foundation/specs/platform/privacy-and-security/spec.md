## Purpose

Guarantees that geoprims needs no account, collects no personal data, never sends user inputs off the device, and ships code whose integrity users and agents can verify.

## ADDED Requirements

### Requirement: No accounts, cookies, or identifiers
The product SHALL NOT offer or require accounts, SHALL NOT set cookies, and SHALL NOT create persistent identifiers of any kind (including fingerprinting, local-storage IDs, or ETag tracking). Local storage SHALL hold only user preferences, recent tools, and user-saved inputs, all of which SHALL be viewable and erasable from one settings screen.

#### Scenario: Fresh visit sets nothing
- **WHEN** a new visitor loads any page
- **THEN** no cookie is set and no storage key is written until the user changes a preference or runs a tool

#### Scenario: Erase all local data
- **WHEN** the user chooses "Erase all local data" and confirms
- **THEN** local storage, IndexedDB, cache storage (except the app shell), and service-worker-held data created by the app are removed, and a reload shows default settings

### Requirement: User inputs never leave the device
User-entered values, uploaded files, and results SHALL NOT be transmitted over the network. Network requests SHALL be limited to same-origin static assets (code, docs, data assets, tiles) whose URLs are independent of user input except for spatial tile addresses at zoom levels at most 12 or dataset tile indices at a granularity no finer than 1° × 1°.

#### Scenario: Automated egress test
- **WHEN** the end-to-end privacy test runs every tool with sentinel input values
- **THEN** no request URL, header, or body observed by the test proxy contains any sentinel value

#### Scenario: Coarse tile requests only
- **WHEN** a user requests a geoid height at a precise coordinate
- **THEN** the only resulting request is for the containing dataset tile, and the tile address reveals location no finer than the declared granularity

### Requirement: Shareable state stays client-side
Permalinks SHALL encode tool inputs in the URL fragment (after `#`), which browsers do not send to servers. The query string SHALL NOT carry user inputs.

#### Scenario: Permalink privacy
- **WHEN** a user copies a permalink to a geodesic calculation
- **THEN** the coordinates appear only after `#` in the URL

### Requirement: No third-party requests
Pages SHALL make no requests to third-party origins: no analytics, fonts, CDNs, ads, error reporting, or embeds. All assets SHALL be served from the geoprims origin (or a geoprims-controlled asset subdomain listed in the CSP).

#### Scenario: Third-party audit
- **WHEN** CI loads every route and records network requests
- **THEN** every request targets an allow-listed geoprims origin, else the build fails

### Requirement: Aggregate, cookieless usage metrics only (optional)
If usage metrics are collected, they SHALL be limited to server-side aggregate counts of static asset requests from the hosting provider's logs, with no client-side script, no IP retention beyond the provider's default, and no per-user data. The privacy page SHALL state exactly what is and is not collected.

#### Scenario: No analytics script
- **WHEN** any page's source is inspected
- **THEN** it contains no analytics or telemetry script

### Requirement: Strict Content Security Policy
Every page SHALL be served with a Content Security Policy that at minimum sets `default-src 'self'`, `script-src 'self' 'wasm-unsafe-eval'`, `connect-src 'self'` (plus any geoprims asset origin), `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'none'`, and no `unsafe-inline` for scripts. Pages SHALL also send `Referrer-Policy: no-referrer`, `X-Content-Type-Options: nosniff`, and a `Permissions-Policy` that disables camera, microphone, geolocation (except on explicit user action for "use my location"), payment, and USB.

#### Scenario: Header check
- **WHEN** the deployment smoke test fetches any route
- **THEN** the CSP and security headers match the required policy

### Requirement: Geolocation only on explicit request
The app SHALL request the browser Geolocation API only when the user activates a "use my location" control, SHALL use the position only as a tool input, and SHALL NOT persist it unless the user saves the inputs.

#### Scenario: No automatic location prompt
- **WHEN** any page loads
- **THEN** no geolocation permission prompt appears

### Requirement: Local file handling
Imported files (GeoJSON, KML, GPX, CSV, GeoTIFF, WKT) SHALL be parsed locally with size limits declared per format, SHALL never be uploaded, and parsers SHALL reject external entity references (XML), remote resource references, and scripts embedded in file content.

#### Scenario: KML with external reference
- **WHEN** a user imports a KML containing a `NetworkLink` or an external `href`
- **THEN** the link is not fetched and the import report lists it as ignored

### Requirement: Supply-chain integrity
Releases SHALL be built reproducibly in CI from a tagged commit, SHALL publish SHA-256 digests of every shipped artifact, SHALL publish npm packages with provenance attestations, SHALL pin all build dependencies by lockfile and hash, and SHALL run dependency license and vulnerability audits that block release on critical findings.

#### Scenario: Reproducible build
- **WHEN** two independent CI runs build the same tag
- **THEN** the Wasm modules and JS bundles have identical digests

### Requirement: Vulnerability disclosure
The repository SHALL publish a `SECURITY.md` with a private reporting channel and a response-time commitment, and the site SHALL serve `/.well-known/security.txt`.

#### Scenario: security.txt present
- **WHEN** `/.well-known/security.txt` is requested
- **THEN** it returns a valid RFC 9116 file with a contact and expiry
