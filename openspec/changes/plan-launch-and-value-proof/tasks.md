## 1. Phase 0 exit

- [ ] 1.1 Take one sample tool through every gate on web and MCP (correctness layers, mobile, SEO content, report round trip, both-surfaces); verify all gates are green on a release candidate
- [ ] 1.2 Complete the problem-reporting launch checklist in production; verify results recorded in the runbook (2026-09-23: paused, offline, and kill switch pass on geoprims.com and reporting is on; the round trip, duplicate, and quota rows wait on one send from a person's browser, since the bot check gives an automated browser no token)

## 2. Phase 1 launch

- [ ] 2.1 Bring each hero tool (design L2) to the launch-ready bar (design L4); verify a per-tool checklist in `docs/launch/hero-tools.md` with every box checked
- [x] 2.2 ~~Recruit practitioner reviewers and reach at least 50% reviewed hero tools per audience~~ Owner decision 2026-09-23: launch without practitioner review. Every domain page keeps saying "Not yet independently reviewed by a …" (the claims gate enforces it), the disclaimer is linked from every footer, and correctness rests on the published worked examples, differential tests, and golden vectors; `docs/review-signoffs.md` stays open for sign-offs that arrive later
- [ ] 2.3 Publish the 8 journeys and 25 explainers; verify each journey runs end to end with prefilled steps
- [x] 2.4 ~~Complete export-control and liability review and the privacy page review~~ Owner decision 2026-09-23: launch on the published disclaimer, privacy, accuracy, and security pages without an outside review
- [ ] 2.5 Verify Search Console and Bing, submit the sitemap index, and record the baseline in `docs/seo-log.md`; verify coverage reports show the hero pages discovered
- [ ] 2.6 Publish the MCP server release tag with the prebuilt `mcp/dist/` and the MCPB bundle; verify clone-and-run on a clean checkout with Node alone (no npm package or registry entry: owner decision 2026-09-23)

## 3. Measurement

- [ ] 3.1 Create `docs/value-log.md` with the design L5 signals and targets; verify the first monthly entry after launch
- [ ] 3.2 Hold the 90-day review against targets and re-rank Phase 2 by demand; verify the decision record is committed

## 4. Inventory restatement

- [ ] 4.1 Update the foundation rollup table and README counts to design L6; verify the claims-honesty gate passes
