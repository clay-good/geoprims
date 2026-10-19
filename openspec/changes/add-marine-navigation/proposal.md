## Why

The navigation domain was built for "pilots, mariners, drone operators, and developers," but every example speaks aviation. A mariner, a sailing student, or a merchant-marine candidate working Bowditch problems finds the geodesic, rhumb, horizon, and CPA tools and then stops: there is no plane, mid-latitude, or Mercator sailing in the forms the exams and the ship's log use, no set-and-drift triangle, no running fix, no distance by vertical angle, no tide prediction, and no under-keel clearance. Those are the daily questions on a bridge and in a navigation class, the free sites that answer them are thin, and every one has a public-domain source with worked examples: *The American Practical Navigator* (Bowditch, NGA Pub. 9, 2019), NOAA's harmonic constituents and the Schureman manual, and the USACE squat notes.

This change adds marine navigation to the navigation domain without changing its character: the same ellipsoid tools underneath, Bowditch's conventions on top, and the exam's worked examples as golden vectors.

Depends on: `add-navigation-and-geometry` (geodesics, rhumb lines, CPA, horizon), `add-geodesy-suite` (magnetic variation), `add-practitioner-essentials` (time scales, time zones), `add-job-workflows`.

## What Changes

Four new groups in `navigation`, 15 operations:

| Group | Tools | What they answer |
|---|---|---|
| `sailing` | plane, traverse, parallel, mid-latitude, mercator, great-circle (with composite limiting latitude) | Course and distance between positions, and the position reached, by each of Bowditch's sailings, with the ellipsoidal answer shown beside the sphere's |
| `piloting` | set-and-drift, dead-reckoning, running-fix, two-bearing-fix, distance-by-vertical-angle, compass-conversion | Current triangles (track and speed made good, course to steer), DR positions, fixes from bearings and a run, distance off a charted object, and TVMDC compass work with deviation |
| `tide` | predict, datums | Tide heights and the times of high and low water from pasted NOAA harmonic constituents, and conversion of a height between tidal datums from a pasted datum sheet |
| `vessel` | under-keel-clearance | Clearance under the keel from charted depth, tide, draft, squat (Barrass, via USACE), and a safety margin |

Every result carries the planning notice ("Planning and education aid. Not for navigation."), names its source and section, and shows Bowditch's rounding beside full precision.

## Capabilities

### New Capabilities

- `navigation/sailings`: The six sailings in Bowditch's forms.
- `navigation/piloting`: Current triangles, dead reckoning, fixes, distance by vertical angle, and compass conversion.
- `navigation/tides`: Harmonic tide prediction from pasted constituents and datum conversions.
- `navigation/under-keel-clearance`: Under-keel clearance with squat.

### Modified Capabilities

None.

## Non-goals

- Fetching tide constituents, datums, or predictions. The user pastes the NOAA station page or JSON.
- Tidal currents (ebb and flood from current constituents) in this change; they share the harmonic engine and can follow.
- Electronic chart, AIS, radar, or any operational data.
- Celestial sight reduction, which is `add-celestial-and-moon`.
- Ship-specific hydrodynamics beyond the published squat formulas; the vessel tool takes block coefficient and speed as inputs.

## Impact

- `core/gp-navigation` grows by 15 operations and the harmonic-constituent engine (Schureman node factors and equilibrium arguments for the 37 NOAA constituents); its 400 KB budget is checked, with the tide engine split into its own module if needed.
- `data/taxonomy.json` adds `sailing`, `piloting`, `tide`, and `vessel` under `navigation`; the MCP `navigation` toolset is split (it is at its cap).
- New sources-ledger rows: Bowditch 2019 (volumes I and II), Schureman SP 98 (1940, 1958 reprint), Parker NOS CO-OPS 3 (2007), NOAA CO-OPS harmonic constituents and datums (data), NCEP `calctide.f` (reference), USACE ERDC CHETN-I-72 and CHETN-IX-19/20 (Briggs), NOAA tide predictions (oracle).
- A new workflow, "Passage leg" (sailing, set and drift, ETA, tide at arrival, under-keel clearance), and an explainer each for the sailings, set and drift, and tidal datums.
