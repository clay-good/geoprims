# Tasks

- [ ] 1 Add the ETP/PNR and wind-profile sources to `data/sources-ledger.json` at their current editions, read in full → verify: the ledger gate passes, and each row's `verificationNote` names the section read
- [ ] 2 `aviation.flight-plan.nav-log` → verify: PHAK chapter 16 vector; every leg's heading and groundspeed equal `heading-groundspeed` on the same inputs; antimeridian and single-leg cases
- [ ] 3 `aviation.performance.climb-plan` → verify: PHAK vector; zero-climb (field at cruise) returns zero time with a note
- [ ] 4 `aviation.performance.etp-pnr` → verify: the source's worked example; with no wind, the ETP is at mid-leg exactly; a headwind moves the ETP toward destination
- [ ] 5 `drone.mission.sorties` → verify: a hand-worked 3-sortie grid; one sortie when the path fits one battery; an error naming the waypoint when a single leg cannot be flown out and back
- [ ] 6 `drone.ops.wind-limit` → verify: published power-law example; at report height the wind is unchanged
- [ ] 7 `drone.ops.vlos-check` → verify: distances match GeographicLib `GeodSolve`; waypoints exactly at the range count as inside
- [ ] 8 `drone.photogrammetry.gcp-plan` → verify: ASPRS table rows; every suggested point lies inside the polygon
- [ ] 9 Visuals from the proposal table → verify: `visual-purpose.test.mjs` passes with each new tool reached
- [ ] 10 Derivation notes, citations, hubs, glossary, search questions for each tool → verify: the full gate sequence, and each tool's own questions rank it first
