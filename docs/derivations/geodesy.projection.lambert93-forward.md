<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to Lambert-93 (`geodesy.projection.lambert93-forward`)

## Method

French cadastral plans, IGN maps, and public datasets are drawn on Lambert-93, a Lambert conic conformal projection of RGF93 that covers mainland France and Corsica in one grid. Large-scale surveys use one of nine narrower conic zones instead, CC42 to CC50, named by the latitude of their origin, which keep the scale error small within a band one degree of latitude wide. EPSG registers them as 2154 and 3942 to 3950.

All ten grids are EPSG method 9802, Lambert Conic Conformal with two standard parallels, on GRS 80 and central meridian 3°E:

| Grid | Origin latitude | Standard parallels | False easting | False northing |
|---|---|---|---|---|
| Lambert-93 | 46°30′N | 44°N and 49°N | 700,000 m | 6,600,000 m |
| CC*z* (42 to 50) | *z*°N | *z* ± 0.75°N | 1,700,000 m | (*z* − 41) × 1,000,000 + 200,000 m |

The math is closed form: the cone constant from the two standard parallels, the radius from the isometric latitude, and the easting and northing from the radius and the cone angle. IGN's NT/G 71 states the same algorithm as ALG0001, ALG0003, and ALG0054, and prints the resulting constants for Lambert-93.

Beside the coordinates the tool returns the **grid convergence**, the angle from true north to grid north, and the **point scale factor**.

## Equations

- `n = (ln m₁ − ln m₂)/(ln t₁ − ln t₂)`, `F = m₁/(n·t₁ⁿ)`, with `m = cos φ/√(1 − e² sin² φ)` and `t = tan(π/4 − φ/2)/((1 − e sin φ)/(1 + e sin φ))^(e/2)`.
- `r = a·F·tⁿ`, `θ = n(λ − λ₀)`.
- `E = FE + r sin θ`, `N = FN + r_F − r cos θ`, with `r_F` the radius at the origin's latitude.
- Convergence `γ = θ`; point scale factor `k = r·n/(a·m)`.
- IGN's form, the second oracle: `X = Xs + c·exp(−n·L) sin θ`, `Y = Ys − c·exp(−n·L) cos θ`, with `L` the isometric latitude and, for Lambert-93, `n = 0.7256077650`, `c = 11,754,255.426 m`, `Xs = 700,000 m`, `Ys = 12,655,612.050 m`.

## Symbols and units

Latitude and longitude in degrees on RGF93, north and east positive; easting and northing in meters from the grid's false origin; convergence in degrees; the point scale factor dimensionless; the grid a name from the list.

## Domain

- The ten grids above; Lambert-93 is the default.
- Outside a grid's published area the arithmetic still holds and the answer carries `OUTSIDE_AREA_OF_USE`, naming the area.
- The South Pole is at infinity on a cone that opens north and is refused with `OUT_OF_DOMAIN`.
- Heights are untouched; RAF20 is a separate asset.

## Approximations

None in the projection, which is closed form. The coordinates are taken to be on RGF93, which is the `REALIZATION_ASSUMED` warning: its realizations v1, v2, and v2b differ by centimeters and share these grids, and positions on the older NTF datum are another ellipsoid and another set of Lambert zones.

## Worked example

- sourcePublisher: IOGP and IGN, through PROJ
- sourceTitle: RGF93 v1 / Lambert-93, registered as EPSG 2154
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The grid's own parameters, applied by PROJ to the Eiffel Tower at 48.858370°N 2.294481°E on RGF93
- independent: yes
- inputs: lat 48.85837, lon 2.294481
- outputs: E 648235.878 m, N 6862268.358 m
- tolerance: 0.001 m
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/projections.rs` `lambert93_round_trips_every_grid_and_pins_the_false_origins`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_lambert93.py` builds 28 of the 33 vectors in `core/vectors/geodesy.projection.lambert93-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, applying each grid's own EPSG record (checked against `projinfo` on every run): 14 places from Brest to Strasbourg and Lille to Ajaccio, each on Lambert-93 and on its CC zone, so all nine zones are covered, within a millimeter.
- `tools/vectors/gen_lambert93.py` builds the other 5 from IGN's printed Lambert-93 constants put through NT/G 71's ALG0003 in Python, an independent implementation from the issuer's own numbers: the coordinates within a millimeter, and the convergence and point scale factor from the same constants. The printed constants agree with PROJ to 0.15 mm across France.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `lambert93_round_trips_every_grid_and_pins_the_false_origins`: on all ten grids the false origin gives the false easting and northing, zero convergence, and a scale just under 1; a point north-east of it lands north-east, with a positive convergence of about n times the longitude difference, and round-trips to a nanodegree; Lambert-93 is the default; and the Eiffel Tower matches PROJ to a tenth of a millimeter.
- `core/crates/gp-geodesy/tests/projections.rs` `lambert93_flags_a_point_outside_its_grid`: Marseille on CC50 answers with `OUTSIDE_AREA_OF_USE`, and the South Pole is refused.
