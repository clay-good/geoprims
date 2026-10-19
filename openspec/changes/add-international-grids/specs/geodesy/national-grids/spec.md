## Purpose

Gives users outside the United States their national grid references, projections, and the official datum and height transformations, with the issuer's accuracy and attribution, from redistributable grids.

## ADDED Requirements

### Requirement: National grid references
The domain SHALL parse and format OS National Grid references (two letters, 2 to 10 figures), Irish Grid references (one letter), and ITM references, encoding by truncation and decoding to the square's south-west corner and center with the square size, rejecting nonexistent squares, and accepting spaces or none. The coordinate parser's auto-detection SHALL recognize them.

#### Scenario: OS reference
- **WHEN** `TQ 30 80` is decoded
- **THEN** the square is 1 km with south-west corner E 530000 N 180000, and encoding E 530624.974 N 178388.464 to 10 figures gives `TQ 30624 78388`

#### Scenario: Nonexistent square
- **WHEN** `TI 30 80` is decoded
- **THEN** the result is `INVALID_INPUT` stating that the letter I is not used

### Requirement: OSTN15 and OSGM15
The domain SHALL transform ETRS89 ↔ OSGB36 National Grid by the OSTN15 1 km shift grid with Ordnance Survey's documented procedure (bilinear interpolation, iterative reverse) and SHALL convert ETRS89 heights to ODN and the island datums by OSGM15, carrying the per-cell datum flag. Results SHALL state the 0.1 m RMS accuracy and carry the notice "© Copyright and database rights Ordnance Survey Limited 2016. All rights reserved." Without the asset, the registry's 7-parameter path SHALL be used with `LOW_ACCURACY_TRANSFORM`.

#### Scenario: Caister Water Tower
- **WHEN** 52°39′28.8282″N 1°42′57.8663″E, h 108.05 m (ETRS89) is transformed
- **THEN** the result is E 651409.804 N 313177.450, ODN height 63.822 m, each within 0.001 m

#### Scenario: Developer-pack test points
- **WHEN** the 130 OS test points are run forward and reverse
- **THEN** every result matches the published output file within 0.001 m and the interpolation intermediates match

### Requirement: Irish Grid and ITM
The domain SHALL transform Irish Grid (TM75) ↔ ETRS89 (ITM) by the OSi/LPS third-order polynomial (forward direct, reverse by iteration to 1 mm), stating its 0.4 m (95%) accuracy, and SHALL convert heights by the OSGM15 Malin and Belfast grids with the three-agency notice.

#### Scenario: Northern Ireland test point
- **WHEN** the OSGM15 NI test point IG 223657.156, 340618.902, 37.958 is transformed to ETRS89
- **THEN** the result is 54°18′49.673618″N 7°38′14.133209″W, h 95.4675 m, within 0.001 m

### Requirement: GDA2020 and AUSGeoid2020
The domain SHALL transform GDA94 ↔ GDA2020 by the conformal grid (EPSG 8446) or the conformal-plus-distortion grid (EPSG 8447), naming which and the manual's guidance for the choice, SHALL support MGA2020 and MGA94 zones, and SHALL convert ellipsoidal to AHD heights by AUSGeoid2020, with the CC BY 4.0 attribution.

#### Scenario: ALIC
- **WHEN** ALIC GDA94 −23°40′12.446019″ 133°53′07.847844″ h 603.3466 m is transformed by the conformal grid
- **THEN** the result is GDA2020 −23°40′12.39650″ 133°53′07.87779″ h 603.2489 m within 0.001 m, and the shift is reported as ΔN 1.5236 m, ΔE 0.8487 m, ΔU −0.0977 m

### Requirement: NZGD2000 and its deformation model
The domain SHALL transform NZGD1949 ↔ NZGD2000 by the LINZ distortion grid (stating that its accuracy degrades after 2000), SHALL propagate NZGD2000 coordinates between epochs with the NZGD2000 deformation model (version named), SHALL support NZTM2000, and SHALL convert heights by NZGeoid2016, with the LINZ attribution "Sourced from the LINZ Data Service and licensed for reuse under the CC BY 4.0 licence."

#### Scenario: LINZ example
- **WHEN** 36°30′00″S 175°00′00″E (NZGD1949) is transformed by the grid
- **THEN** the result is 36°29′53.48482″S 175°00′00.69469″E within 0.0001″

#### Scenario: Epoch propagation
- **WHEN** an NZGD2000 point observed at epoch 2024.5 is propagated to 2000.0
- **THEN** the deformation model's displacement is applied and its version and uncertainty are reported

### Requirement: Canada, Europe, and the other national paths
The domain SHALL provide: NAD83(CSRS) epoch propagation by the NRCan v7 velocity grid and CGG2013a heights (Open Government Licence – Canada attribution); ITRF ↔ ETRF2000/2014/2020 by EUREF Technical Note 1 (March 2024) with the realization and table named, and ETRS89-LAEA and LCC; LV95 by swisstopo's exact formulas with CHENyx06; RD by RDNAPTRANS2018 with NLGEO2018; DHDN Gauss-Krüger ↔ ETRS89 UTM by BETA2007; RGF93 Lambert-93 with the IGN grids and RAF20; Hartebeesthoek94 Lo zones with SAGEOID2010; and the 19 JGD2011 plane rectangular zones as projections only, with a note that GSI's datum grids are approval-only.

#### Scenario: Rigi
- **WHEN** the swisstopo Rigi example is projected to LV95
- **THEN** the result is E 2679520.05 N 1212273.44 within 0.01 m

#### Scenario: NSGI check point
- **WHEN** 5°E 52°N h 43 m (ETRS89) is transformed by RDNAPTRANS2018
- **THEN** the result is RD 128410.0958, 445806.496, NAP 43.0 m within the issuer's stated precision

#### Scenario: Japan
- **WHEN** a Tokyo-datum transformation is requested
- **THEN** the result is `UNSUPPORTED` explaining that GSI's parameter files require Survey Act approval, with the JGD2011 projection offered

### Requirement: Attribution and accuracy on every result
Every national transformation result SHALL name the grid, version, issuer, and stated accuracy, SHALL carry the license's required attribution text, and SHALL appear with that text on `/licenses`.

#### Scenario: Licenses page
- **WHEN** the OSTN15 asset is registered
- **THEN** `/licenses` lists it with the BSD license and the Ordnance Survey notice

### Requirement: Verification against the issuer and PROJ
Every grid path SHALL reproduce its issuer's published test points to their printed precision and SHALL agree with PROJ 9.x using the same PROJ-data grids within 1 mm over 10,000 differential points per country (10 mm for the Irish polynomial path, as the issuer's accuracy allows).

#### Scenario: PROJ differential
- **WHEN** 10,000 GB points are run through OSTN15 and PROJ's `uk_os_OSTN15_NTv2_OSGBtoETRS.tif`
- **THEN** every easting and northing agrees within 0.001 m
