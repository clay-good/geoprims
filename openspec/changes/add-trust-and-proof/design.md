## Context

The pattern source is `docs/research/06-roughlogic-patterns.md` §4. roughlogic.com runs all of the following:
- a citation record per tile (formula, edition, free access, governance, assumptions)
- `sources-cycle.json` with `next_expected`, and a gate that fails when a manifest names a non-current edition (added after ICC published the 2027 IMC the same day a page claimed 2024 was current)
- monotonic data stamps
- 3,145 worked-example rows with source and section
- dimension annotations and a bounds fuzzer
- a "both doors" gate
- a claims-honesty gate
- a reviewer sign-off layer, which it honestly discloses has never run

geoprims adopts all of it and adds show-your-work rendering, downloadable vectors, and a `/sources` page.

## Goals / Non-Goals

**Goals:**
- Every number on the site is traceable, current, and honestly qualified.

**Non-Goals:**
- Replacing professional judgment or certification.

## Decisions

### T1. Citations as data, not prose
Citation records live in `data/citations/<domain>.json`, keyed by tool id, validated by schema, and rendered by one component used on pages, in print, in exports, and in MCP. Prose inside docs may reference them but never replaces them. This mirrors roughlogic's `CITATIONS[toolId]` while splitting by domain to keep files reviewable.

### T2. The ledger drives both CI and the UI
`data/sources-ledger.json` is the single source of edition, status, and dates. The UI reads "last verified" from it, and CI gates read the same file. There is no second place to update.

### T3. Show-your-work from the core
The Wasm core returns, alongside results, an optional `trace`: an ordered list of named intermediate values with units, plus the substituted-expression template ids. The UI renders formulas from build-time MathML templates filled with trace values. The trace is deterministic and byte-identical in the MCP, where `geoprims_run` accepts `explain: true`.

Alternative considered: computing the displayed steps separately in JavaScript. Rejected: two code paths could disagree.

### T4. Reviewer program
Recruit at least one qualified reviewer per domain before a domain's hero tools launch:
- a PLS (licensed surveyor) for survey
- a CFI or dispatcher for aviation
- a Part 107 mapping professional for drone
- a geodesist or GIS engineer for geodesy and indexing

Reviews are scoped to named tools and dated. Absence is disclosed, never hidden (roughlogic's lesson).

### T5. Tolerance ceilings per domain

| Domain | Default ceiling |
|---|---|
| Geodesy positions | 1 mm |
| Geodesy angles | 1e-6° |
| Aviation | 0.1% or 1 unit of display precision |
| Drone | 0.5% |
| Survey | 0.001 of the unit |
| Indexing | Exact |

Anything looser needs a justification in the fixture.

## Risks / Trade-offs

- **[The gates slow shipping]** → That is the point for stable tools. Experimental tools can ship behind the badge while gates are completed.
- **[Reviewer availability]** → Launch with honest "review pending" disclosures. Prioritize reviews for hero tools.
- **[Ledger upkeep burden]** → The overdue check forces a periodic look. Rows carry issuer URLs to make re-verification a 2-minute task.

## Migration Plan

Not applicable.

## Open Questions

None that affect the specs.
