# Value log

Aggregate signals only. geoprims does not add analytics, cookies, accounts, or per-user identifiers to measure adoption. A missing measurement is recorded as unavailable, not zero.

## Targets

The initial targets come from `plan-launch-and-value-proof` design L5 and are revisited quarterly.

| Review date | Target |
|---|---|
| December 22, 2026 (90 days) | Every hero tool indexed; top-10 average position for at least 10 hero queries; at least 50 GitHub stars; median time to fix confirmed wrong results under 7 days |
| March 22, 2027 (180 days) | At least 1,000 weekly search clicks; at least 3 independent community links; every hero tool reviewed |

## Measurement rules

| Signal | Aggregate source | Cadence |
|---|---|---|
| Search discovery | Google Search Console and Bing Webmaster Tools: impressions, clicks, and top queries | Monthly in `docs/seo-log.md` |
| Correctness | Problem-report counts and median time to fix | Monthly on `/quality` |
| Developer adoption | GitHub stars, clone traffic, and release downloads; package and registry downloads if those channels exist | Monthly |
| Practitioner trust | Named sign-offs and community links or endorsements recorded with permission | Quarterly |
| Usability | 5-second and task-test pass rates | Each hero-tool release |

## Monthly entries

### September 2026: launch baseline

Measurement window: September 23–30, 2026. This first entry was assembled on October 12, after the window closed, so metrics without historical snapshots are marked unavailable.

| Signal | Result | Evidence and limits |
|---|---|---|
| Search discovery | Unavailable | Search Console and Bing Webmaster Tools were not yet verified, and no September query or coverage export exists. |
| Correctness | Unavailable | The repository did not receive a month-end aggregate from the report Worker. `/quality` therefore reports received and time-to-fix as not measured rather than zero; the curated known-issues file has no entries. |
| Developer adoption | Partial baseline | GitHub release `v0.1.0` was published September 23. No month-end star, clone, or download snapshot was kept. The late October 12 snapshot was 0 stars, 3 release-asset downloads, and 1,502 clones from 398 unique cloners over GitHub's rolling 14-day window. npm and the MCP Registry are not distribution channels for this project. |
| Practitioner trust | 0 recorded sign-offs; no community links logged | `docs/review-signoffs.md` has no signed rows. No endorsement or community link has been recorded with permission. |
| Usability | Not run | No 5-person-per-domain 5-second or task-test cohort has been completed, so there is no pass rate to report. |

The late snapshot is a starting point, not a September result. Future entries must be captured at month end and must name the measurement window for rolling GitHub traffic.
