<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Irish Grid to ETRS89 by the OSi and LPS polynomial (`geodesy.datum.osi-polynomial`)

## Method

In the Republic of Ireland and Northern Ireland, Ordnance Survey Ireland (now Tailte Éireann) and Land & Property Services recommend one transformation between the Irish Grid (TM75) and ETRS89: a third-order polynomial in latitude and longitude, fitted to points known in both. Ordnance Survey's Transformations and OSGM15 User Guide (v1.3, 2020) publishes its coefficients. Heights go between the ellipsoid and the two Irish height datums, Malin Head in the Republic and Belfast in Northern Ireland, by OSGM15's Irish geoid grids. The tool takes four steps:

1. **Off the Irish Grid.** The easting and northing come off the Irish Grid transverse Mercator (Airy Modified 1849, true origin 53°30′N 8°W, scale 1.000035, false origin 200 km west and 250 km south) by OS's annexe C formulas, giving TM75 latitude and longitude.
2. **The polynomial.** With `U = 0.1(φ − 53.5)` and `V = 0.1(λ + 7.7)` in degrees, the shifts in seconds are `Δφ = Σ Aᵢⱼ Uⁱ Vʲ` and `Δλ = Σ Bᵢⱼ Uⁱ Vʲ` over i, j from 0 to 3, added to give ETRS89.
3. **The height.** OSGM15's geoid height on the datum named is added to the orthometric height. The guide asks for the geodetic grid to be resampled onto a 1 km Irish Transverse Mercator grid first. The tool takes each corner of the point's kilometre square from the geodetic grid bilinearly, then interpolates the four.
4. **Irish Transverse Mercator.** The ETRS89 point goes onto ITM by OS's annexe B formulas, for the grid current Irish mapping uses.

## Equations

- `U = K₀(φ − φm)`, `V = K₀(λ − λm)`, `K₀ = 0.1`, `φm = 53.5°`, `λm = −7.7°`.
- `φ_ETRS = φ_TM75 + Δφ/3600`, `λ_ETRS = λ_TM75 + Δλ/3600`.
- `h_ellipsoidal = H_orthometric + N_OSGM15`.
- The 16 coefficients A and B are the guide's table, in seconds.

## Symbols and units

Eastings, northings, and heights in meters; latitude and longitude in degrees; the polynomial's shifts in arcseconds.

## Domain

- The height datum is required: Belfast or Malin Head. The tool has no border to decide it.
- Outside OSGM15's Irish grids, 51°N to 56°N and 11°30′W to 5°W, there are no geoid values, and the answer is `OUT_OF_DOMAIN`.
- The grids are fetched on first use: `ASSET_UNAVAILABLE` without them, and `ASSET_INTEGRITY` for a damaged copy.

## Approximations

The polynomial itself is the official transformation, stated at 0.4 m (95%) against the old triangulation, because it fits a smooth surface to a network that was never perfectly regular. The guide does not say how to resample the geoid onto the 1 km ITM grid. Bilinear resampling brings heights within 0.45 mm of OS's published ones, against 1.1 mm for plain bilinear interpolation on the geodetic grid.

## Worked example

- sourcePublisher: Ordnance Survey
- sourceTitle: OSGM15 Northern Ireland test files, in the OSTN15-OSGM15 Developers Pack
- sourceEdition: Test files of 2016-05-09
- sourceLocator: OSGM15_NI_TestOutput_IGtoETRS89.txt, NI_IG_1
- independent: yes
- inputs: easting 223657.156 m, northing 340618.902 m, height 37.958 m, datum Belfast
- outputs: 54°18'49.673618"N 7°38'14.133209"W, 95.4675 m
- tolerance: 5e-7 arcseconds on the position; 0.001 m on the height
- verifiedBy: golden vector v001 and `core/crates/gp-geo/tests/osi.rs` `ni_test_point_both_ways`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_osi.py` builds all 100 vectors in `core/vectors/geodesy.datum.osi-polynomial.jsonl` from Ordnance Survey's OSGM15_NI_TestOutput_IGtoETRS89.txt: every point's latitude and longitude to half the printed millionth of a second, and its ellipsoidal height to a millimeter.

## Invariants

- `core/crates/gp-geo/tests/osi.rs` `ni_test_point_both_ways`: the specification's test point forward, and the iteration back to the TM75 point it came from.
- `core/crates/gp-geo/tests/osi.rs` `the_geoid_grid_is_bounded_and_a_bad_package_refused`: points off the grid have no geoid height, the corners read their own nodes, and a truncated package is refused.
