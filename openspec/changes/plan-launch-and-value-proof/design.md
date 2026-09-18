## Context

The full plan now spans 15 changes. Without sequencing, the risk is building breadth before proving depth. This document sets the order, the launch bar, and the measures of success.

## Goals / Non-Goals

**Goals:**
- Launch a small set of excellent tools that practitioners trust and share.
- Measure value with aggregate, tracking-free signals.

**Non-Goals:**
- Launching everything at once.

## Decisions

### L1. Phases

| Phase | Contents | Exit criteria |
|---|---|---|
| **0: Platform** | `establish-platform-foundation`, `build-web-experience` (except audio), `add-trust-and-proof` infrastructure, `add-glanceable-and-field-ux`, `add-problem-reporting`, `add-seo-and-discoverability` infrastructure, `add-local-mcp-server` | A sample tool passes every gate end to end on web and MCP. The report round trip works in production. |
| **1: Launch** | The ~30 hero tools (L2), the 8 journeys (L3), 25 explainers, and trust pages | Every hero tool is launch-ready (L4). Legal review is done. Search consoles are verified. |
| **2: Complete domains** | The remaining operations in each domain move to stable, in order of search demand from the SEO log | Monthly: each promoted tool meets the stable bar. |
| **3: v1.1 data** | PLSS lookup (BLM CadNSDI), EGM2008 1′ and GEOID18 full packs, tz-boundaries, optional basemap | Assets pass the integrity and privacy tile rules. |
| **Post-launch** | Audio feedback, A5 promotion, least-squares marketing, MCP Apps canvas | Driven by problem reports and demand. |

### L2. Hero tools at launch

| Audience | Tools (target queries) |
|---|---|
| Pilots | Crosswind/headwind (crosswind calculator); density altitude; pressure altitude; E6B wind triangle; sunrise, sunset, and the four nights (civil twilight, night currency); METAR/TAF decoder; winds-aloft decoder; holding entry; Zulu time; weight and balance; top of descent / VDP / glidepath VS |
| Drone operators | GSD and altitude-for-GSD; overlap, trigger, and image count; flight time and mAh→Wh; mapping sun window; VLOS guidance; Part 107 altitude and structure rule |
| Surveyors | DMS↔decimal and UTM/MGRS converter; state plane converter; geoid and orthometric height; grid↔ground combined factor; traverse closure (Bowditch); deed plotter; horizontal and vertical curves; acreage from coordinates |
| Developers | H3 cell and k-ring; tile, quadkey, and bbox; geohash; geodesic distance (vs haversine); magnetic declination |

Selection criteria: daily practitioner need, search volume, competitors that are ad-heavy or wrong, and no heavy data dependency.

### L3. Journeys at launch
1. VFR preflight
2. VFR cross-country
3. IFR approach brief
4. Drone mapping day
5. GNSS control to OPUS to state plane
6. Boundary retracement from a deed
7. Lidar bid
8. Developer: index to area

Each journey is a page with prefilled steps (per `web/tool-docs` and `discovery/search-pages`).

### L4. Launch-ready bar for a hero tool
- **Stable per `trust/correctness-program`:** all layers pass, including at least one independent worked example.
- **Reviewed:** practitioner review signed off, or a "review pending" disclosure. At least 50% of hero tools in each audience must be reviewed at launch.
- **Usable:** passed the 5-second and task tests (≥ 80%).
- **Mobile-gated:** 320 px, landscape, 200% zoom, WebKit.
- **Findable:** an indexable page meeting the content minimums, an OG image, and an explainer link.
- **MCP-reachable:** the both-surfaces gate passes, and `summary` and references are present.
- **Reportable:** the report button is wired, and the known-issues banner is supported.

### L5. Proving value without tracking

| Signal | Source | Cadence |
|---|---|---|
| Search impressions, clicks, and top queries | Google Search Console, Bing Webmaster (aggregate) | Monthly in `docs/seo-log.md` |
| Correctness | Problem reports: received, confirmed, median time-to-fix | Monthly on `/quality` |
| Developer adoption | GitHub stars, clones, and release downloads (repository traffic API); npm weekly downloads; MCP Registry listing | Monthly |
| Practitioner trust | Named reviewer sign-offs; unsolicited endorsements or links from flying clubs, state survey societies, and drone communities (logged with permission) | Quarterly |
| Usability | 5-second and task-test pass rates | Per release of hero tools |

**Initial targets**, revisited quarterly:

| Horizon | Targets |
|---|---|
| 90 days | Every hero tool indexed. Top-10 average position for at least 10 hero queries. At least 50 GitHub stars. Median time-to-fix under 7 days for confirmed wrong results. |
| 180 days | At least 1,000 weekly search clicks. At least 3 independent community links. Every hero tool reviewed. |

No per-user data is collected to measure any of these.

### L6. Restated inventory

| Domain | Operations | Tool ids | Notes |
|---|---|---|---|
| geodesy | 89 | 173 | |
| navigation | 46 | 58 | |
| geometry | 38 | 44 | |
| aviation | 97 | 117 | +weather, +IFR; −Koch; alias slugs not counted |
| drone | 49 | 49 | +sensors and links; alias slugs not counted |
| survey | 72 | 72 | +land descriptions, +GNSS; −angle of repose |
| indexing | 52 | 92 | |
| raster | 30 | 40 | |
| time | 19 | 19 | new domain |
| units | 22 | 170 | |
| **Total** | **514** | **834** | |

Indexable pages are about 300 tool pages (stable operations with full content, plus at most 60 high-intent conversion pages), plus at least 25 explainers, 8 journeys, domain and group hubs, and trust pages. The rest resolve as presets canonicalized to their parent (per `discovery/search-pages`).

### L7. Cut and defer list

| Item | Decision | Reason |
|---|---|---|
| Koch-chart takeoff estimate | Cut | Would be mistaken for aircraft data |
| Angle-of-repose table | Cut | Unclear sources; slope-design liability |
| AASHTO design values | Input only | Copyright |
| Blanket generated pair pages | Canonicalized presets | Scaled-content risk |
| Audio feedback | Post-launch | Does not prove value |
| A5 grid | Experimental | Pre-1.0 upstream |
| Least-squares adjustment | Experimental, unmarketed | Scope and liability |
| GEOREF, orthographic, and equidistant-cylindrical pages | Tool ids and preset routes only (canonicalized, not indexed) | Low demand |
| FAA cold-temperature airport list | Link only | Republished annually |
| Terrain-following export | Gated by acknowledgment and margin | GLO-30 is a surface model |
| PLSS lookup | v1.1 | Data size |
| WebMCP, CLI, public library | Not planned | Two surfaces only |

## Risks / Trade-offs

- **[Reviewer recruitment slows launch]** → The 50% reviewed bar, with honest disclosure for the rest.
- **[Hero tools miss real demand]** → The quarterly review re-ranks from the SEO log and problem reports.
- **[Scope creep back toward breadth]** → New tools enter Phase 2 only through demand evidence (queries, reports, requests).

## Migration Plan

Not applicable.

## Open Questions

None.
