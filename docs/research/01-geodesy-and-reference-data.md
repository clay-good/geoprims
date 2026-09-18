# Research: geodesy libraries, datums, and reference data

> Research brief gathered 2026-09-18 to inform the OpenSpec changes. Items marked [unverified] still need checking against a primary source.


Everything below was checked on the web today unless it is marked **[unverified]**. Three things matter most for the spec:

- **The new US datums are still in beta.** Official adoption of NATRF2022, NAPGD2022, GEOID2022 and SPCS2022 is expected in late 2026 to early 2027.
- **The magnetic models are current.** WMM2025 was released December 17, 2024, and WMMHR2025 is its high-resolution companion.
- **PROJ runs in the browser but has no npm package.** Browser builds exist and PROJ 9.8 added a browser-friendly way to fetch grids over the network.

---

## 1. GeographicLib (Karney)

**Accuracy**
- Karney's 2013 geodesic algorithms solve both the direct and inverse problems. The inverse converges for every pair of points, including nearly antipodal ones. Round-off error is about 15 nm (nanometers) on WGS84. https://arxiv.org/abs/1109.4448
- **Vincenty's formulae** are accurate to about 0.5 mm on the Earth ellipsoid. His inverse iteration can fail to converge for nearly antipodal points. Karney's inverse has no such failure. https://en.wikipedia.org/wiki/Vincenty's_formulae
- GeographicLib's series solution is "2–3 times faster and 2–3 times more accurate" than solving with elliptic integrals. `GeodesicExact` (the elliptic-integral version) is for ellipsoids with flattening above 0.02. https://geographiclib.sourceforge.io/C++/doc/geodesic.html
- **Polygon area** on the ellipsoid is computed in closed form from an area integral (the `PolygonArea` class, same Karney 2013 paper).
- **Rhumb lines:** the C++ library has a `Rhumb` class. The fetched geodesic page does not cover it. **[unverified]** I could not confirm that the JS npm package includes Rhumb; its documented scope is Geodesic, GeodesicLine, PolygonArea and DMS. Check before relying on it.

**Test data (GeodTest.dat)**
- 500,000 test cases:
  - 100,000 random
  - 50,000 nearly antipodal
  - 50,000 short-distance
  - pole, meridional, equatorial and vertex cases
- About 39 MB compressed. A short version is about 690 KB. It is on SourceForge under `testdata/`. https://geographiclib.sourceforge.io/C++/doc/geodesic.html

**Ports and licenses**

| Package | Version | License | Notes |
|---|---|---|---|
| C++ GeographicLib | 2.7 (current docs) | MIT | Full library |
| npm `geographiclib-geodesic` | 2.2.0 | MIT | Unpacked size about 294 KB |
| Rust `geographiclib-rs` | 0.2.7 (updated Feb 17, 2026) | MIT | **[unverified]** coverage of PolygonArea |

- npm: https://registry.npmjs.org/geographiclib-geodesic/latest
- Rust: https://crates.io/crates/geographiclib-rs

## 2. PROJ and proj4js

**PROJ releases**

| Version | Date | EPSG database |
|---|---|---|
| 9.9.0 | Sep 15, 2026 | EPSG v13.102 |
| 9.8.1 | Apr 10, 2026 | Reverted to EPSG v12.029 over compatibility issues with national ETRS89 realizations |
| 9.8.0 | Mar 2, 2026 | Added "Use Emscripten fetch in networkfilemanager" (#4627) |

- The PROJ 9.8.0 change lets a browser (WebAssembly) build fetch grids over the network.
- Nothing in the recent release notes mentions NATRF2022 or SPCS2022.
- License: X/MIT.
- Source: https://proj.org/en/stable/news.html

**PROJ in WebAssembly**
- Upstream issue #4327 (opened November 2024) proposes an official browser build so PROJ and proj4js stop disagreeing. https://github.com/OSGeo/PROJ/issues/4327
- **`jjimenezshaw/wasm-proj`** is the working option today. https://github.com/jjimenezshaw/wasm-proj
  - MIT license. It bundles PROJ 9.8.0 plus libtiff, sqlite3 and zlib.
  - Its build files come from PROJ's own automated builds.
  - It downloads only the needed part of each grid from cdn.proj.org.
  - It uses synchronous network requests, so it must run in a Web Worker.
  - There is **no npm package yet**.
  - Demo: https://jjimenezshaw.github.io/wasm-proj/
- **[unverified]** The size of `proj.db` (PROJ's SQLite database of coordinate systems and transformations) is about 9–10 MB uncompressed in PROJ 9.x. That is from memory; no source confirmed it. The .wasm file size is not documented. Measure both from a real build.

**Grid files (PROJ-data and cdn.proj.org)**
- About 806 MB in total, stored as Cloud-Optimized GeoTIFF files, organized by agency folder (e.g. `us_noaa`). https://cdn.proj.org/
- Allowed licenses: public domain, X/MIT, BSD, CC0, CC-BY 3.0+, CC-BY-SA 3.0+. Every file has a README stating its own license. https://github.com/OSGeo/PROJ-data
- The CDN deletes access logs after 24 hours. That matters because grid requests reveal roughly where a user is.

**proj4js**
- Handles NTv2 (`.gsb`) and GeoTIFF grids through `proj4.nadgrid()`; the GeoTIFF path needs geotiff.js.
- Reads WKT2 and PROJJSON definitions, and can pick up a simplified datum shift embedded in a CRS definition.
- Built in: EPSG:4326, EPSG:3857, the UTM zones EPSG:326xx and 327xx, and the polar systems EPSG:5041 and 5042.
- It has no EPSG database, no pipeline or time-dependent (epoch-based) transforms, and no automatic choice of transformation. **[inferred]** Its results can differ from PROJ's.
- https://github.com/proj4js/proj4js

## 3. Datums and the US datum modernization

**WGS84**
- **Current realization: WGS 84 (G2296).** It replaced G2139 on Jan 7, 2024, and was fully in place (updated monitor stations) on Mar 4, 2024.
- It is aligned to ITRF2020 and IGS20, within about 3 cm historically. EPSG codes: datum 1383, CRS 10604 and 10606.
- https://earth-info.nga.mil/php/download.php?file=WGS+84%28G2296%29.pdf and https://epsg.io/1383-datum

**NAD83(2011)**
- Published coordinates stay at epoch 2010.00. NGS's latest multi-year CORS solution (MYCS3) is aligned to ITRF2020.
- EPSG:10334 is the ITRF2020 to NAD83(2011) transformation.
- https://geodesy.noaa.gov/web/news/pdf/NGS-product-enhancements-updates-alignment-ITRF2020.pdf

**HTDP (NGS time-dependent frame transformation tool)**
- Version 3.6.0 (user guide dated April 7, 2025). It adds WGS 84 (G2296) = ITRF2020/IGS20.
- https://github.com/noaa-ngs/HTDP/releases

**NADCON5 (NGS datum grids for NAD27, NAD83 and others)**
- PROJ supports NADCON5 grids and the method (PR #3510), and NCAT (NGS's online conversion tool) uses them.
- https://github.com/OSGeo/PROJ/pull/3510

**Status of the new US datums (NATRF2022, PATRF2022, CATRF2022, MATRF2022, NAPGD2022)**
- As of February 2026, beta products were being released in stages on beta.ngs.noaa.gov.
- Testing continues for at least 6 months after the last component appears.
- Adoption steps: a vote by the Federal Geodetic Control Subcommittee (FGCS), expected late 2026 or early 2027, then Federal Geographic Data Committee (FGDC) endorsement, then a Federal Register notice.
- **Official adoption is expected in early 2027. Today, none of these datums are official.**
- The first reference epoch is 2020.00, and coordinates depend on epoch.
- Sources:
  - https://www.gpsworld.com/ngs-presents-the-latest-nsrs-news-at-geo-week-2026/
  - https://geodesy.noaa.gov/datums/newdatums/FAQNewDatums.shtml
  - https://www.federalregister.gov/documents/2024/10/09/2024-23347/updated-implementation-timeline-for-the-modernized-national-spatial-reference-system-nsrs

**What replaces GEOID18: GEOID2022**
- GEOID2022 is a purely gravimetric geoid and defines zero height for NAPGD2022, which replaces NAVD 88 (the current US vertical datum).
- GEOID18 was the last "hybrid" geoid (one fitted to leveled benchmarks).
- Built jointly by the US, Canada and Mexico agencies (NGS, CGS, INEGI). 1 arc-minute resolution.
- Regions: North America–Pacific, Guam and the Northern Mariana Islands, American Samoa.
- **[unverified]** File sizes and whether the model includes a time-varying component.
- https://www.oicrf.org/-/geoid18-last-u.s.-hybrid-geoid-prior-to-napgd2022

## 4. State Plane Coordinate System

**SPCS83**
- **124 zones.** https://www.usgs.gov/faqs/what-state-plane-coordinate-system-can-gps-provide-coordinates-these-values
- Defining document: NOAA Manual NOS NGS 5. https://www.ngs.noaa.gov/library/pdfs/NOAA_Manual_NOS_NGS_0005.pdf

**SPCS2022**
- Status: beta. The zone definitions are described as finalized and are being registered with EPSG and the ISO Geodetic Registry.
- Zone Information page last updated May 14, 2026. https://beta.ngs.noaa.gov/SPCS/zone-information.html
- **Units:** meters and the international foot only. The US survey foot is kept only for SPCS83 and SPCS27. https://beta.ngs.noaa.gov/SPCS/learn-more.html
- **Layers:** up to three per state:
  - a single statewide zone (54 in total)
  - a multi-zone layer (traditional zones or low-distortion projections)
  - special-use zones, some spanning several states
- **Zone count is unsettled:**

| Figure | Source | Confidence |
|---|---|---|
| 953 zones, "as of April 2026" | A search-engine summary (pages behind it could not be fetched) | Low |
| 159 NGS-designed zones plus 810 stakeholder-proposed zones | Older (2021) Wisconsin page | Older plan |

  - Get the authoritative count from https://beta.ngs.noaa.gov/SPCS/zone-definitions.shtml. It returned a 503 error today.
- Policy background: NOAA Special Publication NOS NGS 13 (2018), https://geodesy.noaa.gov/library/pdfs/SP_NOS_NGS_13.pdf

## 5. UTM, UPS, MGRS and USNG

**Standards**
- NGA.STND.0037_2.0.0_GRIDS (2014), "Universal Grids". http://www.legallandconverter.com/files/NGA.STND.0037_2.0.0_GRIDS.pdf (this mirror is not NGA's own site)
- NGA.SIG.0012_2.0.0_UTMUPS (2014), "The Universal Grids and the Transverse Mercator and Polar Stereographic Map Projections".
- NGA index page: https://earth-info.nga.mil/index.php?dir=coordsys&action=coordsys

**Rules** (standard; not re-fetched today)
- UTM covers 80°S to 84°N, in 60 zones 6° wide.
- Latitude bands C–X are 8° tall, except band X at 12°. I and O are not used.
- **Norway:** zone 32V is widened to 3°E–12°E, and 31V is narrowed to 0°–3°E.
- **Svalbard (band X, 72°–84°N):** zones 32X, 34X and 36X do not exist. 31X covers 0–9°E, 33X 9–21°E, 35X 21–33°E, and 37X 33–42°E.
- **UPS (polar stereographic)** is used beyond these limits: bands A/B in the south, Y/Z in the north, with scale factor 0.994.
- MGRS has two 100 km square lettering schemes: "AA" for WGS84/GRS80 and "AL" for older ellipsoids.
- USNG is effectively MGRS on NAD83/WGS84, written with spaces.

**GeographicLib MGRS behavior**
- Precision runs from −1 (grid zone only) to 11 (1 µm). Levels 0–5 give 100 km down to 1 m.
- Coordinates are truncated, not rounded.
- Valid UTM ranges are 100 km tighter than for plain UTM conversion.
- Near a band boundary (within 5 nm), a neighboring band letter is accepted.
- https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1MGRS.html

**Libraries**
- npm `mgrs` 2.2.0 (MIT, from the proj4js project). https://registry.npmjs.org/mgrs/latest
- **[inferred]** Its handling of UPS and polar areas is limited. Test it against GeographicLib or NGA GEOTRANS.

## 6. Geoid models

GeographicLib grid files are 16-bit PGM images. Heights are quantized to 3 mm, the files are uncompressed, and any point can be read directly. https://geographiclib.sourceforge.io/C++/doc/geoid.html

| Grid | Size | Bilinear error, max / RMS | Cubic error, max / RMS |
|---|---|---|---|
| egm84-30 | 0.6 MB | 1.546 m / 70 mm | 0.274 m / 14 mm |
| egm84-15 | 2.1 MB | 0.413 m / 18 mm | 0.021 m / 1.2 mm |
| egm96-15 | 2.1 MB | 1.152 m / 40 mm | 0.169 m / 7.0 mm |
| egm96-5 | 19 MB | 0.140 m / 4.6 mm | 3.2 mm / 0.7 mm |
| egm2008-5 | 19 MB | 0.478 m / 12 mm | 0.294 m / 4.5 mm |
| egm2008-2.5 | 75 MB | 0.135 m / 3.2 mm | 0.031 m / 0.8 mm |
| egm2008-1 | 470 MB | 0.025 m / 0.8 mm | 2.2 mm / 0.7 mm |

- The errors are interpolation error against the full spherical-harmonic model, not the model's accuracy against the real Earth.
- Cubic interpolation uses a 12-point fit and has small jumps at cell boundaries.
- **[unverified, from memory]** Absolute accuracy: EGM2008 is about 5–10 cm in well-surveyed areas; EGM96 is about 0.5–1 m.
- PROJ-data also provides EGM96 and EGM2008 as Cloud-Optimized GeoTIFFs. **[unverified]** The exact PROJ-data file sizes.

## 7. Magnetic models

**WMM2025**
- Released Dec 17, 2024. Valid 2025.0 to 2030.0; NCEI's page says "valid through late 2029".
- Public domain ("not licensed or under copyright").
- `WMM.COF` coefficient file: degree 12, about 5 KB (3 KB in GeographicLib's format).
- https://www.ncei.noaa.gov/products/world-magnetic-model

**Where the compass is unreliable** (https://www.ncei.noaa.gov/products/world-magnetic-model/accuracy-limitations-error-model)
- **Blackout zone:** horizontal field strength H below 2000 nT. Compasses should not be relied on.
- **Caution zone:** H between 2000 and 6000 nT.
- **Declination uncertainty:** √(0.26² + (5417/H)²) degrees, with H in nT.
- Local crustal anomalies of 3–4° are common, and some exceed 10°. The WMM does not model them.

**WMMHR2025 (high resolution)**
- Main field and its change over time to degree 15; crustal field to degree 133. Resolution about 300 km at the equator, versus about 3,300 km for WMM.
- 18,210 coefficients in a 534 KB file (281 KB in GeographicLib's format).
- Uncertainty: H 130 nT, inclination 0.19°.
- NGA recommends WMMHR over WMM for all Department of Defense systems.
- https://www.ncei.noaa.gov/products/world-magnetic-model-high-resolution

**IGRF-14**
- Adopted by IAGA in November 2024. Covers 1900.0 to 2030.0, degree 13, about 29 KB.
- https://link.springer.com/article/10.1186/s40623-025-02360-0 and https://www.ncei.noaa.gov/products/international-geomagnetic-reference-field

**EMM (Enhanced Magnetic Model)**
- The latest is still EMM2017: degree 790, valid 2000–2022 (now expired), 24.5 MB.
- NCEI asks users to fill in an intended-use survey before downloading. **Treat redistribution as unclear.**
- https://www.ncei.noaa.gov/products/enhanced-magnetic-model

**GeographicLib magnetic model list:** https://geographiclib.sourceforge.io/C++/doc/magnetic.html

**Aviation practice**
- The FAA assigns "epoch year" magnetic variation (MV) to navaids and airports; the policy is in Order 8260.19K §2-5. The PDF returned a 403 error.
- **[unverified]** From memory: WMM-based, reviewed on the 5-year epoch cycle, whole degrees, updated when drift passes a tolerance.
- Background: https://www.faa.gov/sites/faa.gov/files/about/office_org/headquarters_offices/avs/130617_PARCMagVarRecommendations.pdf

## 8. Transverse Mercator and Lambert Conformal Conic

- **Karney's Transverse Mercator** (J. Geodesy 85(8), 2011):
  - The 6th-order Krüger series is accurate to under 5 nm within 3900 km of the central meridian.
  - The exact method (Lee's elliptic-function formulas) is accurate to 9 nm over the whole ellipsoid.
  - Grid convergence and scale factor come from the same complex series.
  - GeographicLib lets you choose series order 4 to 8 at compile time.
  - Sources: https://arxiv.org/abs/1002.1417 and https://geographiclib.sourceforge.io/C++/doc/transversemercator.html
- **Lambert Conformal Conic** (standard, from memory; Snyder 1987, EPSG Guidance Note 7-2):
  - Convergence γ = n·(λ − λ₀), where n = sin φ₀ is the cone constant.
  - Scale k = m₀·(t/t₀)ⁿ / m, where m = cos φ/√(1 − e² sin² φ) and t is the isometric-latitude term.

## 9. Maidenhead, degree parsing and Web Mercator limits

These are standard definitions, not re-fetched today.

**Maidenhead locator**
- Pairs of characters, each pair splitting the previous cell:
  - field: A–R, 18×18 (20° × 10°)
  - square: 0–9 digits, 10×10 (2° × 1°)
  - subsquare: a–x, 24×24 (5′ × 2.5′)
  - extended: 0–9 digits, 10×10
- Handle upper and lower case, odd-length input, and the ±90°/180° edges.

**Degree-minute-second parsing edge cases**
- Negative values with zero degrees ("-0°30′").
- Hemisphere letter combined with a minus sign.
- Seconds that round up to 60.
- Unicode degree/prime/double-prime signs versus ASCII ' and ".
- Comma used as the decimal separator.
- "E" meaning East versus an exponent.
- Latitude/longitude order ambiguity.
- Packed formats like DDMMSS.s.
- GeographicLib's `DMS::Decode` is a good reference implementation.

**Web Mercator (EPSG:3857)**
- It uses the spherical formula applied to WGS84 coordinates, so it is not conformal on the ellipsoid.
- Latitude limit: ±atan(sinh π) = ±85.05112878°.
- Extent: ±20,037,508.342789 m.

## 10. Map tile math (XYZ and TMS)

- Number of tiles per side: n = 2^z.
- Tile x = ⌊(λ + 180)/360 · n⌋.
- Tile y = ⌊(1 − ln(tan φ + sec φ)/π)/2 · n⌋.
- **TMS numbers rows from the bottom:** y_TMS = n − 1 − y_XYZ.
- Ground resolution (256 px tiles) = 156,543.03392 · cos φ / 2^z meters per pixel.
- Quadkeys (Bing) interleave the x and y bits.
- Standard OSM "slippy map" convention, e.g. https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames (not re-fetched).

---

## Implications for a client-side product

1. **Use GeographicLib as the geodesic core.** npm `geographiclib-geodesic` (MIT, about 294 KB) or `geographiclib-rs` compiled to WebAssembly. Keep Vincenty only as a labeled comparison tool, and check the Rhumb gap in JS.
2. **Test against GeodTest.dat.** Ship the short 690 KB set in CI; keep the full 39 MB set out of the bundle.
3. **Offer two datum engines.** proj4js for light, common cases. For full PROJ, the browser build (Web Worker plus network grid fetch) is a heavy optional load. `proj.db` is roughly 10 MB ([unverified]) and is not on npm yet, so plan to build it yourself.
4. **Only request grids with user consent.** Grid downloads reveal a user's location, so a no-server product should make this opt-in. The CDN deletes logs after 24 hours, and PROJ-data licenses must be attributed per file.
5. **Load geoids on demand by tile.** Put EGM96-15 (2.1 MB) or egm96-5 (19 MB) in by default. EGM2008 at 2.5′ (75 MB) or 1′ (470 MB) should be fetched in pieces via HTTP range requests from static hosting. The PGM layout makes those reads simple.
6. **Magnetic models are cheap to bundle.** WMM2025 about 5 KB, IGRF-14 about 29 KB, WMMHR2025 about 534 KB, all public domain.
   - Always show the blackout/caution-zone flag and the uncertainty formula.
   - Hard-warn after 2030.0.
   - Leave out EMM: it expired and its redistribution terms are unclear.
7. **Label NATRF2022, NAPGD2022, GEOID2022 and SPCS2022 as "beta, not official"** until the Federal Register notice (expected early 2027).
   - SPCS2022 zone definitions are still changing, so version the data and plan to refresh it.
   - Use meters and the international foot for SPCS2022, and the US survey foot for SPCS83.
8. **MCP server (the agent-facing tool server).** Return provenance with every answer: model name and version, epoch, validity window, error estimate, and license. This matters most for magnetic variation and datum answers, where results expire or depend on epoch.
9. **Aviation magnetic variation.** Clearly separate computed WMM variation from the FAA-assigned epoch-year variation. Charts and procedures use the assigned value, not the live model.

**Still open:**
- `proj.db` and browser build sizes (measure from a build)
- SPCS2022 total zone count (fetch the beta zone-definitions page when it is back up)
- GEOID2022 file sizes
- FAA 8260.19K §2-5 wording
- Rhumb support in the JS package
