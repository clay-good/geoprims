# Research: aviation, drone, surveying, and remote-sensing math

> Research brief gathered 2026-09-18 to inform the OpenSpec changes. Items marked [memory] still need checking against a primary source; [computed] values were derived from the stated base constants.


**How confident each part is:**
- **Verified this session:** Part 108 status, the ASPRS Edition 2 tables and checkpoint rules, the NSRS/survey-foot timeline, EASA classes and the end of national standard scenarios, the FAA cold-temperature procedure, the corrected ICAO cold-temperature equation, and Pix4D's overlap guidance.
- **Constants I computed myself:** marked **[computed]**, from the stated base constants.
- **From memory, not re-fetched:** marked **[memory]**. Check these against the primary document before shipping.

---

## 1. ICAO Standard Atmosphere (Doc 7488/3, ISO 2533:1975)

**Base constants (exact by definition unless noted):**

| Symbol | Value |
|---|---|
| P0 | 101325 Pa (1013.25 hPa, 29.92126 inHg) |
| T0 | 288.15 K (15 °C) |
| ρ0 | 1.225 kg/m³ (P0/(R·T0) = 1.2250000 **[computed]**) |
| g0 | 9.80665 m/s² |
| R (specific gas constant) | 287.05287 J/(kg·K). Comes from R* = 8.31432 J/(mol·K) and M = 0.0289644 kg/mol |
| γ | 1.4 |
| Earth radius for geopotential | r0 = 6,356,766 m |
| Troposphere lapse rate | −0.0065 K/m |
| a0 (speed of sound at sea level) | 340.294 m/s = 661.4786 kt **[computed]** |

**Layers.** Base altitudes are geopotential H. Geometric altitude is z = r0·H/(r0 − H), and H = r0·z/(r0 + z).

| H base (km) | z base (km) | Lapse (K/km) | T base (K) | P base (Pa) |
|---|---|---|---|---|
| 0 | 0 | −6.5 | 288.15 | 101325 |
| 11 | 11.019 | 0.0 | 216.65 | 22632.06 |
| 20 | 20.063 | +1.0 | 216.65 | 5474.89 |
| 32 | 32.162 | +2.8 | 228.65 | 868.02 |
| 47 | 47.350 | 0.0 | 270.65 | 110.91 |
| 51 | 51.413 | −2.8 | 270.65 | 66.94 |
| 71 | 71.802 | −2.0 | 214.65 | 3.956 |
| Top of ICAO table: 80 km geopotential (≈81.0 km geometric) | | | | |

Source: [Wikipedia ISA summary](https://en.wikipedia.org/wiki/International_Standard_Atmosphere) and [ICAO Doc 7488 store page](https://store.icao.int/en/manual-of-the-icao-standard-atmosphere-extended-to-80-kilometres-262500-feet-doc-7488).

**Pressure within a layer:**
- Layer with a gradient (L ≠ 0): P = Pb·[Tb/(Tb + L·(H − Hb))]^(g0/(R·L)). At L = −0.0065, g0/(R·L) = 5.255880 **[computed]**.
- Isothermal layer: P = Pb·exp[−g0·(H − Hb)/(R·Tb)].

**US Standard Atmosphere 1976:**
- Identical to ISA up to 32 km, and in practice identical through 71 km.
- It continues the −2.0 K/km layer to H = 84.852 km (86 km geometric), then uses a geometric-altitude model with changing molecular weight up to 1000 km.
- ICAO stops at 80 km. So the two standards differ only in how far they extend and in the upper-atmosphere physics.
- Implementation note: build every layer on geopotential altitude. Mixing geometric and geopotential altitude gives about 19 m of error at 11 km.

## 2. Airspeed

**IAS → CAS:**
- CAS = IAS + position error + instrument error. There is no general formula.
- It needs the POH/AFM calibration table, interpolated. The tool must accept a user-supplied table and must not ship a default.

**Impact pressure from CAS.** Uses sea-level constants, where Vc is CAS:
- Subsonic (Vc ≤ a0): qc = P0·[(1 + 0.2·(Vc/a0)²)^3.5 − 1]
- Inverse: Vc = a0·√(5·[(qc/P0 + 1)^(2/7) − 1])
- Supersonic (Rayleigh pitot formula): qc = P0·[166.9215801·(Vc/a0)^7 / (7·(Vc/a0)² − 1)^2.5 − 1]

**Mach from impact pressure.** Use static pressure p, not P0:
- Subsonic: M = √(5·[(qc/p + 1)^(2/7) − 1])
- Supersonic: qc/p + 1 = 166.9215801·M^7/(7M² − 1)^2.5. Solve by iteration, e.g. M(n+1) = 0.881285·√((qc/p + 1)·(1 − 1/(7·M(n)²))^2.5).

**Other speeds:**
- EAS = a0·M·√(p/P0) = TAS·√(ρ/ρ0).
- TAS = M·a, where a = √(γ·R·T) = 38.967853·√T(K) kt **[computed]**.
- Correct CAS→TAS path: CAS → qc → M (using p at pressure altitude) → TAS using the actual outside air temperature. The ISA temperature must not be used here.

**The compressibility correction (CAS − EAS)** is always ≥ 0. It is negligible below about 200 kt / 10,000 ft. Any rule-of-thumb "TAS = IAS + 2% per 1000 ft" should be labeled as an approximation.

**Temperature probe trap:** Indicated OAT at speed reads warm by ram rise. SAT = TAT/(1 + 0.2·r·M²), where the recovery factor r is about 0.95–1.0 for the probe.

## 3. Altimetry

**Pressure altitude:**
- Exact (troposphere): PA_ft = 145442.16·[1 − (P/1013.25 hPa)^0.190263] **[computed from ISA]**.
- NOAA/NWS publishes 145366.45 and 0.190284, which come from slightly different constants. Pick one set and document it.
- Rule of thumb: PA = field elevation + (29.92 − altimeter setting in inHg)·1000 ft. The hPa version uses about 27–30 ft per hPa, but the exact value varies with altitude.
- 1 inHg = 3386.389 Pa (the standard conventional value).

**Altimeter setting ↔ station pressure:** QNH is a reduction that assumes the ISA lapse rate. The NWS/Smithsonian form subtracts 0.3 hPa: A = (p − 0.3)·[1 + (P0^n·L/T0)·H/(p − 0.3)^n]^(1/n), with n = 0.190284 **[memory]**.

**Density altitude:**
- Exact (dry air): σ = (P/P0)/(T/T0), then DA_ft = 145442.16·(1 − σ^0.234969) **[computed]**.
- Rule of thumb: DA ≈ PA + 118.8·(OAT − ISA temp). The exact sensitivity at sea level is 118.3 ft/°C **[computed]**, so "120 ft/°C" is fine as an approximation.
- Humidity: virtual temperature Tv = T/(1 − 0.378·e/p) raises DA. It is usually ignored, so say so on the page.
- ISA temperature at PA: 15 − 1.98·(PA/1000) °C, capped at −56.5 °C above 36,089 ft.

**Q-codes:**
- QNH: reads field elevation on the ground.
- QFE: reads 0 on the ground.
- QNE: altimeter at 29.92 inHg / 1013.25 hPa gives pressure altitude. US transition altitude is 18,000 ft MSL.

**Cold-temperature correction.** Rule: "high to low, look out below."
- ICAO rule of thumb: add 4% of height above the altimeter-setting source for every 10 °C below ISA.
- The accurate equation, per Transport Canada [AC 500-020 §4.8](https://tc.canada.ca/en/aviation/reference-centre/advisory-circulars/advisory-circular-ac-no-500-020), based on ICAO Doc 8168 Vol II, 7th ed. (2020):
  Correction = (Δt_std/L0)·ln[1 − L0·Δh_aircraft/(273.15 + t_std − L0·h_aerodrome)]
  - L0 = 0.0019812 °C/ft
  - Δt_std = aerodrome temperature − ISA temperature at the aerodrome
  - Heights are above aerodrome and are pressure heights.
- **Trap:** TC says Doc 8168 Vol III (2018) had an error in this equation. Don't copy that version.
- The ICAO table (AIM TBL 7-3-1) assumes a sea-level aerodrome, so it gives conservative results. The equation is more accurate.
- FAA Cold Temperature Airports (CTA, AIM 7-3-4):
  - Marked with a snowflake icon. Airport-specific temperature limits.
  - Corrections by the "All Segments" or "Individual Segments" method.
  - Pilots report corrected altitudes to ATC on every segment except final.
  - The current list is valid [Oct 2, 2025 – Sep 3, 2026](https://aeronav.faa.gov/d-tpp/Cold_Temp_Airports.pdf). That window closed two weeks ago, so link to the live d-TPP page rather than hard-coding the list.
  - See the [AIM 7-3](https://www.faa.gov/air_traffic/publications/aim_html/chap7_section_3.html) and [NBAA summary](https://nbaa.org/aircraft-operations/safety/in-flight-safety/aircraft-icing/cold-temperature-restricted-airports/).

## 4. Wind triangle, turns and descent planning

**Units:**
- NM = 1852 m, ft = 0.3048 m, statute mile = 1609.344 m (all exact).
- kt = 1852/3600 m/s.
- US gallon = 3.785411784 L.
- Planning fuel weights: avgas 6.0 lb/gal, Jet-A about 6.7 lb/gal (this one varies with temperature).

**Wind triangle.** Wind direction is where the wind blows FROM. Angles in degrees; normalize headings to the range 0–360:
- WCA = asin[(W/TAS)·sin(WD − TC)]. If |W·sin| > TAS, the aircraft cannot hold the course; the tool must return an error.
- TH = TC + WCA
- GS = TAS·cos(WCA) − W·cos(WD − TC)
- Finding wind from TAS, TH, GS and track: subtract the ground vector from the air vector and convert the result to "from" direction/speed.
- Components against a runway: headwind = W·cos(WD − RWY), crosswind = W·sin(WD − RWY). A negative headwind is a tailwind.

**Which winds are true vs magnetic (FAA practice) [memory]:**
- METAR, TAF and winds aloft: true north.
- ATIS and tower-reported winds: magnetic.
- Runway numbers: magnetic bearing ÷ 10, rounded. Some high-latitude runways (northern Canada, some in Alaska) are true, marked "T."

**Turns:**
- Rate: ω = g·tanφ/V.
- Standard rate: 3°/s, a 2-minute 360° turn. Bank ≈ KTAS/10 + 7 (the exact value at 100 kt is 15.36° **[computed]**).
- Radius: r = V²/(g·tanφ). With V in kt and r in ft, r = V²/(11.294·tanφ) **[computed from g0]**. Many texts print 11.26; document which you use.
- Load factor n = 1/cosφ. Stall speed in the turn = Vs·√n.
- 3° descent: 318.4 ft/NM, and descent rate ≈ 5.31·GS fpm **[computed]**. The rule of thumb is ×5.

**Descent and glide:**
- Top-of-descent 3:1 rule: distance (NM) = 3 × (altitude to lose in ft/1000). This is about 3.16° and assumes zero wind.
- Glide distance = height × L/D. Best-glide L/D is in the POH. Adjust for wind by multiplying by GS/TAS.
- Fuel = burn rate × time, plus the required reserve. Reserves (14 CFR 91.151/91.167) are 30 min day VFR, 45 min night VFR, and 45 min IFR **[memory]**.

## 5. Magnetic variation

- Conversion: TC = MC + E variation. MC = TC − E, or MC = TC + W. Mnemonic: "east is least, west is best."
- FAA charts, navaids and runways use a *fixed epoch* declination. VOR radials use the station's "assigned variation," which can be degrees out of date.
- The current model is WMM2025, valid 2025–2029, released December 2024 **[memory]**.
- **The tool should show both values:** the model declination on today's date, and the note that published charts and procedures may use an older epoch value.

## 6. Drones

**FAA Part 107** ([eCFR 107.51](https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107)) **[memory for details]:**

| Limit | Value |
|---|---|
| Max groundspeed | 87 kt (100 mph) |
| Max altitude | 400 ft AGL, or within a 400 ft radius of a structure and no more than 400 ft above its top |
| Visibility | ≥ 3 SM |
| Cloud clearance | 500 ft below, 2000 ft horizontal |
| Line of sight | VLOS required |
| Max weight | < 55 lb |

- Night operations have been allowed since April 21, 2021, with anti-collision lighting visible for 3 SM and updated training. Operations over people fall into Categories 1–4.
- **Remote ID (Part 89):** fully enforced since March 16, 2024, when discretionary enforcement ended ([FAA](https://www.faa.gov/newsroom/faa-ends-discretionary-enforcement-policy-drone-remote-identification)). Three ways to comply: Standard Remote ID, a broadcast module, or flying inside a FRIA.

**Part 108 (BVLOS) is NOT final as of September 18, 2026:**
- The NPRM was published August 7, 2025. Comments closed October 6, 2025.
- The comment period was reopened January 28 to February 11, 2026, limited to ADS-B Out, electronic conspicuity and detect-and-avoid ([Federal Register](https://www.federalregister.gov/documents/2026/02/10/2026-02649/normalizing-unmanned-aircraft-systems-beyond-visual-line-of-sight-operations-reopening-of-comment); [Flying](https://www.flyingmag.com/faa-reopens-part-108-drone-comments/)).
- A September 14, 2026 report says the rule is at OIRA, and the FAA hopes to publish by the end of 2026 ([The Flight Brief](https://www.theflightbrief.com/articles/faa-part-108-bvlos-update-september-2026)).
- One blog (uavhq.com) has a headline claiming "FAA Finalizes Part 108." It conflicts with the other sources. Treat it as wrong unless the Federal Register shows otherwise.
- NPRM contents: aircraft up to 1,320 lb, permits vs operating certificates, population-density risk categories, and new Operations Supervisor / Flight Coordinator roles.
- Spec implication: any Part 108 tool must be labeled "proposed" and point to the Federal Register.

**EASA:**
- Open category: MTOM < 25 kg, 120 m height limit.

| Subcategory | Classes | Rule |
|---|---|---|
| A1 | C0 (< 250 g), C1 (< 900 g or < 80 J, ≤ 19 m/s) | May fly over uninvolved people, not over crowds |
| A2 | C2 (< 4 kg) | 30 m from uninvolved people, or 5 m in low-speed mode |
| A3 | C2, C3, C4 (< 25 kg; C4 has no automation) | 150 m from residential, commercial or industrial areas |

- C5 and C6 were added by Regulation (EU) 2020/1058 for the Specific category's standard scenarios (STS). C5 is for STS-01 (VLOS). C6 is for STS-02 (BVLOS).
- Class labels have been mandatory for new drones since January 1, 2024 ([EASA](https://www.easa.europa.eu/en/newsroom-and-events/news/reminder-drone-identification-labels-mandatory-beginning-2024)).
- Legacy (unlabeled) drones: under 250 g may fly in A1; 250 g to 25 kg are limited to A3.
- **National standard scenarios expired December 31, 2025.** STS operations now require C5/C6 drones ([PrimeCor](https://primecorsys.com/en/c5-c6-class-marking-mandatory-drones-2026/); a vendor source, so confirm against EASA).
- Direct Remote ID is required on C1, C2, C3, C5 and C6.

**Photogrammetry (Sw = sensor width in mm, f = focal length in mm, H = height AGL, Iw = image width in px):**
- GSD = Sw·H/(f·Iw). Use the sensor's physical width, not the 35 mm-equivalent focal length.
- Footprint across track = GSD·Iw. Footprint along track = GSD·Ih, which depends on camera orientation.
- Trigger distance = footprint along track × (1 − front overlap). Trigger interval = distance/groundspeed.
- Line spacing = footprint across track × (1 − side overlap).
- Motion blur in pixels = V·t_shutter/GSD. Keep it ≤ 0.5–1 px, so t ≤ k·GSD/V.
- Overlap (Pix4D): at least 75% front / 60% side in general; at least 85% / 70% for forest and dense vegetation ([Pix4D](https://support.pix4d.com/hc/en-us/articles/115002471546)).
- Overlap is lower on high terrain because H AGL drops. Use H above the highest terrain for the worst case.

**ASPRS Positional Accuracy Standards:**
- Edition 2, v1 was approved August 23, 2023. **v2 was adopted June 24, 2024** ([Geo Week](https://www.geoweeknews.com/articles/asprs-edition-2-version-2-positional-accuracy-standards-photogrammetry-remote-sensing-lidar/)).
- v2 added addenda for lidar, photogrammetry, UAS and oblique imagery.
- Details below were read from the Edition 2 v1 main body ([PDF](https://aagsmo.org/wp-content/uploads/2023/03/ASPRS_PosAcc_Edition2_MainBody.pdf)). The core formulas and tables were not reported as changed in v2.

| Item | Rule |
|---|---|
| Accuracy measure | RMSE only. The 95% confidence figures are gone |
| Classes | Open-ended "X-cm" classes, all metric |
| Horizontal | RMSEH = √(RMSEx² + RMSEy²) ≤ X. Ortho seamline mismatch ≤ 2X |
| Vertical, non-vegetated (NVA) | RMSEV ≤ X; this is the pass/fail test |
| Vertical, vegetated (VVA) | Reported as found; no pass/fail |
| Lidar internal precision | Within-swath ≤ 0.60X; swath-to-swath RMSDz ≤ 0.80X; swath-to-swath max difference ≤ 1.60X |
| 3D | RMSE3D = √(RMSEH² + RMSEV²) |
| Product accuracy including checkpoint error | √(RMSE_fit² + RMSE_survey²). Example: 1.00 cm fit with 2.0 cm survey error gives 2.24 cm |
| Checkpoint accuracy | At least 2× better than the product (was 3×) |
| Minimum checkpoints | 30 (was 20) |
| Blunder threshold | Error > 3× target RMSE |
| Mean error | Should be < 25% of target RMSE |
| Vertical checkpoint sites | Slope ≤ 10% |
| GSD | Explicitly not used to express accuracy |

**Battery endurance model:**
- Ideal hover power per rotor disk: P = T^1.5/√(2ρA), where T is thrust in N (= m·g per total disk area) and A is disk area.
- Actual power = P_ideal/(FM·η_motor·η_ESC). Figure of merit FM is typically 0.5–0.7.
- Endurance (min) = 60·E_Wh·DoD_usable·(1 − reserve)/P_total.
- Peukert: t = H·(C/(I·H))^k. For LiPo, k is about 1.02–1.1 **[memory; weak model for LiPo]**.
- Cold derating: about 10–30% at 0 °C or below. Heuristic, so flag it as such.
- ρ comes from the density-altitude calculation in section 3, which is how the tools connect.

## 7. Surveying

**Traverse:**
- Linear misclosure = √(ΣLat² + ΣDep²). Precision = 1 : (perimeter/misclosure).
- Interior angles of a closed polygon should sum to (n − 2)·180°.
- Bowditch (compass) rule: correction_i = −ΣLat·(L_i/ΣL); same for departures.
- Transit rule: correction_i = −ΣLat·|Lat_i|/Σ|Lat|. Transit is for when angles are better than distances, and it depends on orientation.

**Distances and elevations:**
- Slope to horizontal with zenith angle Z: HD = SD·sin Z, VD = SD·cos Z, and ΔElev = HI + VD − HR.
- Curvature and refraction: h = (1 − k)·K²/(2R).
  - With R = 6371 km and k = 0.14: 0.0675·K² m (K in km). With k = 0.13: 0.0683·K² m **[computed]**.
  - In feet: 0.0206·F² (F in thousands of ft), or 0.574·M² (M in miles).
  - **Trap:** "0.0675" goes with k = 0.14, not 0.13.

**Earthwork:**
- Average end area: V = L·(A1 + A2)/2. Divide by 27 for yd³ from ft.
- Prismoidal: V = L/6·(A1 + 4Am + A2). Am is the *measured* middle area, not the mean of the ends.

**Grid ↔ ground:**
- Combined factor = k(grid scale) × EF, where EF = R/(R + h).
- **h must be ellipsoid height (H + N), not orthometric height.**
- Grid distance = ground distance × CF.

**US survey foot:**
- 1 US survey ft = 1200/3937 m. It is 2 ppm longer than the international foot (0.3048 m).
- Deprecated since January 1, 2023 ([Federal Register](https://www.federalregister.gov/documents/2020/10/05/2020-21902/deprecation-of-the-united-states-us-survey-foot)). It is kept permanently for SPCS 83 and SPCS 27 legacy use.
- NGS count: **40 states** specify US survey ft for SPCS 83 (28 by statute, 12 by Federal Register notice). 6 states use the international foot. 2 states, plus Puerto Rico and Guam, specify neither ([NGS policy](https://www.ngs.noaa.gov/INFO/Policy/NAD83FEETPOLICY2.html), [NIST FAQ](https://www.nist.gov/pml/us-surveyfoot/frequently-asked-questions-faqs)).
- I believe the international-foot states are AZ, MI, MT, ND, OR and SC **[memory; confirm in NGS SP NOS NGS 13, Appendix C]**.
- SPCS2022 will be international-foot only. Official NSRS adoption is now expected in late 2026 or 2027 ([GPS World, March 2026](https://www.gpsworld.com/ngs-presents-the-latest-nsrs-news-at-geo-week-2026/)).
- Implication: any SPCS 83 tool needs a per-zone foot choice.

## 8. Horizon and line of sight

Exact geometric horizon: d = √(2Rh + h²). With an effective radius Re = R/(1 − k), where R = 6371 km **[computed]**:

| Model | km per √(h in m) | NM per √(h in ft) |
|---|---|---|
| Geometric | 3.570 | 1.064 |
| Optical, k = 0.13 | 3.827 | 1.141 |
| Radio, 4/3 Earth | 4.122 | 1.229 |

- Line of sight between two heights: d = c·(√h1 + √h2).
- Many sources quote 1.17√h NM for the visual horizon; k = 0.13–0.14 gives about 1.14–1.15. Document which you use.

## 9. Spectral indices

Inputs are surface reflectance on a 0–1 scale.

| Index | Formula |
|---|---|
| NDVI | (NIR − Red)/(NIR + Red) |
| NDWI (McFeeters, water) | (Green − NIR)/(Green + NIR) |
| NDWI (Gao, leaf water) | (NIR − SWIR1)/(NIR + SWIR1) |
| MNDWI (Xu) | (Green − SWIR1)/(Green + SWIR1) |
| EVI | 2.5·(NIR − Red)/(NIR + 6·Red − 7.5·Blue + 1) |
| EVI2 | 2.5·(NIR − Red)/(NIR + 2.4·Red + 1) |
| SAVI | (1 + L)·(NIR − Red)/(NIR + Red + L), with L = 0.5 |

**Band numbers and scaling [memory]:**
- Sentinel-2: B2 blue, B3 green, B4 red, B8 NIR (B8A narrow NIR), B11 SWIR1. L2A reflectance = (DN − 1000)/10000 since processing baseline 04.00 (January 2022).
- Landsat 8/9: B2 blue, B3 green, B4 red, B5 NIR, B6 SWIR1. Collection 2 surface reflectance = DN·0.0000275 − 0.2.

## 10. Disclaimers

- Typical E6B app wording: for informational purposes only; do not use as the sole tool for flight planning or navigation; always use official sources ([App Store example](https://apps.apple.com/app/id6742122583)).
- Apps also note that phone apps can't be used during FAA knowledge tests.
- Recommended site-wide text:
  - "Not for navigation or operational use. Educational estimates only. Verify against the POH/AFM, official charts, NOTAMs and current regulations. The pilot in command / licensed surveyor is responsible."
  - Show each tool's standard/model version and "rules as of" date.
  - Mark Part 108 as "proposed."

---

## Precision and validation traps

1. **Altitude types:** mixing geometric and geopotential altitude in ISA layers.
2. **Pressure-altitude constants:** 145442/0.190263 vs 145366/0.190284; pick one and document it.
3. **Mach:** using P0 instead of static p. Subsonic formulas above M = 1 need Rayleigh.
4. **TAS:** computing it from the ISA temperature instead of the actual OAT; not correcting indicated OAT for ram rise.
5. **Rule-of-thumb labels:** 120 ft/°C DA, 2%/1000 ft TAS, the 4% cold correction and 3:1 descent are all approximations.
6. **Cold-temperature equation:** copying the flawed 2018 Doc 8168 Vol III version. Hard-coding the CTA list, which expires every year.
7. **Wind reference:** true METAR winds vs magnetic ATIS winds; wind direction is "from," not "to"; WCA fails when crosswind > TAS; angles wrapping at 0/360.
8. **Turn-radius constant:** 11.26 vs 11.294.
9. **Declination:** model declination today vs the epoch value on charts and runways.
10. **GSD:** using 35 mm-equivalent focal length or the wrong sensor dimension; portrait vs landscape; height above takeoff vs above terrain.
11. **ASPRS:** still reporting 95% confidence values; using 20 checkpoints instead of 30; ignoring checkpoint error; treating VVA as pass/fail.
12. **Curvature and refraction:** a 0.0675 coefficient paired with k = 0.13.
13. **Elevation factor:** using orthometric height instead of ellipsoid height.
14. **Survey foot:** mixing US and international feet. The 2 ppm difference is about 0.6 m at 1,000,000 ft northings.
15. **Earthwork:** prismoidal Am taken as the average of the end areas; forgetting ÷27 for yd³.
16. **Spectral indices:** running them on raw DN or TOA values without scale/offset; the S2 −1000 offset; the two NDWI definitions; the EVI 7.5/6/1 constants assume 0–1 reflectance.
17. **Horizon constants:** 3.57 vs 4.12 (and 1.06/1.17/1.23 in NM); document which Earth radius and k.
18. **Rounding:** don't round inside the calculation chain; round only for display. Say whether each unit is exact (1852 m, 0.3048 m, 1200/3937 m) or conventional (3386.389 Pa/inHg).
19. **Hover power:** using the ideal value without FM and efficiency losses overstates endurance by 30–50%.
20. **Part 108:** presenting it as final.
