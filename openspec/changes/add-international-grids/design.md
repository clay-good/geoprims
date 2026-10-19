## Context

Research gathered 2026-10-19 from the issuers and the OSGeo PROJ-data repository, whose acceptance policy admits only grids under open licenses with a verifiable source; everything PROJ-data carries is therefore vetted for redistribution. Facts:
- OSTN15/OSGM15: BSD, 1 km native grid (41 MB CSV, or 15 MB NTv2 each way), 0.1 m RMS; 130 published test points with interpolation intermediates; OSGM15 geoid SE 0.010 m (GB).
- OSi/LPS polynomial (Irish Grid ↔ ETRS89): 3rd-order, 32 coefficients, 0.4 m at 95%, published in the OS developer pack under BSD; reverse by iteration.
- GDA94 → GDA2020: conformal (EPSG 8446) and conformal + distortion (8447) NTv2 grids, 83 MB each, CC BY 4.0; AUSGeoid2020 CC BY 4.0; the GDA2020 Technical Manual gives ALIC as the worked point.
- NZGD1949 → NZGD2000 distortion grid 318 KB, CC BY 4.0, accuracy degrading with earthquakes; the NZGD2000 deformation model (20180701) is CC BY 4.0; NZGeoid2016 CC BY 4.0; LINZ publishes worked examples.
- Canada: NAD83(CSRS) v7 velocity grid and CGG2013a under OGL-Canada; v8 (2025) not yet in PROJ-data.
- EUREF Technical Note 1 (March 4, 2024) gives ITRF ↔ ETRF2000/2014/2020 parameters and rates.
- Japan: zone definitions are public; TKY2JGD/PatchJGD/SemiDyna need Survey Act approval. Projections only.

## Goals / Non-Goals

**Goals:**
- Each country's official path, to the issuer's accuracy, verified against the issuer's test points and PROJ.
- Attribution exactly as each license requires, on the result and on `/licenses`.

**Non-Goals:**
- Approval-gated grids. Generic EPSG coverage.

## Decisions

### IG1. Reuse the NADCON5 and GEOID18 machinery
Every grid becomes a float32 tiled asset with the issuer's interpolation (bilinear for OSTN15's native 1 km grid as OS specifies; NTv2 bilinear for ICSM and LINZ; biquadratic where the issuer says so) through the existing asset provider, signed index, integrity, and 1° privacy-tile rules. Grids under 1 MB (NZGD2000 distortion, BETA2007, CHENyx06, the French grids, the Dutch grids) are single files. The large ones (OSTN15 15 MB each way, GDA2020 83 MB each) are tiled at 1° × 1° and offered as offline packs by country.

### IG2. OSTN15 from the native data file, not the NTv2
OS's native 1 km easting/northing shift grid with the OSGM15 height column is the authoritative product (NTv2 is derived from it to 1 mm). The tool implements OS's documented procedure (TM on Airy from ETRS89 via the shift grid, iterative reverse) and reproduces the developer pack's 40 + 90 test points with their interpolation intermediates. The datum flag per cell (which OSGM15 datum applies: Newlyn, Lerwick, Stornoway, and the others) is carried into the height result.

### IG3. Grid references as parsers and formatters
OS National Grid letters (two letters, 500 km and 100 km squares, 2 to 10 figures), Irish Grid letters (one letter, 25 squares), and ITM are handled in `geodesy.grid-ref` like MGRS: encode by truncation, decode to the square's south-west corner and center with the square size, reject nonexistent squares, and accept spaces or none. The coordinate parser's auto-detect learns these forms.

### IG4. Transformations name their frame and epoch
GDA94 ↔ GDA2020 reports which grid (conformal or distortion) and why (the manual's guidance on when distortion applies); NZGD2000 results carry the epoch and the deformation model version, and a point given at another epoch is propagated with the model's stated uncertainty; NAD83(CSRS) propagation names the realization epochs (1997.0, 2002.0, 2010.0); ETRF results name the realization and the technical note's table used. The existing `REALIZATION_ASSUMED`, `FRAME_MISMATCH`, and `DEFORMATION_ZONE` warnings apply.

### IG5. Projections from the issuers' definitions
National projections come from the issuers' published parameters into the CRS registry: OSGB36 BNG (Airy, 0.9996012717), TM75 (Airy modified), ITM (GRS80), MGA2020 zones, NZTM2000, MTM zones, ETRS89 LAEA (3035) and LCC (3034), RD (the RDNAPTRANS2018 projection), Gauss-Krüger 3° zones, Lambert-93 and the CC zones, Lo zones (south-oriented TM, k = 1, odd central meridians), and the 19 JGD2011 plane rectangular zones. LV95 uses swisstopo's exact formulas (December 2016) with its Rigi example as the vector. The existing projection engine (TM, LCC, LAEA via Albers machinery, Hotine) covers every method; Switzerland's is a Hotine oblique Mercator variant, implemented as swisstopo's approximate formulas (sub-millimeter) and checked against PROJ's `somerc`.

### IG6. Verification
- Issuer test points: OS developer pack (130 points, including Caister Water Tower 651409.804, 313177.450, 63.822 m), OS NTv2 TP01–TP40, OSGM15 NI test files, GDA2020 manual (ALIC), LINZ datum transformation examples, swisstopo Rigi, NSGI check point (5°E 52°N 43 m → 128410.0958, 445806.496, 43.0), BETA2007's 31 test points, each to the issuer's printed precision.
- Differential: PROJ 9.x with the same PROJ-data grids over 10,000 points per country within the grid interpolation tolerance (1 mm) and the issuer's stated accuracy for polynomial paths.
- Round trips: forward and reverse of every grid path within 1 mm (polynomial within 10 mm, as the issuer states).

## Tool inventory

| Group | Operations | Endpoints | Reference | Independent vector |
|---|---|---|---|---|
| `geodesy.grid-ref` | bng-forward, bng-inverse, irish-grid-forward, irish-grid-inverse, itm-forward, itm-inverse | 6 | OS Guide to Coordinate Systems v3.6; OSi | OS and OSi examples |
| `geodesy.datum` | ostn15, osi-polynomial, gda2020, nzgd2000, nzgd2000-epoch, nad83csrs-epoch, etrf, chenyx06, rdnaptrans, beta2007, rgf93 | 11 | each issuer's guide; EUREF TN1 | issuer test points |
| `geodesy.height` | osgm15, ausgeoid2020, nzgeoid2016, cgg2013a, nlgeo2018, raf20, sageoid2010 | 7 | each issuer | issuer test points |
| `geodesy.projection` | national CRSs through the registry; lv95-forward, lv95-inverse by swisstopo's formulas | 2 | swisstopo 2016 | Rigi |
| **Total** | **26** | **26 + high-intent pairs** | | |

The geodesy rollup moves from 89 to 115 operations. Generated pairs: `bng-to-latlon`, `latlon-to-bng`, `mga-to-latlon`, `nztm-to-latlon`, each allow-listed with a justification.

## Risks / Trade-offs

- **[Asset size and egress]** → Tiling, offline packs by country, and the existing 50 MB confirmation rule.
- **[Issuers update grids (NAD83(CSRS) v8, NZGD2000 deformation)]** → Ledger rows with `nextExpected`; versions in `meta.assets`.
- **[A license is misread]** → Only grids in PROJ-data or with the license text in hand are included; task 1 records the license text per asset, and `/licenses` shows it.

## Migration Plan

The BNG entry in the CRS registry keeps its 7-parameter path as the fallback when the OSTN15 asset is absent, with `LOW_ACCURACY_TRANSFORM`, so existing permalinks keep working and improve when the asset is present.

## Open Questions

- Whether to tile OSTN15 at 1° or ship the whole 15 MB grid as one offline pack; decided by the size and privacy gates in task 2.
- NAD83(CSRS) v8 velocity grid: added when PROJ-data carries it or NRCan's download is confirmed open.
