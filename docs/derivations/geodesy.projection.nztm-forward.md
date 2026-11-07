<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to NZTM2000 (`geodesy.projection.nztm-forward`)

## Method

New Zealand is drawn on one grid, and NZTM2000 is it: a single transverse Mercator zone, central meridian 173°E, stretched to cover a country 1,600 km long. That is unusual — most countries that long use several zones, or the UTM zones the grid's parameters otherwise borrow — and it is why the scale factor at the edges matters enough to be an output rather than a footnote.

The projection is EPSG method 9807 with the parameters LINZ states in LINZS25002 and EPSG registers as 2193: GRS 80, latitude of origin 0°, central meridian 173°E, scale 0.9996 on that meridian, false easting 1,600,000 m and false northing 10,000,000 m. The false northing is what keeps every New Zealand northing positive, since the country is south of the equator the northings are measured from.

The math is Krüger's series to sixth order in Karney's arrangement, the same series the generic transverse Mercator tool uses; this tool is that one with the parameters fixed and named. Beside the coordinates it returns the **grid convergence**, the angle from true north to grid north, and the **point scale factor**, grid distance over ground distance there. Those two are what a surveyor needs in order to use the grid at all: without the convergence a true bearing is not a grid bearing, and without the scale factor a measured distance is not a grid distance.

## Equations

- `E = FE + k₀·B₁·a·η`, `N = FN + k₀·B₁·a·ξ − N₀`, with ξ and η the series in the conformal latitude and the longitude from the central meridian, and `N₀` the northing the series gives at the latitude of origin.
- Convergence `γ = γ₀ + atan2(q, p)`, with p and q the series' derivatives; near enough `γ ≈ (λ − λ₀)·sin φ`, which the invariant test uses as the sanity check.
- Point scale factor `k = k₀·B₁·k′·|p + iq|`.
- Parameters: `φ₀ = 0`, `λ₀ = 173°E`, `k₀ = 0.9996`, `FE = 1,600,000 m`, `FN = 10,000,000 m`, GRS 80.

## Symbols and units

Latitude and longitude in degrees on NZGD2000, north and east positive, so New Zealand's latitudes are negative; easting and northing in meters from the false origin; convergence in degrees; the point scale factor dimensionless.

## Domain

- Any latitude and longitude the series reaches, which is 3,900 km from the central meridian. New Zealand and its shelf are well inside that.
- A latitude or longitude outside its range is refused with `INVALID_INPUT`.
- Heights are untouched: the projection is horizontal, and New Zealand's height datums are a separate step.

## Approximations

The coordinates are taken as NZGD2000 and nothing checks that they are. NZGD2000 was aligned with ITRF96 on 1 January 2000 and then held there while the ground kept moving, so a satellite's WGS 84 position is not an NZGD2000 one: LINZ puts the difference at 5 cm a year since 2000, 1.25 m in 2025, and closing it needs LINZ's deformation model, which this tool does not carry. That is the `REALIZATION_ASSUMED` warning, and it is a metre-scale caveat on a nanometre-scale projection.

One zone over a long country means the scale factor runs from 0.9996 on the central meridian to about 1.0012 at Invercargill and the far east, which is 1.2 m per kilometre: large enough that ignoring it is a survey error, which is why it is returned. NZTM2000 is also not what New Zealand cadastral surveys use — those use the meridional circuits, each with its own origin — nor the older New Zealand Map Grid, and neither is this tool.

## Worked example

- sourcePublisher: Land Information New Zealand, through PROJ
- sourceTitle: NZTM2000, defined by LINZS25002 Standard for New Zealand Geodetic Datum 2000 Projections, registered as EPSG 2193
- sourceEdition: LINZS25002 version 2 (24 July 2008); EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The projection's own parameters, applied by PROJ to the Beehive in Wellington at 41.278611°S 174.776944°E on NZGD2000
- independent: yes
- inputs: lat -41.278611, lon 174.776944
- outputs: E 1748815.784 m, N 5428791.056 m
- tolerance: 0.001 m against PROJ, and 1e-6 m against GeographicLib
- verifiedBy: golden vectors v001 and v017 and `core/crates/gp-geodesy/tests/projections.rs` `nztm_round_trips_and_pins_its_origin`
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_nztm.py` builds 16 of the 21 vectors in `core/vectors/geodesy.projection.nztm-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, with the parameters taken from PROJ's own rendering of EPSG 2193 rather than from this repository and checked against `projinfo` on every run: fifteen places from Cape Reinga to Bluff and from Invercargill to East Cape, within a millimetre, and the grid's own origin, where the easting must be the false easting, the northing the false northing, and the scale factor the declared 0.9996.
- `tools/vectors/gen_tm_convergence.py` builds the other 5 from GeographicLib's TransverseMercatorProj, a second reference and a different algorithm (the exact projection rather than the series): the convergence, the point scale factor, and the coordinates to a micrometre at four places and on the central meridian, where the convergence is zero. The tight coordinate tolerance is deliberate — projecting on WGS 84 instead of GRS 80 moves the Beehive's northing 0.12 mm, which passes a millimetre and fails this.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `nztm_round_trips_and_pins_its_origin`: the projection and its inverse return six places to within a nanodegree and report the same convergence and scale factor at each; every place lands inside the one zone's band; on the central meridian at the latitude of origin the coordinates are the false origin exactly, the convergence is zero and the scale factor is the declared 0.9996; the convergence changes sign across the central meridian and is about `(λ − λ₀)·sin φ` in degrees; and the Beehive's northing matches GeographicLib on GRS 80 to a micrometre, which is what distinguishes GRS 80 from WGS 84 here.
