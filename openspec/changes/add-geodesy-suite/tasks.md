## 1. Parsing and formatting

- [ ] 1.1 Implement the notation grammar (DD, DMS, DDM, packed, labeled, hemisphere forms, Unicode marks); verify every parsing scenario plus a 500-case fixture corpus (built: DD, DMS, DDM, Unicode marks, hemisphere prefix and suffix, colon forms, packed aviation, labeled pairs, MGRS and UTM routing, with every parsing scenario; pending: the 500-case corpus and the other grid notations)
- [ ] 1.2 Implement ambiguity detection with alternatives (order, exponent E, decimal comma, multi-grid strings); verify the ambiguity scenarios (built: order inference, the unlabeled-pair alternative, and decimal commas; pending: exponent-E and multi-grid ambiguity)
- [x] 1.3 Implement strict component validation; verify the seconds-overflow, contradictory-sign, and negative-zero scenarios
- [x] 1.4 Implement the formatter with rounding carry and resolution reporting; verify the carry and resolution scenarios
- [ ] 1.5 Implement angle arithmetic and bearing difference; verify across-north cases and a property test (built: bearing difference across north; pending: DMS add and subtract)

## 2. Ellipsoids and frames

- [ ] 2.1 Implement the ellipsoid catalog and derived parameters; verify the WGS 84 scenario and published values for each ellipsoid
- [ ] 2.2 Implement radii of curvature, meridian arc, degree lengths, and auxiliary latitudes; verify the 45° scenarios and round trips against GeographicLib
- [ ] 2.3 Implement geodetic ↔ ECEF (non-iterative inverse); verify the forward and pole scenarios and 1e-9 m accuracy over the full height range
- [ ] 2.4 Implement ENU, NED, and AER conversions with rotation matrices; verify the overhead-target and round-trip scenarios

## 3. Datums

- [ ] 3.1 Implement 7- and 14-parameter Helmert in both conventions; verify the convention-required scenario and IERS published examples
- [ ] 3.2 Load NGS, IERS, NGA, and EPSG parameter sets with citations; verify each set against its source document example
- [ ] 3.3 Implement the frame graph and path selection with accuracy accumulation; verify the realization-assumed and coincidence scenarios
- [ ] 3.4 Implement ITRF2020 plate-motion propagation, site-velocity override, and deformation-zone flag; verify the San Andreas scenario
- [ ] 3.5 Implement NADCON5 grid transformations with chaining; verify against NCAT outputs for 200 points per region and the outside-grid scenario
- [ ] 3.6 Implement the WGS 84 vs NAD83 displacement explainer with canvas arrow; verify the Kansas scenario
- [ ] 3.7 Implement legacy shifts with `LOW_ACCURACY_TRANSFORM`; verify the ED50 scenario
- [ ] 3.8 Implement NATRF2022 beta transformations with labeling; verify the beta scenario
- [ ] 3.9 Run HTDP differential tests (ITRF2020 ↔ NAD83(2011)); verify ≤ 1 mm agreement

## 4. Projections

- [ ] 4.1 Port GeographicLib TransverseMercator and TransverseMercatorExact; verify the Pittsburgh scenario and GeographicLib's TM test set (built: the 6th-order Krüger series with the Pittsburgh scenario, round trips, and finite-difference checks of convergence and scale; pending: TransverseMercatorExact and the GeographicLib TM test set)
- [x] 4.2 Implement UTM zone rules (Norway, Svalbard, boundary assignment) and forced zones; verify the exception and forced-zone scenarios
- [x] 4.3 Port PolarStereographic and UPS; verify the UPS scenario
- [ ] 4.4 Implement LCC, Albers, Hotine, Polar Stereographic variants, azimuthal equidistant, equidistant cylindrical, orthographic, gnomonic, and Web Mercator; verify domain-limit scenarios and PROJ differential tests (10,000 points each)
- [ ] 4.5 Build SPCS83 and SPCS2022-beta support with zone lookup and legal units; verify the Pennsylvania South and outside-zone scenarios and NCAT agreement on 50 points per zone
- [ ] 4.6 Implement CRS search by code, name, and location and composite CRS transform with step reporting; verify the Denver and composite-path scenarios
- [ ] 4.7 Implement convergence, scale factor, and arc-to-chord correction; verify the UTM convergence scenario and PROJ factors agreement (built: UTM and UPS convergence and point scale; pending: arc-to-chord and PROJ agreement)
- [ ] 4.8 Run the round-trip property suite for every projection pair; verify 1e-9 m round trips

## 5. Grid references

- [ ] 5.1 Port GeographicLib MGRS (truncation, polar bands, lettering schemes, band tolerance, invalid-square rejection); verify all MGRS scenarios and NGA/GEOTRANS test points (built: truncation, polar bands, AA lettering, band tolerance, invalid-square rejection, and every MGRS scenario; pending: the AL scheme for legacy ellipsoids and NGA/GEOTRANS test points)
- [ ] 5.2 Implement USNG encode/decode including local truncated references; verify the space-delimited scenario
- [ ] 5.3 Implement Maidenhead with edge clamping; verify the six-character and edge scenarios
- [ ] 5.4 Implement GARS and GEOREF; verify published examples and the keypad scenario
- [ ] 5.5 Implement grid overlays for UTM, MGRS, Maidenhead, and GARS; verify the MGRS overlay visual fixture

## 6. Heights

- [ ] 6.1 Port the GeographicLib geoid evaluator for tiled PGM grids (cubic and bilinear); verify the differential check against GeoidEval
- [ ] 6.2 Integrate EGM96, EGM2008 (5′, 2.5′, 1′), and GEOID18 assets with coverage checks; verify the GEOID18-coverage scenario
- [ ] 6.3 Implement h ↔ H with frame-consistency checks; verify the NAVD 88 and frame-mismatch scenarios
- [ ] 6.4 Implement the height reference converter with the vertical diagram; verify the drone-altitude scenario
- [ ] 6.5 Implement geoid model comparison (point and profile); verify the EGM96 vs EGM2008 scenario
- [ ] 6.6 Gate GEOID2022/NAPGD2022 on asset presence with beta labeling; verify the label scenario

## 7. Geomagnetism

- [x] 7.1 Implement the spherical-harmonic evaluator with secular variation; verify all WMM2025 official test values
- [ ] 7.2 Add WMMHR2025 and IGRF-14 models with validity windows; verify the model-selection and historical scenarios (built: IGRF-14 with DGRF/provisional/predictive labels, matching ppigrf within 0.001 nT, and the historical scenario; pending: WMMHR2025)
- [x] 7.3 Implement uncertainty and blackout/caution zones; verify both zone scenarios
- [x] 7.4 Implement true ↔ magnetic conversion with chart variation parsing and model comparison; verify the chart-variation scenarios
- [ ] 7.5 Implement grivation; verify the UPS scenario
- [ ] 7.6 Implement isogonic overlays in a worker; verify the overlay visual fixture

## 8. Catalog, docs, and promotion

- [ ] 8.1 Register all 89 operations and the 84 allow-listed pairs with aliases (e.g. "lat long to UTM", "grid ref", "mag var"); verify catalog counts
- [ ] 8.2 Write docs for every geodesy tool per the tool-docs template and the "Convert survey coordinates to GPS" guide; verify the docs build passes
- [ ] 8.3 Promote tools meeting the stable bar; verify the verification report lists each tool's vectors and max error
