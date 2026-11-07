<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Irish Transverse Mercator to latitude and longitude (`geodesy.grid-ref.itm-inverse`)

## Method

Irish Transverse Mercator is the grid Ireland and Northern Ireland have used since 2001, north and south of the border, replacing the older Irish Grid. It is a single transverse Mercator zone covering the island, registered as EPSG 2157: the GRS 80 ellipsoid, a latitude of origin of 53°30′N, a central meridian of 8°W, a scale factor of 0.999 820 on that meridian, and a false origin 600,000 m east and 750,000 m north so that the whole island has positive coordinates well clear of zero.

Because ITM is defined on ETRS89 and ETRS89 is fixed to the European plate, there is no datum transformation in this tool and nothing to be approximate about in the projection itself. What there is to say is the frame: a satellite position is on WGS 84 or ITRF, which has moved away from ETRS89 by the plate's motion since 1989 — about half a metre today, growing about 2.5 cm a year. The result says so and carries `REALIZATION_ASSUMED`.

The projection is Karney's series, the same one the UTM and State Plane tools use, so the convergence and the point scale factor come out of it for free and are reported: those are what turn a true bearing into a grid bearing and a ground distance into a grid distance.

## Equations

- Transverse Mercator inverse on GRS 80 with the parameters above (Karney 2011's series), taking easting and northing back to latitude and longitude, with the convergence and point scale factor computed at the point found.
- `x = E − 600000`, `y = N − 750000 + y₀`, with `y₀` the projected northing of 53°30′N on the central meridian, then the inverse series.

## Symbols and units

Latitude and longitude in degrees on ETRS89; easting and northing in meters from ITM's false origin; convergence in degrees; the point scale factor dimensionless.

## Domain

Any latitude and longitude the projection is defined for; the series holds to the nanometre within 3,900 km of the central meridian, which is all of Ireland and far beyond. The grid is defined for the island of Ireland, and coordinates far outside it are arithmetic rather than a national grid reference.

## Approximations

None in the projection beyond the series, which is exact to a nanometre here. The approximation worth stating is the frame: ETRS89 against WGS 84 or ITRF, about half a metre today. Heights are untouched. This is not the Irish Grid (TM75), a different projection on a different datum: the two are hundreds of metres apart.

## Worked example

- sourcePublisher: IOGP, through PROJ
- sourceTitle: EPSG Geodetic Parameter Dataset, operation 2157 (IRENET95 / Irish Transverse Mercator)
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The projection's own parameters, applied by PROJ in reverse to E 715830.018671 N 734698.346164, the Spire of Dublin
- independent: yes
- inputs: easting 715830.018671 m, northing 734698.346164 m
- outputs: 53.349806°N, 6.260247°W on ETRS89
- tolerance: 1e-9°
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/gridref.rs` `itm_round_trips_and_pins_its_origin` Then: the convergence at five places is about -(lambda - lambda0) sin(phi) and in degrees, which is what a forward and inverse tool that agreed with each other could not establish.
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_itm.py` builds 13 of the 18 vectors in `core/vectors/geodesy.grid-ref.itm-forward.jsonl` from PROJ 9.9.0 through `cs2cs` applying EPSG:2157 itself, so the parameters come from the EPSG dataset and not from this repository: twelve places from Mizen Head to Malin Head, north and south of the border, within a nanodegree, and the grid's own false origin, which must read back as the latitude of origin on the central meridian. A second set, the 5 vectors from `tools/vectors/gen_tm_convergence.py`, comes from GeographicLib's TransverseMercatorProj instead, and pins what PROJ's rows did not: the grid convergence, the point scale factor, and the coordinates to a micrometre rather than a millimetre. Those three went unchecked for as long as the tool shipped, and all three were wrong -- the convergence by a factor of 57.3, and the coordinates by the ellipsoid.

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `itm_round_trips_and_pins_its_origin`: the projection and its inverse return every test point to within a nanodegree; on the central meridian the convergence is zero and the scale factor is the projection's own; and an ITM easting read as if it were an Irish Grid easting lands hundreds of kilometres away, which is why the two are separate tools.
