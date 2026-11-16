<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Lambert-93 to latitude and longitude (`geodesy.projection.lambert93-inverse`)

## Method

This is the forward tool read backwards: an easting and northing on Lambert-93 or one of the CC42 to CC50 conic zones, back to the RGF93 latitude and longitude. It is the inverse of EPSG method 9802 on GRS 80 with the grid's parameters (see the forward tool's note for the table): the radius and cone angle from the easting and northing, the isometric latitude from the radius, and the latitude from the isometric latitude by iteration, as IGN's NT/G 71 does it in ALG0004 and ALG0002.

The grid is an input. Lambert-93 is the default, and a CC zone is named on the plan.

Beside the coordinates the tool reports the **grid convergence** and the **point scale factor** at the point it found, from projecting the answer forward again.

## Equations

- `r = sign(n)·√((E − FE)² + (r_F − (N − FN))²)`, `θ = atan2(E − FE, r_F − (N − FN))`.
- `t = (r/(a·F))^(1/n)`; `φ` from `t` by iteration; `λ = θ/n + λ₀`.
- Parameters as in the forward tool.

## Symbols and units

Easting and northing in meters from the grid's false origin; the grid a name from the list; latitude and longitude in degrees on RGF93; convergence in degrees; the point scale factor dimensionless.

## Domain

- The ten grids; Lambert-93 is the default.
- The easting and northing must be finite meters; a pair the inverse cannot place is refused on `/northing`.
- An answer outside the grid's published area carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

The latitude is recovered by iteration to the limit of double precision. The answer is on RGF93; that is the `REALIZATION_ASSUMED` warning.

## Worked example

- sourcePublisher: IOGP and IGN, through PROJ
- sourceTitle: RGF93 v1 / Lambert-93, registered as EPSG 2154
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The grid's own parameters, applied by PROJ in reverse to E 648235.8777 m N 6862268.3583 m
- independent: yes
- inputs: easting 648235.8777 m, northing 6862268.3583 m
- outputs: lat 48.85837, lon 2.294481
- tolerance: 1e-9 degrees
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/projections.rs` `lambert93_round_trips_every_grid_and_pins_the_false_origins`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_lambert93.py` builds all 28 vectors in `core/vectors/geodesy.projection.lambert93-inverse.jsonl` from PROJ 9.9.0 through `cs2cs` in reverse, applying each grid's own EPSG record: 14 places, each read back from Lambert-93 and from its CC zone, within a nanodegree.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `lambert93_round_trips_every_grid_and_pins_the_false_origins`: a point on each of the ten grids goes out through the forward tool and comes back through this one to a nanodegree, with the same convergence at both ends.
- `tools/trust/example-pairs.mjs` carries the forward tool's worked example through this one and requires it to land back on the same point.
