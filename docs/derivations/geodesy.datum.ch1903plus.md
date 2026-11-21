<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# CH1903+ to ETRS89 by swisstopo's parameters (`geodesy.datum.ch1903plus`)

## Method

LV95, Switzerland's national grid, is a projection of CH1903+, and a GNSS position is on ETRS89 or WGS 84. The step between those datums is the one the Swiss projection tools leave out, and swisstopo publishes it as a geocentric translation and nothing more: the Swiss frame CHTRS95 is tied to ETRS89, and CH1903+ sits a fixed distance from it along the ECEF axes. There are no rotations and no scale.

1. **To geocentric coordinates.** The latitude, longitude and ellipsoidal height go to Cartesian X, Y, Z on the ellipsoid of the datum given — Bessel 1841 for CH1903+, GRS 80 for ETRS89.
2. **Translate.** Add or subtract 674.374 m, 15.056 m and 405.346 m along X, Y and Z.
3. **Back to a latitude and longitude.** On the other datum's ellipsoid, by Vermeille's closed form, as GeographicLib's `Geocentric::Reverse` does it — no iteration.

The horizontal move is reported as a ground distance with its direction, and the height change separately, because the step does both: across Switzerland it carries a position about 160 m south-west and 50 m up.

swisstopo states plainly that these parameters applied to CH1903, rather than CH1903+, can be wrong by up to 1.6 m, because of the local distortions of the 1903 triangulation. The tool cannot tell which one it was handed, so it says so on every answer and names `geodesy.datum.chenyx06`, which removes those distortions first.

## Equations

- `R_N = a / √(1 − e² sin²φ)`, `X = (R_N + h) cos φ cos λ`, `Y = (R_N + h) cos φ sin λ`, `Z = (R_N(1 − e²) + h) sin φ`.
- `X_CH1903+ = X_ETRS89 − 674.374`, `Y_CH1903+ = Y_ETRS89 − 15.056`, `Z_CH1903+ = Z_ETRS89 − 405.346`, all metres; the other direction adds them.
- The reverse conversion is Vermeille's closed form on the target ellipsoid.
- Ground move: `north = Δφ·M`, `east = Δλ·N cos φ`, `shift = √(north² + east²)`, `azimuth = atan2(east, north)`, with `M` and `N` the meridian and prime-vertical radii of the target ellipsoid at the point.

## Symbols and units

Latitude and longitude in degrees, height in metres above the ellipsoid of whichever datum the coordinates are on; the translation in metres along the ECEF axes; the move in metres and its direction in degrees from true north.

## Domain

- Anywhere, arithmetically; the parameters are Switzerland's and mean nothing elsewhere. The only refusal is a position at the centre of the Earth, where latitude has no meaning, which is `OUT_OF_DOMAIN`.
- No asset: the three parameters are in the code, cited to swisstopo.

## Approximations

None in the step itself: a geocentric translation is exact, and the conversions either side are closed-form. The published parameters are what carry the uncertainty, and swisstopo gives the one figure that matters — up to 1.6 m if they are applied to CH1903 rather than CH1903+. Bessel 1841 here is swisstopo's own constant, 1/f = 299.15281285, which is what its printed e² of 0.006674372230614 states; PROJ's `+ellps=bessel` is EPSG's 299.1528128, and the two disagree by 4 nanometres of latitude and 2 micrometres of height, which is the tolerance on the PROJ vectors. ETRS89 here is CHTRS95; a consumer receiver's WGS 84 is about 0.8 m away in Switzerland today, which is the `REALIZATION_ASSUMED` warning. The step is three-dimensional: 100 m of height error moves the latitude about 2 mm and the longitude about 1 mm, measured.

## Worked example

- sourcePublisher: Federal Office of Topography swisstopo
- sourceTitle: Formulas and constants for the calculation of the Swiss conformal cylindrical projection and for the transformation between coordinate systems
- sourceEdition: December 2016 edition
- sourceLocator: Section 7.1, the numerical example: Zimmerwald, a Swiss EUREF point, with every intermediate printed
- independent: yes
- inputs: lat 46°52'42.269284"N, lon 7°27'58.416328"E, height 897.361 m
- outputs: ETRS89 46°52'37.540562"N 7°27'54.983506"E, 947.149 m
- tolerance: 0.00001 arcseconds and 0.0005 m
- verifiedBy: golden vectors v001 and v002 and `core/crates/gp-geo/tests/ch1903.rs` `swisstopos_five_euref_points_with_every_intermediate`
- verifiedOn: 2026-11-21

## Differential tests

- `tools/vectors/gen_ch1903plus.py` builds 28 of the 38 vectors in `core/vectors/geodesy.datum.ch1903plus.jsonl` from PROJ 9.9.0 applying the same three parameters through `cart` and `helmert`, an independent implementation of the translation and of the conversions either side: twenty places from Geneva to Davos and Schaffhausen to Lugano, and eight of them in reverse, every latitude and longitude within 1e-10 degrees and every height within 0.01 mm, which is the distance between PROJ's Bessel and swisstopo's. The other ten are swisstopo's five EUREF points, each way.
- `core/crates/gp-geo/tests/ch1903.rs` `swisstopos_five_euref_points_with_every_intermediate`: swisstopo prints the geocentric coordinates on both sides of the translation as well as the ellipsoidal coordinates, and all four numbers reproduce at all five points — the geocentric ones to the millimetre it prints, the ellipsoidal ones to a hundred-thousandth of a second, 0.3 mm on the ground — in both directions, which is its section 7.2.

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `ch1903plus_moves_switzerland_south_west_and_up_and_round_trips`: at five places the move is south, west and up, between 140 and 190 m horizontally and 45 to 55 m in height, with an azimuth in the third quadrant; the reported height change is the difference between the two heights exactly; the reverse returns the point to 1e-11 degrees and 0.1 mm and reports the same move the other way; every answer carries swisstopo's 1.6 m warning about CH1903 against CH1903+; and 100 m of height error moves the latitude about 2 mm, the figure the limitations give.
