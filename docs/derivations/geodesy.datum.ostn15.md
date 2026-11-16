<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GNSS position to British National Grid by OSTN15 (`geodesy.datum.ostn15`)

## Method

OSTN15, together with the ETRS89 positions of Ordnance Survey's OS Net GNSS stations, is the official definition of the OSGB36 National Grid. OSGM15 is the geoid model that turns an ellipsoidal height into a height above Ordnance Datum Newlyn or the island datums. Both come as one data file: at every node of a 1 km grid, 700 km by 1,250 km, the easting shift, the northing shift, the geoid height, and a height-datum flag. The tool follows Ordnance Survey's Transformations and OSGM15 User Guide (v1.3, 2020) step by step:

1. **ETRS89 onto the grid's projection.** The latitude and longitude go onto the National Grid transverse Mercator on GRS 80 by OS's own formulas (annexe B), giving the ETRS89 easting `x` and northing `y`.
2. **The kilometre square.** Record number `⌊x/1000⌋ + ⌊y/1000⌋ × 701 + 1` is the south-west corner. Its east, north-east, and north neighbors are the other three.
3. **Bilinear shifts.** With `t` and `u` the point's fractional position in the square, each of `se`, `sn`, and `sg` is `(1−t)(1−u)s0 + t(1−u)s1 + tu·s2 + (1−t)u·s3`.
4. **The answer.** `E = x + se`, `N = y + sn`, and the orthometric height is `h − sg`.
5. **The datum flag.** If the four corners agree, that flag. Otherwise it is the flag of the corner in whose quadrant the point lies.

| Flag | Height datum |
|---|---|
| 1 | Newlyn, mainland Great Britain |
| 2 | St Marys, Scilly Isles |
| 3 | Douglas02, Isle of Man |
| 4 | Stornoway15, Outer Hebrides |
| 6 | Lerwick, Shetland Isles |
| 7 | Newlyn, Orkney Isles |
| 15 | Newlyn Offshore |
| 16 | Outside the transformation area: refused |

The grid is the asset `ostn15@2016`, packed from OS's data file by `tools/data/ostn15-assets.mjs` with every value kept to the millimeter.

## Equations

- Projection (annexe B): `n = (a−b)/(a+b)`, `ν = aF₀(1−e²sin²φ)^−½`, `ρ = aF₀(1−e²)(1−e²sin²φ)^−³⁄₂`, `η² = ν/ρ − 1`, the meridian arc `M` in `n`, and `N = I + II·P² + III·P⁴ + IIIA·P⁶`, `E = E₀ + IV·P + V·P³ + VI·P⁵` with `P = λ − λ₀`.
- Constants: GRS 80 `a = 6,378,137.000 m`, `b = 6,356,752.3141 m`; `F₀ = 0.9996012717`, `φ₀ = 49°N`, `λ₀ = 2°W`, `E₀ = 400,000 m`, `N₀ = −100,000 m`.
- Shifts: `s = Σ wᵢ·sᵢ` with `w = ((1−t)(1−u), t(1−u), tu, (1−t)u)`, `t = (x − x₀)/1000`, `u = (y − y₀)/1000`.

## Symbols and units

Latitude and longitude in degrees on ETRS89; ellipsoidal height, orthometric height, eastings, northings, and shifts in meters; the datum flag a small whole number.

## Domain

- Inside the grid, 0 to 700 km east and 0 to 1,250 km north on the ETRS89 projection, and not where the datum flag is 16: elsewhere `OUT_OF_DOMAIN`.
- The grid is fetched on first use; without it the answer is `ASSET_UNAVAILABLE`, and a damaged copy is `ASSET_INTEGRITY`.

## Approximations

OS's annexe B projection is Redfearn's series. Far from the central meridian it differs from an exact transverse Mercator by up to about a millimeter. OS's published results are computed with it, and OSTN15 is defined against it, so the tool uses it too. The coordinates are taken to be ETRS89, the `REALIZATION_ASSUMED` warning. A WGS 84 or ITRF position is about 0.8 m from it in Britain today.

## Worked example

- sourcePublisher: Ordnance Survey
- sourceTitle: Transformations and OSGM15 User Guide
- sourceEdition: v1.3, November 2020
- sourceLocator: Annexe A, Caister Water Tower
- independent: yes
- inputs: lat 52°39'28.8282"N, lon 1°42'57.8663"E, height 108.05 m
- outputs: E 651409.804 m, N 313177.450 m, orthometric height 63.822 m on Newlyn (flag 1); shifts se 102.801 m, sn −78.236 m, sg 44.228 m
- tolerance: 0.0005 m, the printed millimeter
- verifiedBy: golden vector v001 and `core/crates/gp-geo/tests/ostn15.rs` `caister_water_tower_step_by_step`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_ostn15.py` builds the other 40 vectors in `core/vectors/geodesy.datum.ostn15.jsonl` from Ordnance Survey's own test files in the OSTN15-OSGM15 Developers Pack, TP01 to TP40 from the Scilly Isles to Shetland and across every height datum. Each pins the easting, northing, and orthometric height to the printed millimeter, the datum flag exactly, and the three interpolated shifts to the five decimals OS prints.

## Invariants

- `core/crates/gp-geo/tests/ostn15.rs` `caister_water_tower_step_by_step`: the ETRS89 projection to OS's 0.1 mm, all four corner record numbers and their values exactly, the interpolated shifts, the answer, and the datum flag.
- `core/crates/gp-geo/tests/ostn15.rs` `the_projection_reproduces_both_of_os_worked_examples`: annexe B's own Airy 1830 example forward and back.
- `core/crates/gp-geo/tests/ostn15.rs` `outside_the_grid_is_none_and_a_bad_package_is_refused`: points off the grid have no shifts, and a truncated or foreign package is refused.
- `tools/data/ostn15-assets.test.mjs`: the committed package has the pinned digest, and the packer keeps values to the millimeter and refuses records out of order.
