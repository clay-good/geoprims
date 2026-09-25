## Context

Every tool here strings together functions that already exist and are verified: geodesic inverse, wind triangle, magnetic variation (WMM), fuel plan, endurance, return-to-home budget, and geodesic distance. The new code is the wiring and the answer sentence, not new formulas. That keeps the correctness surface small.

## Decisions

### D1. Nav log: one tool over all legs, not a page calling the wind triangle N times
`aviation.flight-plan.nav-log` takes waypoints (lat/lon rows, or course and distance rows), TAS, altitude, a wind per leg or one wind for all, variation per leg (or WMM at a date), optional compass deviation, and fuel burn per hour. It returns, per leg: TC, distance, WCA, TH, MH, CH (if deviation is given), GS, ETE, and fuel. It also returns totals and cumulative time and fuel.
- *Alternative: only a workflow that calls `heading-groundspeed` per leg.* Rejected. Agents and the batch runner want the nav log as one call, and the CSV export needs one table.
- Source: FAA-H-8083-25C (PHAK) chapter 16, which walks through a flight log (`faa-phak` in the ledger). Independent vector: that chapter's worked flight. Differential: each leg's heading equals `aviation.wind.heading-groundspeed` on the same inputs, bit for bit.

### D2. Climb plan
Inputs: field elevation, cruise altitude, average rate of climb, climb TAS, the wind in the climb, fuel burn in the climb, and optional start-and-taxi fuel. Outputs: time, fuel, and ground distance to climb, plus top of climb as a distance along the first leg. Source: PHAK chapter 11 (climb performance) and the POH "time, fuel, and distance to climb" table method. The reader may instead give the table's values and the tool subtracts field from cruise, as the POH instructs.

### D3. Equal time point and point of no return
ETP distance from departure = D × GSback / (GSon + GSback). PNR time = E × GSback / (GSout + GSback), where E is safe endurance (fuel minus reserve, divided by burn). Inputs: leg distance, TAS, wind (or GSout and GSback directly), fuel, reserve, burn. The source is added to the ledger before implementation; candidates are the Transport Canada AIM (`tc-aim`) and a national flight-planning manual that is free to read. Independent vector: the worked example in that source.
- *Alternative: leave it out as an airline-only topic.* Rejected. Over-water and remote VFR and IFR pilots use ETP and PNR. The math is short, but it is easy to get backward (whether GS is taken out or home).

### D4. Sorties
`drone.mission.sorties` takes a waypoint path (from the survey grid, corridor, orbit, or facade), a home point, the battery's usable flight time (or energy and power, through `drone.power.endurance`), cruise and transit groundspeed, and the reserve. It walks the path and ends a sortie at the last waypoint from which `drone.power.rth-budget` still returns home within the reserve. It returns the sorties (first and last waypoint, flying time, return time) and the swap points. Wind can be one value applied as a headwind on the return, which is conservative.
- *Alternative: divide total time by battery time.* Rejected. It ignores the trip home, which grows as the path moves away from home. That is the mistake that ends flights in a field.

### D5. Wind limit
Inputs: the reported wind (speed, gust, height of the report, 10 m by default for a METAR), flying height, and the drone's maximum wind rating (or its maximum airspeed and a margin). Wind at height uses the power law with a stated exponent (1/7 over open terrain by default, with a choice for rough terrain). Outputs: sustained wind and gust at height, margin to the limit, and groundspeed into the wind. The wind-profile source joins the ledger with the exponent's terrain classes. The result sentence names the exponent and warns that gusts and turbulence near obstacles are not modeled.

### D6. VLOS check
Inputs: pilot position, the mission's waypoints, and a visual range (entered, or from `drone.sensors.vlos` for the airframe's size). Outputs: the farthest waypoint distance, how many waypoints lie beyond the range, and those waypoints' indexes. It cites 14 CFR 107.31 (`cfr-14-107`), and the result says visual line of sight is the pilot's judgment on the day, not a distance.

### D7. GCP plan
Inputs: the area polygon and the accuracy class. Outputs: the checkpoint count per ASPRS Edition 2 (`asprs-pas`, the table on product area), a GCP count and suggested layout (perimeter corners plus the interior at a stated spacing, labeled a rule of thumb with its source), and the points as lat/lon rows for export.

## Tool inventory

| Family | Tool | Endpoints | Reference source | Independent vector |
|---|---|---|---|---|
| Aviation: flight plan | nav-log | 1 | faa-phak ch. 16 | PHAK worked flight |
| Aviation: performance | climb-plan | 1 | faa-phak ch. 11 | PHAK climb table example |
| Aviation: performance | etp-pnr | 1 | Added in task 1 | That source's worked example |
| Drone: mission | sorties | 1 | Composition of endurance and rth-budget | Hand-worked grid, 3 sorties |
| Drone: ops | wind-limit | 1 | Added in task 1 (wind profile) | Published power-law example |
| Drone: ops | vlos-check | 1 | cfr-14-107 (107.31) | Geodesic distances from GeographicLib |
| Drone: photogrammetry | gcp-plan | 1 | asprs-pas (Ed. 2) | ASPRS table rows |

Total: 7 operations, 7 endpoints.
