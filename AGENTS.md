# AGENTS.md: how to build geoprims

Rules for anyone, human or AI, implementing this repository. The specs in `openspec/changes/` are the source of truth. Where two specs meet (URLs, page layout, manifest fields, codes, budgets, report API), `openspec/changes/define-build-contracts/` is authoritative. This file is the short version of how to work.

## The product in one paragraph

A free, static website and a local MCP server for geospatial, aviation, drone, surveying, spatial-indexing, and time math. Both run the same Rust → WebAssembly calculators and return byte-identical results. There are no ads, accounts, tracking, or server-side calculation. The only server code is the opt-in problem-report Worker.

## Three doors, one core

Every tool is reachable through three doors, and all three must work before a tool is done:

1. **Web page:** answer first, "How we got this" panel, mobile-gated.
2. **MCP server:** `geoprims_search` finds it, `geoprims_describe` explains it, `geoprims_run` runs its example.
3. **Report a problem:** the button is wired and the payload is correct.

Never implement math twice. Never bypass the core from the web app or the MCP server.

## Order of work

Follow `openspec/changes/plan-launch-and-value-proof/design.md`:
- **Phase 0:** platform changes.
- **Phase 1:** hero tools.
- **Phase 2:** complete the domains by demand.

Within a change, work its `tasks.md` top to bottom and tick boxes only when the stated verification passes.

## Adding or changing a tool

1. Start from the spec requirement and scenarios. If behavior is not specified, propose a spec change first.
2. Write the manifest: inputs, outputs, units, ranges, errors, sentence template, visualization, prefill slots, related tools.
3. Write the citation record (source, edition, locator, free-access link, assumptions with sources). Check that the source is in the sources ledger at its current edition.
4. Write the derivation note in `docs/derivations/<tool>.md`.
5. Add worked examples, at least one from an independent published source, with tolerances.
6. Implement in the core and pass all correctness layers: dimensions, bounds fuzz, invariants, differential, example parity, both-surfaces.
7. Check the page: glance test, mobile sweep, content minimums, OG image, related links.
8. If a confirmed defect is fixed, add a regression vector, bump the tool version, and label any result change in the changelog.

## Non-negotiables

- **Privacy.** User inputs never leave the device, except in a problem report the user previews and sends. No analytics, no cookies, no third-party requests (Turnstile loads only inside the report dialog).
- **Determinism.** No host `Math` in results, no clock, locale, or time-zone reads in tools, and ECMAScript-format number serialization (shortest round trip) produced by the core.
- **Honesty.** Never claim review, verification, or accuracy the build does not prove. The claims gate enforces this.
- **Regulations and standards** are dated reference data with review dates. Proposed rules are labeled "Proposed".
- **Copyright.** Do not reproduce copyrighted tables (AASHTO, ASTM, and similar). Cite the table and take the value as an input.
- **US English**, plain language, reading grade 8 or below for result sentences. No marketing superlatives.
- **Work in a git worktree.** Commit with Conventional Commits.

## Where things live

| Path | What |
|---|---|
| `openspec/changes/` | Specs, designs, tasks (source of truth) |
| `docs/research/` | Research briefs behind the specs |
| `core/` | Rust workspace, one crate per domain |
| `packages/runtime/` | Internal Wasm loader and asset providers (unpublished) |
| `apps/web/` | Astro static site and PWA |
| `mcp/` | Zero-dependency MCP server; release tags include `mcp/dist/` |
| `worker/` | The single Cloudflare Worker (static assets + `/api/reports*`), D1 migrations |
| `data/` | Citations, sources ledger, reference data, glossary, SEO lists |
| `verify/` | Differential test harness against reference implementations |

## Patterns borrowed

The problem-report loop, citation records, freshness ledger, SEO shells, mobile gates, and clone-and-run MCP all follow roughlogic.com. See `docs/research/06-roughlogic-patterns.md` for how it does each one, and copy the pattern rather than reinventing it.
