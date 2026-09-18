## Why

A second independent review of the specs asked the questions an implementer would stop and ask. Each one sits at a seam between changes:
- What URL does each kind of page get?
- Which banners show at 320 px, and in what order?
- What is the exact report API?
- What does the sentence-template language look like?
- Which example is "the" example?
- Which device and network profile do all the budgets use?
- Is `1,250` 1250 or 1.25?
- What is the full list of warning codes, and how severe is each?

Answering them once, in one place, is what makes the build seamless. It is "measure twice, cut once" applied to the interfaces rather than the math.

Depends on: all other changes. It is authoritative for the seams it defines. Where another spec names one of these contracts, it defers to this change.

## What Changes

- **Routes and URLs:** one route map for every page class, sitemap membership per class, and a formal permalink fragment grammar with shared web/MCP test vectors.
- **Page chrome:** the single canonical tool-page anatomy (merging the documentation sections, SEO content minimums, answer card, proof panel, and related tools), the 320 px header contents, and banner priority with a two-visible limit so the answer stays above the fold.
- **Manifest extensions:**
  - every `x-` field in the manifest meta-schema
  - the sentence-template language
  - the glossary schema
  - per-field steps
  - the clock-default flag
  - near-limit margins
  - the primary-worked-example rule
- **Codes registry:** the seed list of error and warning codes with severity, meaning, and when each is an error or a warning.
- **Reference profiles:** the one device, network, viewport, and engine set used by every performance, mobile, palette, and canvas budget.
- **Report API:** exact endpoints, request and response schemas, the authoritative limits table, and token handling. The Cloudflare topology is in design.md.

## Capabilities

### New Capabilities

- `contracts/routes-and-urls`: The route map, sitemap membership, and permalink grammar.
- `contracts/page-chrome`: The canonical tool-page anatomy, header, and notice stacking.
- `contracts/manifest-extensions`: All manifest extension fields, sentence templates, glossary, examples, and steps.
- `contracts/codes-registry`: Error and warning codes with severities.
- `contracts/reference-profiles`: The single measurement profile for all budgets and gates.
- `contracts/report-api`: The problem-report HTTP contract and limits.

### Modified Capabilities

None (the other changes are unarchived and already reference these contracts).

## Non-goals

- Visual design specifics (colors, spacing values). Those live in design tokens, per `web/visual-theme`.

## Impact

- The foundation meta-schema task (3.1) validates every extension listed here.
- The web app, MCP server, and report Worker implement the same contracts and share test vectors.
