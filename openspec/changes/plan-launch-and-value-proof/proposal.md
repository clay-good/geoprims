## Why

geoprims has to prove its value before it tries to be exhaustive. A launch with 834 half-verified tools would be worse than one with 30 excellent, reviewed, fast tools that practitioners bookmark and recommend.

This change turns the full plan into a sequenced launch:
- what ships first
- what "good enough to launch" means
- how value is measured without any tracking
- what is deliberately cut or deferred

It changes no behavior, so it carries no spec deltas (`skip_specs: true`). The behavior lives in the other changes. This one decides order, scope, and success criteria.

Research: `docs/research/07` (hero tools, journeys, cuts), `docs/research/05` (SEO and trust), `docs/research/04` (competitors).

## What Changes

- **A phased plan.** Phase 0 is the platform. Phase 1 is launch with about 30 hero tools across four audiences. Phase 2 completes the domains. Phase 3 (v1.1) adds data-heavy additions such as PLSS lookup and full geoid packs.
- **A definition of "launch-ready"** for a hero tool: stable, independently worked example, practitioner-reviewed or disclosed as pending, glance-tested, mobile-gated, indexed, and reachable through the MCP server.
- **Value proof without tracking:**
  - search-console impressions and clicks (aggregate)
  - problem reports and time-to-fix
  - GitHub stars, clones, and releases
  - npm and MCP registry installs
  - practitioner endorsements
  - usability test results
- **A restated inventory:** about 514 operations, about 834 tool ids, and about 300 indexable tool pages, plus explainers, journeys, and trust pages.
- **The cut and defer list,** with reasons.

## Capabilities

### New Capabilities

None (planning change; `skip_specs: true`).

### Modified Capabilities

None.

## Non-goals

- Marketing campaigns, paid acquisition, or growth tactics that need tracking.
- Monetization. geoprims is a free public utility.

## Impact

- Restates the endpoint rollup in `establish-platform-foundation/design.md` and the README.
- Sets the order in which the other changes' tasks are applied.
