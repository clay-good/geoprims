## 1. COGO and traverse

- [ ] 1.1 Implement direction parsing and formatting (quadrant bearings, azimuths, gons); verify the quadrant parse and invalid-bearing scenarios (done so far: quadrant bearings with DMS or dashes, azimuths, and bearing formatting; pending: gons)
- [x] 1.2 Implement inverse, forward, and radial sideshots; verify the inverse scenario
- [ ] 1.3 Implement traverse closure (angular and linear misclosure, precision ratio, standards comparison); verify the loop-closure and angular-misclosure scenarios (done so far: linear misclosure, its direction, and the precision ratio with the perfect-closure warning; pending: angular misclosure and the standards comparison)
- [ ] 1.4 Implement compass, transit, and Crandall adjustments; verify the Bowditch and transit scenarios and exact closure to 1e-9 (done so far: compass and transit rules, with the adjusted loop closing exactly; pending: Crandall)
- [ ] 1.5 Implement the experimental 2D least-squares adjustment with error ellipses and chi-square test; verify against Ghilani's published examples and the chi-square-failure scenario
- [ ] 1.6 Implement intersections and resection with danger-circle detection; verify the two-solution and danger-circle scenarios
- [ ] 1.7 Implement area by coordinates, station/offset tools, and unit context enforcement; verify the rectangle and mixed-feet scenarios (done so far: area by coordinates with US survey acres for US survey feet, and the mixed-feet UNIT_MISMATCH guard on every survey tool; pending: station and offset tools)
- [ ] 1.8 Implement traverse sketches and calculation sheets; verify the exaggeration label and sheet contents

## 2. Instrument reductions

- [x] 2.1 Implement slope reduction and two-face means; verify the 500 m and two-face scenarios (`survey.reduction.slope`: HD, VD, and elevation difference from a slope distance and a zenith or vertical angle in degrees or DMS, the two-face mean and index error, a face-right reading entered as the zenith caught by name, and curvature and refraction added beyond a threshold; 10 golden vectors and the two scenarios in `core/crates/gp-survey/tests/survey.rs`: 498.097 m and 43.578 m; 85°00'15" and -5")
- [x] 2.2 Implement curvature and refraction with labeled coefficients; verify the 1 km scenario (`survey.reduction.curvature-refraction`: (1 − k)·D²/2R with k (default 0.13) and R shown, the curvature and refraction parts, and the coefficient labeled with its k; 1 km gives 0.0675 m at k = 0.14 and 0.0683 m at k = 0.13; 6 golden vectors)
- [ ] 2.3 Implement EDM atmospheric and prism corrections; verify the ppm scenario and a manufacturer-formula fixture
- [x] 2.4 Implement elevation factor, combined factor, and grid/ground conversions with the orthometric guard; verify the combined-factor and guard scenarios
- [ ] 2.5 Implement level runs, arithmetic check, closure, and adjustment; verify the arithmetic-check scenario and a textbook loop
- [x] 2.6 Implement stadia, inaccessible heights, and offset shots; verify the tower and tree-center scenarios (done so far: `survey.reduction.stadia` (K·s·sin²Z + C·sin Z, with K and C editable) and `survey.reduction.inaccessible-height` from one station and a distance, or two stations in line with a baseline; the tower scenario applies curvature and refraction beyond the threshold to each sight's height and notes that it cancels in the tower's height. 12 golden vectors, the two-station ones built from a known tower.. `survey.cogo.offset-shot` takes distance offsets (right or left, out or in) or an angle offset to the center of a tree or pole, the center at the measured distance plus the radius along the direction turned to it; 6 golden vectors with the tree-center scenario)

## 3. Earthwork and grade

- [x] 3.1 Implement average end area and prismoidal with the averaged-middle guard; verify the three volume scenarios
- [ ] 3.2 Implement cross-section areas with daylight points; verify the mixed-section scenario
- [ ] 3.3 Implement borrow-pit and four-point methods with balance-line rendering; verify the corner-weights scenario
- [x] 3.4 Implement shrink/swell and haul loads; verify the truck-load scenario
- [ ] 3.5 Implement grade conversions with ratio disambiguation; verify the ambiguous-ratio scenario
- [ ] 3.6 Implement slope staking with Brent iteration; verify the catch-point scenario
- [ ] 3.7 Implement TIN stockpile and solid volumes; verify the TIN scenario against an analytic cone
- [ ] 3.8 Implement profile slope analysis; verify the grade-threshold scenario

## 4. Alignment curves

- [x] 4.1 Implement circular curve element solving from any two inputs with arc and chord definitions; verify the R = 500 ft and inconsistent-input scenarios
- [ ] 4.2 Implement stationing and layout tables with coordinates; verify the layout scenario
- [ ] 4.3 Implement spirals and spiral-curve-spiral stations; verify the spiral scenario against a textbook example
- [ ] 4.4 Implement vertical curves (equal and unequal tangents, turning point, K); verify the crest and no-turning-point scenarios (done so far: equal-tangent curves with the high or low point and K, in 100 ft or 1,000 m stationing; pending: unequal tangents)
- [ ] 4.5 Implement sight-distance curve lengths with the design K and heights as cited user inputs (no reproduced AASHTO tables); verify the crest SSD scenario
- [ ] 4.6 Implement plan and profile visualization; verify visual fixtures

## 5. Catalog and docs

- [ ] 5.1 Register all 57 operations with alias slugs with surveyor aliases (COGO, Bowditch, compass rule, lat/dep, cut and fill, AEA); verify catalog counts
- [ ] 5.2 Write docs per tool and the "Close a traverse" guide; verify the guide chain end to end
- [ ] 5.3 Promote tools meeting the stable bar; verify the verification report
