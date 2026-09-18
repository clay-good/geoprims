## Why

A practitioner panel review of the v1 inventory (`docs/research/07-practitioner-gap-analysis.md`) found the math deep but the everyday tasks thin. Those everyday tasks are what bring pilots, drone operators, and surveyors back daily:
- When does "night" start for logging vs currency vs position lights vs Part 107?
- What time is it in Zulu?
- What does this METAR or winds-aloft line mean?
- Which holding entry?
- Where is the VDP?
- Does this deed close?
- What is this PLSS description?
- How long do I sit on this point for OPUS?
- How far can I fly this drone under VLOS guidance?

Most of these are high-search-volume queries served by ad-heavy or wrong pages. None needs server data beyond tiny public-domain tables.

Depends on: `establish-platform-foundation`, `add-geodesy-suite`, `add-navigation-and-geometry`, `add-aviation-suite`, `add-drone-suite`, `add-survey-suite`.

## What Changes

About 70 new operations across a new `time` domain and existing domains:

- **Solar and twilight (`time`):**
  - solar position (NREL SPA, with NOAA as cross-check)
  - sunrise, sunset, and all three twilights, with polar day and night states
  - the four US aviation "nights" (§1.1 logging, §61.57(b) currency, §91.209 lights, §107.29 drones)
  - night-currency counting from landing dates
  - photogrammetry sun window and hotspot, shadow length, and sun on a slope
- **Time scales (`time`):** UTC↔local with an explicit offset or named zone (versioned tzdb snapshot), Zulu, decimal hours↔h:mm, GPS week/seconds (with rollover), GPS/TAI/UTC offsets from a leap-second table, Julian date and MJD, day of year.
- **Weather decoding (`aviation`):** METAR/SPECI and TAF (paste only) per WMO FM 15/16 plus FAA JO 7900.5 US remarks, FB winds-and-temperatures-aloft lines, and hand-off of decoded values to the altimetry and wind tools.
- **Instrument procedures (`aviation`):**
  - holding entry sector, wind-corrected outbound heading and timing, and dated maximum holding speeds
  - DME slant/ground range and DME-arc lead radial
  - time and distance to station
  - glidepath vertical speed, VDP, TCH, and VASI/PAPI geometry
  - NOTAM/TFR circle geometry from packed coordinates or fix-radial-distance with a user-supplied navaid
- **Land descriptions (`survey`):**
  - a metes-and-bounds deed parser with plot, closure, and area; every parsed call is shown for confirmation, and curve calls are flagged
  - legacy land units (chain, link, rod, vara by jurisdiction, arpent)
  - a PLSS legal-description parser and aliquot acreage math in v1; the PLSS↔lat/lon lookup is an on-demand BLM CadNSDI asset in v1.1
  - basis-of-bearing rotation
- **GNSS field tools (`survey`):**
  - DOP and sky plot from a pasted almanac
  - RTK/PPK error budget (a + b ppm)
  - OPUS session chooser
  - slant-to-vertical antenna height (NGS ANTINFO offsets)
  - ALTA/NSPS 2026 relative positional precision check
  - 2D similarity/affine localization with residuals
- **Sensors and links (`drone`):**
  - VLOS/ALOS/DLOS range (EASA guidance, labeled)
  - Part 107 civil-twilight window
  - lidar planning (swath, density, USGS quality levels)
  - dataset size estimate
  - radio link budget
  - thermal pixel footprint
- **Cuts and demotions from the panel's review:**
  - remove `koch-estimate` and `angle-of-repose-reference`
  - make the sight-distance design K a cited input
  - gate terrain-following export behind a surface-model acknowledgment and margin
  - collapse generated pairs per `discovery/search-pages`

## Capabilities

### New Capabilities

- `time/solar-and-twilight`: Sun position, rise/set/twilight, legal night definitions, currency, and photogrammetry lighting.
- `time/time-scales`: Civil, aviation, GNSS, and astronomical time conversions.
- `aviation/weather-decoding`: Decoding pasted METAR, SPECI, TAF, and FB winds-aloft text.
- `aviation/instrument-procedures`: Holding, DME, approach geometry, and NOTAM/TFR geometry.
- `survey/land-descriptions`: Deed parsing and plotting, legacy land units, and PLSS descriptions.
- `survey/gnss-field`: GNSS planning, error budgets, antenna heights, precision standards, and localization.
- `drone/sensors-and-links`: VLOS guidance, lighting window, lidar planning, data volume, links, and thermal footprint.

### Modified Capabilities

None (the cuts edit unarchived specs in place; see Impact).

## Non-goals

- Fetching weather, NOTAMs, almanacs, or navaid data. Everything is pasted or typed by the user.
- Celestial navigation sight reduction (possible later).
- Legal interpretation of deeds or PLSS descriptions. The tools do parsing and math, with every assumption shown.

## Impact

- **Adds the `time` domain** to `platform/tool-catalog`.
- **New assets:**
  - `leap-seconds` (IERS, public domain, tiny)
  - `tzdb` (IANA, public domain, versioned snapshot, about 400 KB)
  - optional `tz-boundaries` (timezone-boundary-builder, ODbL, on demand)
  - optional `ngs-antinfo` (NGS, public domain, small)
  - v1.1 `plss-cadnsdi` (BLM, public domain, tiled per state)
- **Reference data:** holding speeds, the four night definitions, ALTA 2026 RPP, and USGS 3DEP quality levels, all dated.
- **Edits to unarchived specs:** aviation (remove the Koch estimate), survey (sight-distance input, remove the angle-of-repose table), drone (terrain-following acknowledgment).
