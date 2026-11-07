<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# NZTM2000 to latitude and longitude (`geodesy.projection.nztm-inverse`)

## Method

Reading an easting and northing off a New Zealand map, title plan, or council dataset and getting back a latitude and longitude is the inverse of EPSG method 9807 with the parameters LINZS25002 states and EPSG registers as 2193: GRS 80, latitude of origin 0°, central meridian 173°E, scale 0.9996, false easting 1,600,000 m, false northing 10,000,000 m.

The false origin comes off first — 1,600 km of easting and 10,000 km of northing — and what is left goes through Krüger's series backwards in Karney's arrangement, which inverts the forward series term by term rather than iterating, so it is the forward projection's own inverse and not a separate approximation. The round trip returns a place to within a nanodegree.

The **grid convergence** and the **point scale factor** are then read at the place the coordinates landed on, by running the forward projection there. They come back beside the latitude and longitude because that is what turns a grid bearing taken off the plan back into a true bearing, and a grid distance back into a ground one.

## Equations

- `ξ = (N − FN + N₀)/(k₀·B₁·a)`, `η = (E − FE)/(k₀·B₁·a)`, with `N₀` the northing the series gives at the latitude of origin.
- The series inverted gives the conformal latitude and the longitude from the central meridian; `λ = λ₀ + Δλ`.
- Convergence and point scale factor at the answer, from the forward series there: `γ = γ₀ + atan2(q, p)` and `k = k₀·B₁·k′·|p + iq|`.

## Symbols and units

Easting and northing in meters from the false origin (any length unit is accepted); latitude and longitude in degrees on NZGD2000, north and east positive, so New Zealand's latitudes come back negative; convergence in degrees; the point scale factor dimensionless.

## Domain

- Any easting and northing the series reaches, which is 3,900 km from the central meridian; past that, or where the inverse does not give a real latitude, `INVALID_INPUT` says the coordinates are not a place on the grid.
- A non-finite easting or northing is refused by name.
- Heights are untouched.

## Approximations

The answer is on NZGD2000, which is not a satellite's WGS 84: LINZ puts the difference at 5 cm a year since 2000, 1.25 m in 2025, because NZGD2000 was aligned with ITRF96 at epoch 2000.0 and held there while the ground moved. Closing that needs LINZ's deformation model, which this tool does not carry, and it is the `REALIZATION_ASSUMED` warning.

Nothing here can tell NZTM2000 coordinates from the other New Zealand grids: a meridional circuit easting or a New Zealand Map Grid northing fed in will be read as NZTM2000 and will come back as a place, the wrong one. The one guard the grid gives for free is its 10,000 km false northing, which puts an NZTM2000 northing far outside the range any of the other grids uses.

## Worked example

- sourcePublisher: Land Information New Zealand, through PROJ
- sourceTitle: NZTM2000, defined by LINZS25002 Standard for New Zealand Geodetic Datum 2000 Projections, registered as EPSG 2193
- sourceEdition: LINZS25002 version 2 (24 July 2008); EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The projection's own parameters, applied by PROJ in reverse to E 1748815.784172 N 5428791.056045, the Beehive in Wellington
- independent: yes
- inputs: easting 1748815.784172 m, northing 5428791.056045 m
- outputs: 41.278611°S, 174.776944°E on NZGD2000
- tolerance: 1e-9° against PROJ, and 1e-11° against GeographicLib
- verifiedBy: golden vectors v001 and v017 and `core/crates/gp-geodesy/tests/projections.rs` `nztm_round_trips_and_pins_its_origin`
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_nztm.py` builds 16 of the 21 vectors in `core/vectors/geodesy.projection.nztm-inverse.jsonl` from PROJ 9.9.0 through `cs2cs`, with the parameters taken from PROJ's own rendering of EPSG 2193 and checked against `projinfo` on every run: fifteen places read back within a nanodegree, and the grid's own false origin, which must read back as the latitude of origin on the central meridian.
- `tools/vectors/gen_tm_convergence.py` builds the other 5 from GeographicLib's TransverseMercatorProj, a second reference running the exact projection rather than the series: the place to 1e-11°, the convergence, and the point scale factor at four places and on the central meridian, where the convergence is zero.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `nztm_round_trips_and_pins_its_origin`: the projection and its inverse return six places to within a nanodegree and report the same convergence and point scale factor at each, so the two tools are checked against one another as well as against the references; the grid's false origin reads back as 0° on the central meridian; and the convergence is about `(λ − λ₀)·sin φ` in degrees, which is the check that the forward and inverse tools agreeing could not supply.
