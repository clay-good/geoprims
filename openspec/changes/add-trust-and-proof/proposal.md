## Why

A pilot deciding whether to take off, or a surveyor signing a plat, will not trust a number from a website they have never heard of unless the site shows its work. geoprims has to prove every answer on the page:

- where the formula comes from (standard, edition, section)
- which worked example it reproduces
- when it was last verified
- what it simplifies

It also has to keep that proof current as standards and models change: WMM expires in 2030, NGS datums are mid-modernization, and FAA and EASA rules move.

roughlogic.com already runs this discipline at scale: citations on every tile, an edition-freshness ledger that fails CI when a newer edition ships, worked-example fixtures with source and section, and honest reviewer-signoff disclosure (`docs/research/06` §4). Trust patterns from the wider field are in `docs/research/05` §5.

Depends on: `establish-platform-foundation` (verification, data assets), `build-web-experience` (tool pages).

## What Changes

- **A citation record for every tool:** formula source, publisher, title, edition or version, section or page, a free-access pointer, the governing authority, and every numeric assumption with its own source. Copyrighted tables are cited by locator and taken as user inputs, never reproduced.
- **A standards and models freshness ledger:** current edition, release cycle, next expected edition, last verified date, and model validity windows. CI fails when a tool names a superseded edition or an expired model, and warns before expiry. A monthly probe checks free-access links.
- **A layered correctness program:**
  - written derivations
  - worked-example fixtures traced to a published source
  - dimension annotations
  - bounds fuzzing and invariants
  - example parity between web and MCP
  - practitioner review sign-off per domain, disclosed honestly when it has not happened
- **Proof on every tool page:** a "How we got this" panel with the formula using your numbers, the worked example, citations, assumptions, limitations, last-verified date, and links to test vectors. A "Copy answer with reference" action produces text suitable for field notes and audit files.
- **Site-wide trust pages:** `/sources` (every cited standard and the tools that use it), `/methodology`, and `/verification/<version>`, which extends the foundation report.

## Capabilities

### New Capabilities

- `trust/citations`: The citation record, its rules, and how it is rendered and exported.
- `trust/freshness`: Edition, model, and regulatory freshness tracking with CI enforcement.
- `trust/correctness-program`: The layered checks that every tool passes before it can be called stable.
- `trust/proof-display`: How proof appears to users on tool pages and site-wide trust pages.

### Modified Capabilities

None. This extends `platform/verification` and `web/tool-docs`; where they overlap, the stricter rule applies.

## Non-goals

- Professional certification or stamped outputs.
- Reproducing copyrighted standards text or tables (ASTM, AASHTO, ASPRS figures beyond short factual values, ICAO tables beyond constants). The site cites them and takes table values as inputs.
- Paid external audits in v1. Practitioner review is volunteer or commissioned, and is disclosed as such.

## Impact

- New repository data:
  - `data/citations/*.json`, one file per domain
  - `data/sources-ledger.json`
  - `test/worked-examples/*.jsonl`, which extends the golden-vector format
  - `docs/derivations/<tool>.md`
  - `docs/review-signoffs.md`
- New CI gates: citation coverage, edition freshness, model expiry, free-access probe (monthly), dimension lint, bounds fuzz, example parity, and claims honesty.
