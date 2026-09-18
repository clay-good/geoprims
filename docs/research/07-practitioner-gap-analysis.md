# Research: practitioner gap analysis, hero tools, cuts, and journeys

> Practitioner-panel review (surveyor, CFI/dispatcher, Part 107 mapping pro, GIS developer, UAM engineer) of the v1 inventory, gathered 2026-09-18. Recommendations are adopted or declined in `openspec/changes/add-practitioner-essentials` and `plan-launch-and-value-proof`.


I read the tool inventory tables in all six domain `design.md` files, skimmed each `spec.md` requirement heading, and checked the platform contracts that constrain new tools. No files were edited.

The inventory is deep on math that is hard to get right, such as Karney geodesics, compressibility, and the frame and epoch handling. It is thin on the everyday tasks that bring practitioners back: sun and time, decoding weather text, hold entries, deeds, PLSS, GNSS field logistics and the drone flight-day checks.

Two constraints in the specs shape everything below:
- **No clock, locale or time-zone reads.** `platform/tool-contract` ("Tools SHALL NOT read the clock, locale, time zone…"). Sun and time tools must take an explicit date and offset, and must not quietly use the browser's `Intl` time-zone data.
- **No operational data.** `add-aviation-suite/proposal.md` excludes METAR/TAF, NOTAMs and airport databases. Decoding text the user pastes is compatible with this; fetching it is not.

---

## A. Gaps, ranked by value for a v1 launch

Each entry gives who uses it, why it matters, the formula or standard, and whether it needs data.

### Tier 1: add before launch

1. **Sun and twilight: sunrise, sunset, civil, nautical and astronomical twilight, solar azimuth and elevation, solar noon, shadow length.**
   - **Who:** every pilot, drone pilot and photogrammetrist, plus surveyors doing solar azimuth observations.
   - **Why:** this is the biggest missing piece. There are three different legal "nights" (see E1), a drone lighting rule, and the photogrammetry lighting window. Pix4D and GeoCue users plan around sun elevation above 30° and around the "hotspot" near solar noon ([GeoCue](https://support.geocue.com/flight-planning-sun-angle/), [Drones Made Easy](https://support.dronesmadeeasy.com/hc/en-us/articles/360060320672-Sun-Hotspots)).
   - **Formula:** NREL SPA, ±0.0003°, valid from year −2000 to 6000 ([NREL SPA](https://midcdmz.nrel.gov/spa/)), which beats the NOAA/Meeus low-accuracy method (about 0.01°, about 1 minute) ([NOAA](https://gml.noaa.gov/grad/solcalc/calcdetails.html)). Show NOAA's result alongside for cross-checking.
   - **Data:** none, apart from ΔT and the leap-second table (below).

2. **Aviation "night" and currency calculator.** Built on #1.
   - **Why:** it answers three different questions for a date and place:
     - §1.1 night (logging): end of evening civil twilight to start of morning civil twilight.
     - §61.57(b) passenger currency: 1 hour after sunset to 1 hour before sunrise ([eCFR 61.57](https://www.ecfr.gov/current/title-14/chapter-I/subchapter-D/part-61/subpart-A/section-61.57)).
     - §91.209 position lights: sunset to sunrise.
   - It should also count the 90-day window from landing dates the user enters. Student pilots mix these up constantly ([Wikipedia summary](https://en.wikipedia.org/wiki/Night_aviation_regulations_in_the_United_States)).
   - **Data:** none. The rule text goes in the dated regulatory file (AV4).

3. **UTC, Zulu and time utilities.**
   - **Scope:** local↔Zulu with an explicit offset; decimal hours↔h:mm (Hobbs and tach logging); a time-of-day calculator; GPS week and seconds-of-week↔UTC; GPS, TAI and UTC offsets; Julian date and MJD; day of year (needed for RINEX and OPUS file names).
   - **Leap seconds:** UTC−TAI = −37 s since January 1, 2017, and GPS−UTC = +18 s. IERS Bulletin C 72 confirms no leap second at the end of 2026 ([IERS](https://datacenter.iers.org/data/html/bulletinc-072.html)).
   - **Data:** a tiny leap-second table, public domain.
   - **Optional:** looking up the time zone from coordinates needs timezone-boundary-builder, which is ODbL and tens of MB ([repo](https://github.com/evansiroky/timezone-boundary-builder)). Make it an on-demand asset. Rules come from the IANA tzdb (public domain), bundled and versioned.

4. **METAR/SPECI and TAF decoder (paste only).**
   - **Who:** student pilots, CFIs, dispatchers and drone pilots.
   - **Why:** very high search volume, and it feeds the pressure altitude, density altitude and crosswind tools directly. The paste-to-detect feature already recognizes `A2992`.
   - **Standard:** WMO FM 15/16 plus the US-specific remarks in FAA JO 7900.5 (SLP, the T group, PK WND and others) ([JO 7900.5E Chg 1](https://www.faa.gov/documentLibrary/media/Order/JO_7900.5E_CHANGE_1.pdf)).
   - **Data:** none.

5. **FB winds and temperatures aloft decoder.**
   - **Why:** a classic student trap. `9900` means light and variable. Speeds of 100–199 kt are encoded by adding 50 to the direction and subtracting 100 from the speed (`731960` = 230° at 119 kt, −60 °C). Above 24,000 ft the minus sign is implied, and no temperature is given at 3,000 ft ([Gleim](https://www.gleim.com/public/pdf/av/private/online/links/faexample6_5.pdf)).
   - Feed the result into the existing `winds-aloft-interpolation` and `heading-groundspeed`.
   - **Data:** none.

6. **Holding: entry sector, timing and wind correction.**
   - **Scope:** the entry sector (direct 180°, teardrop 70°, parallel 110°, per [AIM 5-3-8](https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap5_section_3.html)), outbound heading with triple-drift correction, outbound timing adjusted for wind, maximum holding speeds as dated reference data, and an optional non-standard (left-turn) hold.
   - **Why:** every IFR student and CFII uses this. Existing online versions are mostly bare diagrams.
   - **Data:** none. The speed table is regulatory reference data.

7. **VOR/DME and instrument approach geometry.**
   - **Scope:**
     - DME slant range → ground distance.
     - DME arc lead radial.
     - Time and distance to station from wingtip bearing change (60 × minutes ÷ degrees).
     - Radial intercept angle.
     - Glidepath vertical speed (GS × 101.27 × tan θ, next to the "×5" rule).
     - VDP (HAT ÷ tan θ; HAT/300 as the rule of thumb, HAT/318 for a true 3°) ([AeroCorner](https://aerocorner.com/blog/visual-descent-point-vdp/)).
     - TCH and threshold crossing geometry.
     - VASI/PAPI angle to height.
   - `visual-descent-point` and `descent-angle` already exist, so this mostly means adding four to six operations to `performance`/`route`.
   - **Data:** none. NASR navaids are public domain ([FAA NASR](https://www.faa.gov/air_traffic/flight_info/aeronav/aero_data/NASR_Subscription/)) but they are operational data, so defer them.

8. **Metes-and-bounds deed parser, plotter and closure.**
   - **Who:** surveyors, title examiners, landowners and land agents.
   - **Why:** it combines the existing `direction-parse`, `traverse-closure` and `area-by-coordinates` behind one hero input: a pasted deed, parsed into calls, drawn as a plot, with closure, precision ratio and acres.
   - **Competitors:** there are many (MeteMap, SlateTablet, Tract Plotter, CADastral) ([MeteMap](https://metemap.com/), [SlateTablet](https://www.slatetablet.com/tools/deed-plotter)), but few handle curve calls, "thence along said line" references, or chain, rod and vara units correctly.
   - **Data:** none.
   - **Note:** the survey proposal lists "deed interpretation" as a non-goal. Keep it as parsing plus math, with every parsed call shown for the user to confirm. That keeps it a computational aid, not interpretation.

9. **Legacy land units converter.** Chains, links, rods, poles and perches, Gunter's and Ramsden's chains, varas (the Texas vara is 33⅓ in by statute, and other Spanish-grant varas differ), arpents (French length and area), acres, hectares, sections, and US survey ft vs international ft.
   - **Why:** high search volume and very common in older deeds.
   - **Data:** a small cited table.

10. **PLSS: township, range and section.**
    - **Scope:** parse a legal description (`NE¼ SW¼ Sec 12, T3N R4W, 6th PM`), convert PLSS↔lat/lon, and compute aliquot-part acres.
    - **Who:** surveyors, landmen, oil and gas, ag, forestry and hunters.
    - **Why:** huge search demand. Competitors include EarthPoint, QSTR.us, Township America and randymajors ([EarthPoint](https://www.earthpoint.us/TownshipsSearchByLatLon.aspx)).
    - **Data required:** BLM CadNSDI, public domain ([BLM](https://gis.blm.gov/arcgis/rest/services/Cadastral/BLM_Natl_PLSS_CadNSDI/MapServer), [data.gov](https://catalog.data.gov/dataset/blm-id-cadnsdi-plss-township-hub)). The national section layer is far too large to bundle, so ship township polygons, simplified, as a few MB, and sections tiled per state on demand.
    - The PLSS cannot be computed from a formula; see trap E6.
    - **Recommendation:** v1 should include the parser and aliquot math with no data. v1.1 adds the lookup.

11. **GNSS field planning.** Several small tools:
    - **DOP and sky plot** from a pasted YUMA/SEM almanac or RINEX navigation file. This is Trimble GNSS Planning's core feature ([gnssplanning.com](https://www.gnssplanning.com/)), with no fetching.
    - **RTK/PPK error budget:** horizontal and vertical = a + b·ppm × baseline, using the manufacturer's spec.
    - **OPUS session chooser:** OPUS-RS for 15 min to 2 h, OPUS-S for 2 h or more ([NGS OPUS](https://geodesy.noaa.gov/OPUS/about.jsp)).
    - **Slant antenna height → vertical height to the antenna reference point**, using the antenna radius and offset. This is a very common source of errors.
    - **Data:** none. The antenna offsets come from NGS ANTINFO, which is public domain and small, as an optional asset.

12. **ALTA/NSPS Relative Positional Precision check.**
    - **Formula:** allowed = 2 cm (0.07 ft) + 50 ppm × distance, at 95%. The 2026 standards took effect February 23, 2026, and redefine RPP as the semi-major axis of the relative error ellipse (2.448σ) ([ALTA 2026 PDF](https://cdn.ymaws.com/nsps.us.com/resource/resmgr/alta_standards/2026_OFFICIAL_FINAL_PDF_ALTA.pdf), [American Surveyor](https://amerisurv.com/2026/02/01/the-2026-minimum-standard-detail-requirements-for-alta-nsps-land-title-surveys/)).
    - Pairs naturally with the experimental least-squares adjustment's error ellipses.
    - **Data:** none. The standard is regulatory reference data.

13. **2D similarity / Helmert transform ("localization" or "site calibration").**
    - **Who:** every GNSS surveyor using Trimble Access or Carlson SurvCE.
    - **Scope:** fit 4 parameters (or 6-parameter affine) from control pairs, with residuals and scale in ppm. Also rotate a deed to a new basis of bearing.
    - **Data:** none.

14. **Drone VLOS, ALOS and DLOS range.**
    - **Formula:** the EASA method, ALOS ≈ 327 × characteristic dimension (m) + 20 m for multirotors, with VLOS = min(ALOS, DLOS) ([EASA guidelines](https://www.easa.europa.eu/en/downloads/139435/en), [EU Drone Port](https://eudroneport.com/blog/calculate-vlos-distance/)).
    - **Why:** a daily question ("how far can I fly this drone?"). The US has no formula, so label the result as EASA guidance.
    - **Data:** none.

15. **Part 107 lighting window.**
    - **Rule:** under §107.29, civil twilight is fixed at 30 minutes before sunrise and after sunset outside Alaska, with anti-collision lighting visible for 3 SM required ([eCFR 107.29](https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.29)). This is not the −6° astronomical definition (see E1).

16. **Lidar mission planning.**
    - **Scope:** swath = 2H·tan(FOV/2); point density ≈ PRR ÷ (speed × swath) × number of returns, adjusted for overlap. Check the result against USGS 3DEP quality levels: QL1 ≥ 8 pulses/m², QL2 ≥ 2 pulses/m², RMSEv ≤ 10 cm ([USGS LBS tables](https://www.usgs.gov/ngp-standards-and-specifications/lidar-base-specification-tables)). Also strip overlap and line count.
    - **Who:** the growing drone lidar market (DJI L2, and others).
    - **Data:** none.

17. **Dataset size and processing estimate.** Image count (already exists) × size per image → raw GB. Orthomosaic pixels = area ÷ GSD², multiplied by bands and bit depth, with a COG compression factor. Point cloud size in LAS or LAZ bytes. A heuristic, clearly labeled, but people search for it.

18. **Radio link budget.** Free-space path loss (20·log₁₀(d) + 20·log₁₀(f) + 32.44 in km and MHz) plus fade margin, paired with the existing Fresnel clearance. Needed for C2 links and BVLOS planning. No data.

19. **Thermal inspection spot size.** IFOV × distance = pixel footprint, and minimum resolvable target size using a 3×3 pixel rule. Used for solar and roof inspections. No data.

### Tier 2: v1.1

- **Fuel by type.** Jet-A density varies with temperature (about 6.7 lb/gal nominal), 100LL is about 6.0 lb/gal, and 100LL is being replaced by G100UL and UL94. Existing `fuel-weight` can take a density–temperature option.
- **Visibility and RVR conversions.**
- **Oxygen and pressurization rule reference (§91.211)**, as dated data.
- **Wind limit check for drones.** Maximum wind as a percentage of airspeed, with gust factor.
- **GCP count and layout.** Heuristic ("5–10 plus checkpoints"), with the ASPRS requirement of at least 30 checkpoints (already in the specs).
- **Solar azimuth observation reduction** for surveyors (hour-angle method), built on #1.
- **Standard parallel and correction line explainer**, which comes with PLSS.
- **NOTAM and TFR geometry.** Parse `393400N1224330W` (the parser already handles packed forms), add radius in NM → circle polygon (`buffer-point`), and fix-radial-distance `ABC012098.7` given a user-supplied navaid position ([FAA TFR guide](https://www.faa.gov/sites/faa.gov/files/pilots/safety/notams_tfr/tfrweb.pdf)). This is a compelling demo, and needs no data if the user types the VOR's coordinates.

---

## B. Hero tools: the ~25 most likely to drive launch traffic

Selection criteria: high search volume, daily use, and competitors that are ad-heavy or get the answer wrong.

| # | Tool | Target search queries |
|---|---|---|
| 1 | Crosswind / headwind (exists) | crosswind calculator, crosswind component calculator, headwind tailwind calculator |
| 2 | Density altitude (exists) | density altitude calculator, how to calculate density altitude, DA calculator |
| 3 | E6B wind triangle (exists) | e6b calculator online, wind correction angle calculator, groundspeed calculator |
| 4 | Sunrise, sunset and twilight (new) | civil twilight today, sunset time calculator, end of civil twilight [city] |
| 5 | Night currency / logging night (new) | night currency calculator, when does night start for logging, 1 hour after sunset calculator |
| 6 | METAR decoder (new) | metar decoder, decode metar, how to read a metar, taf decoder |
| 7 | Winds aloft decoder (new) | winds aloft decoder, how to read winds aloft 9900, FB winds decode |
| 8 | Holding entry (new) | holding pattern entry calculator, hold entry teardrop parallel direct |
| 9 | Zulu / UTC converter (new) | zulu time converter, utc to local time, convert zulu time |
| 10 | Weight and balance (exists) | weight and balance calculator, cessna 172 weight and balance, cg calculator |
| 11 | Descent / VDP / TOD (exists; extend) | top of descent calculator, VDP calculation, descent rate calculator 3 degree |
| 12 | Pressure altitude (exists) | pressure altitude calculator, altimeter setting to pressure altitude |
| 13 | GSD calculator (exists) | gsd calculator, ground sample distance drone, altitude for gsd |
| 14 | Drone flight time / mAh to Wh (exists) | drone flight time calculator, mah to wh, battery c rating calculator |
| 15 | Photo overlap and mission (exists) | drone overlap calculator, photogrammetry flight planning, image count calculator |
| 16 | Sun angle for mapping (new) | sun elevation calculator, best time to fly drone mapping, shadow length calculator |
| 17 | VLOS distance (new) | vlos calculator, how far can I fly my drone, drone visibility distance |
| 18 | Coordinate converter DD/DMS/UTM/MGRS (exists) | dms to decimal, utm converter, mgrs converter, lat long to utm |
| 19 | State plane converter (exists) | state plane coordinate converter, spcs83 converter, state plane zone lookup |
| 20 | Geoid / ellipsoid↔orthometric height (exists) | geoid height calculator, GEOID18, ellipsoid height to orthometric, NAVD88 |
| 21 | Grid-to-ground combined factor (exists) | combined scale factor calculator, grid to ground conversion |
| 22 | Deed plotter / metes and bounds (new) | metes and bounds plotter, deed plotter free, plot legal description |
| 23 | Township/range/section (new) | township range section lookup, PLSS lookup, legal description to lat long |
| 24 | Acreage from coordinates / map (exists) | acreage calculator map, polygon area in acres, geojson area |
| 25 | Horizontal/vertical curve (exists) | horizontal curve calculator, vertical curve calculator, degree of curve |
| 26 | Cubic yards / stockpile volume (exists) | cubic yards calculator, stockpile volume calculator |
| 27 | Magnetic declination (exists) | magnetic declination calculator, true to magnetic, WMM2025 |
| 28 | H3 / tile / bbox (exists) | h3 viewer, lat lon to tile, xyz tile calculator, bbox finder |

Why geoprims can win these queries:
- **Aviation:** it shows the exact formula next to the rule of thumb and quantifies the error. Most E6B sites show only the rule of thumb and carry ads ([example list](https://www.pilotmall.com/pages/flight-computer)).
- **Geodesy:** it names the frame and epoch. Competing converters silently treat WGS 84 as NAD83 and EGM96 as NAVD 88.
- **Sun and time:** it applies the three legal definitions correctly.
- **Surveying:** it reports closure precision in a form that is ready for audit.

---

## C. Cut, defer or rework

**Legal or licensing risk**
- **`sight-distance-length`.** AASHTO Green Book values are copyrighted. Either have the user enter the design values, or cut it.
- **`angle-of-repose-reference`.** Material angle tables have unclear sources and carry liability if used for slope design. Cut.
- **`easa-subcategory` and `part108-proposed-reference`.** Legal classification drifts quickly. Part 108 is at OIRA and could be final within months. Keep only as dated links. At most, `part108-proposed-reference` should be a docs page, not a tool.
- **`cold-temp-segments` and the cold-temperature airport links.** The design notes that the FAA list expired September 3, 2026. The correction math is fine, but anything touching the airport list should be deferred until there is a refresh process.
- **`terrain-following` export.** GLO-30 is a surface model, not bare earth, so a drone following it can clip trees and towers. Defer the export, or require an explicit acknowledgment and a margin.
- **`koch-estimate`.** A rule-of-thumb chart that people will treat as performance data. Replace it with the density-altitude explainer.

**Padding that risks thin-content SEO penalties**
- **84 generated geodesy conversion pairs, 40 cross-index pairs, and about 10 sensor-specific NDVI forms.** Google's scaled-content-abuse policy targets exactly this pattern. Ship perhaps 15 high-intent aliases (`dms-to-decimal`, `utm-to-latlon`, `mgrs-to-latlon`) with distinct content, and make the rest parameter presets on one page.
- **GEOREF.** Rare outside military use. Defer, or keep only in the parser.
- **A5 (pre-1.0), `is-class-iii`, `base-cell`, `child-position` and `voronoi-spherical`.** A developer niche. Put them behind an "advanced" section and leave them out of the launch pages.
- **`specific-range`, `lowest-usable-fl`, `tsd-solve` duplicates and the 28 airspeed composites.** Keep the common ones: `ias-to-tas` and `mach-to-tas`.
- **`orthographic` and `equidistant-cylindrical` forward/inverse.** Low demand. Fine to keep as engine functions, but don't give them pages.
- **`least-squares-2d`.** Keep it experimental, as specified, but don't market it at launch. Its main role is to feed the ALTA RPP check.

**Deprioritize (not cut):** `audio-feedback` in the web experience. It does nothing to prove the tools' value.

---

## D. Practitioner journeys (chained tools)

1. **VFR preflight.**
   Paste METAR → decode → altimeter setting + OAT → pressure altitude → density altitude → runway wind components and best runway → weight and balance (user's aircraft profile) → interpolate the takeoff distance table from the user's POH data → sunset and civil twilight, with "you'll land 22 min after civil twilight: loggable night, not currency night."

2. **VFR cross-country.**
   FB winds decode → interpolate winds aloft at cruise altitude → magnetic variation (WMM) → heading chain → groundspeed and ETE per leg → fuel per leg with dated reserves → Zulu ETA → top of descent → glide ring along the route.

3. **IFR approach brief.**
   Hold entry sector + wind-corrected outbound timing → DME arc lead radial → descent rate for the glidepath at the planned groundspeed → VDP from HAT → cold-temperature correction of step-down fixes. Every output carries the not-for-navigation notice.

4. **Drone mapping day.**
   Draw polygon → GSD target → altitude for that GSD (checked against the 400 ft AGL limit and structure rule) → overlap preset (Pix4D 75/60; forest 85/70) ([Pix4D](https://support.pix4d.com/hc/en-us/articles/115002471546)) → motion blur and maximum shutter speed → grid + image count + dataset GB → battery swaps via endurance with losses → sun window (elevation >30°, avoiding the hotspot) → Part 107 lighting check → VLOS distance vs the farthest waypoint → export.

5. **GNSS control and OPUS workflow.**
   Paste almanac → DOP and sky plot window → session length (OPUS-RS or OPUS-S) → slant antenna height → vertical height to the antenna reference point → after OPUS: frame and epoch transform (ITRF2020 → NAD83(2011)) → state plane → combined factor using ellipsoid height from the geoid tool → ground coordinates → localization residuals.

6. **Boundary retracement.**
   Paste deed → parse calls, converting varas and chains → plot and check closure (precision 1:N) → rotate to a new basis of bearing → overlay on PLSS aliquot → acres → RPP check against ALTA 2026 after the field adjustment.

7. **Lidar bid.**
   Area → QL target → altitude and speed from sensor PRR and FOV → line count and strip overlap → flight time and batteries → LAZ file size → checkpoint count per ASPRS Edition 2.

8. **Developer.**
   Paste an H3 or S2 index or a bbox → detect it → neighbors and cover → equivalent tiles → GeoJSON area and simplification → export.

---

## E. Correctness traps in the proposed tools

1. **Four different "nights."**
   - §1.1 (logging): civil twilight, geometric −6°.
   - §61.57(b) (currency): sunset + 1 h and sunrise − 1 h.
   - §91.209 (lights): sunset to sunrise.
   - §107.29 (drones): fixed 30 minutes, except Alaska, which uses the Air Almanac.
   A single "night" output is wrong. Also, sunrise and sunset use −0.833° (refraction plus the sun's semidiameter), and should optionally add a dip correction for observer height. Near and above ±66.5° there may be no sunrise, sunset or twilight at all. Return explicit polar day or night states, never NaN ([NOAA](https://gml.noaa.gov/grad/solcalc/sunrise.html)).

2. **Date-line and day-boundary handling.** Evening civil twilight in the US can fall after 00:00Z. Always return events tied to a local date with a stated UTC offset. Don't use browser `Intl` time-zone data inside a tool: it varies by browser version and breaks the determinism contract. Ship a versioned tzdb snapshot instead.

3. **UT1 vs UTC and ΔT.** SPA needs ΔT, which is about 69 s now. Using UTC as if it were UT1 is fine for sunrise, but not for solar azimuth used in surveying (a 0.9 s DUT1 error is about 0.004° of azimuth). Document it, and let users enter DUT1.

4. **GPS time.** Converting GPS week and seconds needs leap seconds (18 s) and the 1024-week rollover (the legacy 10-bit week number). A bare week number is ambiguous, so require the full week number or an era.

5. **Aviation text decoding:**
   - **Winds are true, except where they aren't.** METAR, TAF and FB winds are true; ATIS and tower winds are magnetic (your research doc already notes this). The decoder must hand off `reference: true` explicitly.
   - **FB wind encoding:** subtract 50 from the direction when it is 51–86 and add 100 to the speed; handle `9900`. Negative temperatures above FL240 are implied.
   - **METAR edge cases:**
     - `M` means minus in temperatures but "less than" in visibility (`M1/4SM`).
     - `1 1/2SM` has a space inside the visibility group.
     - `P6SM` means "more than."
     - `VRB`, `00000KT` and `G` gusts.
     - `CAVOK` exists outside the US.
     - `Q` vs `A` altimeter groups.
     - `T` group signs: `1` = negative.
     - The `SLP` group omits the leading 9 or 10: pick whichever makes the pressure closest to 1000 hPa.
     - `RMK AO2`, and `$` for maintenance.
     - TAF `FM`/`TEMPO`/`BECMG`/`PROB30` change groups and validity periods that cross midnight.
     - Reject non-US or ambiguous formats with warnings rather than guessing.

6. **Hold entry.**
   - **Sector boundaries:** the boundary lines are 70° from the inbound course, measured on the holding side. Right and left holds mirror each other.
   - **Boundaries on the line:** AIM allows a ±5° flex, so report "on boundary: either entry."
   - **References:** use magnetic course, and apply the wind correction to the heading, not to the course.

7. **DME slant range.** Ground distance = √(DME² − h²), with h in NM (ft ÷ 6,076.12). Return an error when h > DME, which happens directly overhead.

8. **VOR magnetic variation.** VOR radials use the station's published magnetic variation from its commissioning epoch, not today's WMM value. For fix-radial-distance work, the user must supply station magnetic variation. Warn when WMM differs by more than 1°.

9. **VDP and glidepath.** HAT/300 corresponds to about 3.14°; HAT/318 gives exactly 3.0°. VDP is measured from the threshold, not the MAP. Vertical speed = GS (kt) × 101.27 × tan θ; the "×5" rule is 5.3 at 3°.

10. **Deed parsing:**
    - **Quadrant bearings:** `N 45°30' E`. Handle `N 90 E`, which equals `East`, and "due north."
    - **Curve calls:** radius, arc and delta, chord bearing, tangent vs non-tangent, left or right. These are ambiguous: flag them, never guess.
    - **Units:** chains and links (a link is 0.66 ft), rods (16.5 ft), and varas, whose values vary by jurisdiction.
    - **Datum:** US survey ft vs international ft (2 ppm).
    - **Closure:** it is not an accuracy measure for the land, only for the arithmetic and the description.
    - **Area when the deed doesn't close:** compute the area of the misclosed figure and disclose that the closing line was implied. Don't auto-adjust with Bowditch without saying so.

11. **PLSS:**
    - **Not a formula.** Sections are not 1 mile × 1 mile. North and west tiers absorb excess and deficiency, there are correction lines, and there are government lots, fractional sections, and resurveys in which quarter corners are not midpoints.
    - **Use the data.** Aliquot parts must be taken from the section polygon in CadNSDI (public domain; the BLM also publishes PLSS second-division polygons), not built by quartering a square.
    - **Label accuracy.** CadNSDI positions can be off by tens of meters. Show its reliability attributes, and state that the result is not survey-grade.
    - **Parsing order.** Read aliquot parts right to left: "NE¼SW¼" is the NE¼ of the SW¼ ([Township America](https://townshipamerica.com/learn/plss/quarter-sections)).
    - **Name the meridian.** There are more than 30 principal meridians; township numbers repeat across them.

12. **ALTA RPP.** The 2026 definition is the semi-major axis of the relative error ellipse at 95% (a factor of 2.448σ, not 1.96σ). It applies between adjacent boundary corners, not every pair of points. Don't compare a distance misclosure against it.

13. **Localization transforms.** A 4-parameter fit with fewer than three control points has no redundancy, so the residuals mean nothing. Say so. Scale should be reported in ppm, and a scale more than about 100 ppm away from the expected combined factor should raise a warning (wrong units or geoid).

14. **DOP from an almanac.**
    - **Almanac age:** almanacs more than about a week old degrade. Warn based on the week number.
    - **Elevation mask:** it changes DOP heavily; use 10–15° as the default.
    - **Multi-GNSS:** requires separate almanacs, and GLONASS uses a different format and time scale.
    - **Terrain and canopy:** not modeled. Say so, or apply the raster horizon from `terrain` if a DEM is present.

15. **Lidar density.**
    - **Pulses vs points:** the density formula gives pulses/m². Returns (points) are a multiple of that, and USGS quality levels are specified in pulses.
    - **Scan pattern:** Livox-style non-repetitive scanners (DJI L1 and L2) are not uniform across the swath.
    - **Overlap:** strip overlap doubles local density but not the "aggregate nominal" figure, so report both.

16. **VLOS formula.** The EASA ALOS formula uses the characteristic dimension in meters and applies to multirotors; fixed-wing uses a different coefficient (490 × CD + 30 m in the same EASA guidance; confirm before implementing). US Part 107 has no numeric VLOS, so label the output "EASA guidance, not a US rule."

17. **Sun for photogrammetry.**
    - **Terrain slope matters.** A steep north-facing slope can be in shadow while solar elevation is above 30°. Offer a "sun incidence on slope" option using existing `slope` and `aspect`.
    - **Hotspot geometry:** it occurs where the camera look vector is anti-parallel to the sun, which happens at nadir only when the sun is near zenith (low latitudes in summer).

18. **Link budget.** The FSPL constant depends on units (32.44 for km and MHz; 92.45 for km and GHz). EIRP limits are regulatory and vary by country, so keep them as dated data.

19. **Unit and time trivia that sites get wrong:**
    - Decimal hours: 1.3 h is 1:18, not 1:30.
    - Hobbs vs tach time.
    - Day-of-year leap years (GPS day 366).
    - The Julian date epoch starts at noon.

---

## Recommendations

- **Launch set:** add sun and twilight, night currency, Zulu and time tools, the METAR and FB winds decoders, hold entry, the VOR/DME and approach set, the deed plotter, legacy land units, VLOS, the Part 107 lighting window, lidar planning, and the RTK/OPUS/antenna-height helpers. Together that is about 15 new operations, and none needs a data asset except the small leap-second and tzdb snapshots.
- **v1.1:** PLSS lookup as an on-demand, public-domain BLM asset per state.
- **Cut or collapse:** the generated-endpoint padding, which carries the biggest SEO risk.

---

## Sources

**Regulations and FAA/ICAO**
- [eCFR 61.57](https://www.ecfr.gov/current/title-14/chapter-I/subchapter-D/part-61/subpart-A/section-61.57)
- [eCFR 107.29](https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.29)
- [Night aviation regulations (Wikipedia)](https://en.wikipedia.org/wiki/Night_aviation_regulations_in_the_United_States)
- [AIM 5-3-8](https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap5_section_3.html)
- [FAA JO 7900.5E Chg 1](https://www.faa.gov/documentLibrary/media/Order/JO_7900.5E_CHANGE_1.pdf)
- [FAA TFR guide](https://www.faa.gov/sites/faa.gov/files/pilots/safety/notams_tfr/tfrweb.pdf)
- [FAA NASR](https://www.faa.gov/air_traffic/flight_info/aeronav/aero_data/NASR_Subscription/)
- [EASA VLOS guidelines](https://www.easa.europa.eu/en/downloads/139435/en)

**Aviation references and competitors**
- [Gleim FB winds example](https://www.gleim.com/public/pdf/av/private/online/links/faexample6_5.pdf)
- [AeroCorner VDP](https://aerocorner.com/blog/visual-descent-point-vdp/)
- [PilotMall E6B](https://www.pilotmall.com/pages/flight-computer)

**Sun, time and leap seconds**
- [NOAA solar calculation details](https://gml.noaa.gov/grad/solcalc/calcdetails.html)
- [NOAA sunrise algorithm](https://gml.noaa.gov/grad/solcalc/sunrise.html)
- [NREL SPA](https://midcdmz.nrel.gov/spa/)
- [IERS Bulletin C 72](https://datacenter.iers.org/data/html/bulletinc-072.html)
- [timezone-boundary-builder](https://github.com/evansiroky/timezone-boundary-builder)

**Surveying, land records and GNSS**
- [BLM CadNSDI](https://gis.blm.gov/arcgis/rest/services/Cadastral/BLM_Natl_PLSS_CadNSDI/MapServer)
- [data.gov PLSS township](https://catalog.data.gov/dataset/blm-id-cadnsdi-plss-township-hub)
- [EarthPoint](https://www.earthpoint.us/TownshipsSearchByLatLon.aspx)
- [Township America](https://townshipamerica.com/learn/plss/quarter-sections)
- [MeteMap](https://metemap.com/)
- [SlateTablet](https://www.slatetablet.com/tools/deed-plotter)
- [ALTA/NSPS 2026](https://cdn.ymaws.com/nsps.us.com/resource/resmgr/alta_standards/2026_OFFICIAL_FINAL_PDF_ALTA.pdf)
- [American Surveyor on ALTA 2026](https://amerisurv.com/2026/02/01/the-2026-minimum-standard-detail-requirements-for-alta-nsps-land-title-surveys/)
- [NGS OPUS](https://geodesy.noaa.gov/OPUS/about.jsp)
- [NGS NCAT](https://www.ngs.noaa.gov/NCAT/)
- [Trimble GNSS Planning](https://www.gnssplanning.com/)

**Drone mapping and lidar**
- [USGS Lidar Base Specification tables](https://www.usgs.gov/ngp-standards-and-specifications/lidar-base-specification-tables)
- [Pix4D image acquisition](https://support.pix4d.com/hc/en-us/articles/115002471546)
- [GeoCue sun angle](https://support.geocue.com/flight-planning-sun-angle/)
- [Drones Made Easy hotspots](https://support.dronesmadeeasy.com/hc/en-us/articles/360060320672-Sun-Hotspots)

**Spec files reviewed:** `/Users/user/Documents/development/public/geoprims/.claude/worktrees/geospatial-aviation-spec-b79149/openspec/changes/{add-aviation-suite,add-drone-suite,add-geodesy-suite,add-navigation-and-geometry,add-spatial-indexing-and-raster,add-survey-suite}/design.md`, the matching `specs/**/spec.md` files, and `establish-platform-foundation/specs/platform/{tool-contract,data-assets}/spec.md`.
