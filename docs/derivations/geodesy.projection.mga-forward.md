<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to MGA2020 (`geodesy.projection.mga-forward`)

## Method

The Map Grid of Australia is the Universal Transverse Mercator system applied to Australian coordinates. Geoscience Australia states it plainly: UTM on the GRS80 ellipsoid, 60 zones six degrees of longitude wide with zone 1 beginning at 177° west, a central meridian down the middle of each, a central scale factor of 0.9996, a false easting of 500,000 m, and a false northing of 10,000,000 m in the southern hemisphere. Fourteen of those zones, 46 through 59, cover Australia and its external territories, and EPSG registers each as its own CRS: 7846 to 7859 on GDA2020.

So the projection is EPSG method 9807 with UTM's parameters and the zone's own central meridian, `λ₀ = 6n − 183`. The math is Krüger's series to sixth order in Karney's arrangement, the same series the generic transverse Mercator tool uses; this tool is that one with the zone's parameters fixed and named.

Two things make it a tool of its own rather than a call to the UTM tool. The first is the zone: MGA coordinates do not carry theirs, so the tool picks the zone the longitude falls in, and takes a zone you name when a job runs across a boundary. The second is the datum, and it is the part that trips people. **MGA94 and MGA2020 are the same projection.** Their EPSG records render to identical PROJ strings, and the same latitude and longitude gives the same easting and northing under either name. What differs is the datum the latitude and longitude are on: GDA94 and GDA2020 place the same ground mark about 1.8 m apart. A GDA94 position put through this tool is MGA94, whatever you call it. Moving between the two is a datum step, `geodesy.datum.gda2020`, not a projection.

Beside the coordinates the tool returns the **grid convergence**, the angle from true north to grid north, and the **point scale factor**, grid distance over ground distance there. Those two are what a surveyor needs in order to use the grid at all: without the convergence a true bearing is not a grid bearing, and without the scale factor a measured distance is not a grid distance.

## Equations

- Zone from longitude: `n = ⌊(λ + 180) mod 360 ÷ 6⌋ + 1`; central meridian `λ₀ = 6n − 183`.
- `E = FE + k₀·B₁·a·η`, `N = FN + k₀·B₁·a·ξ − N₀`, with ξ and η the series in the conformal latitude and the longitude from the central meridian, and `N₀` the northing the series gives at the latitude of origin, which for MGA is zero at the equator.
- Convergence `γ = γ₀ + atan2(q, p)`, with p and q the series' derivatives; near enough `γ ≈ (λ − λ₀)·sin φ`, which the invariant test uses as the sanity check.
- Point scale factor `k = k₀·B₁·k′·|p + iq|`.
- Parameters: `φ₀ = 0`, `k₀ = 0.9996`, `FE = 500,000 m`, `FN = 10,000,000 m`, GRS 80.

## Symbols and units

Latitude and longitude in degrees on GDA2020 (or GDA94 when MGA94 is asked for), north and east positive, so Australia's latitudes are negative; easting and northing in meters from the zone's false origin; convergence in degrees; the point scale factor dimensionless; the zone a whole number 46 to 59.

## Domain

- Zones 46 to 59, the ones EPSG registers for Australia. A longitude outside them is refused with `OUT_OF_DOMAIN` naming the range, rather than projected onto a zone that is not an MGA zone.
- A zone you name outside 46 to 59 is refused by the field's own declared range, which names it too.
- Outside the published Australian extent — 90°E to 174°E, 60°S to 8°S — the arithmetic still holds and the answer carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched: the projection is horizontal, and AUSGeoid2020 is a separate asset.

## Approximations

The coordinates are taken to be on the datum named and nothing checks that they are. That is the `REALIZATION_ASSUMED` warning, and it is the metre-scale caveat on a nanometre-scale projection: a GDA94 latitude and longitude run as MGA2020 is out by the 1.8 m between the datums, and no amount of precision in the series recovers it.

Inside its own zone a point stays within three degrees of the central meridian, where the point scale factor runs from 0.9996 on the meridian to about 1.0010 at the edge — Janssen's figure, and the reason the factor is returned rather than assumed. On a forced neighbouring zone it goes further still, which is what `NONSTANDARD_ZONE` is about.

The tool applies the conformal transformation's projection only. It does not apply the conformal-plus-distortion grid (EPSG 8447), which carries the distortion the seven-parameter conformal step leaves behind, and it does not convert ellipsoidal heights to AHD.

## Worked example

- sourcePublisher: Geoscience Australia and IOGP, through PROJ
- sourceTitle: Map Grid of Australia 2020, GDA2020 / MGA zone 56, registered as EPSG 7856
- sourceEdition: Geoscience Australia's datums and projections guidance, read 2026-11-14; EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The zone's own parameters, applied by PROJ to the Sydney Opera House at 33.858611°S 151.214167°E on GDA2020
- independent: yes
- inputs: lat -33.858611, lon 151.214167
- outputs: zone 56, E 334799.233 m, N 6252086.100 m
- tolerance: 0.001 m against PROJ, and 1e-6 m against GeographicLib
- verifiedBy: golden vectors v001 and v018 and `core/crates/gp-geodesy/tests/projections.rs` `mga_rounds_trips_picks_its_zone_and_pins_its_origins`
- verifiedOn: 2026-11-14

## Differential tests

- `tools/vectors/gen_mga.py` builds 21 of the 27 vectors in `core/vectors/geodesy.projection.mga-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, with each zone's parameters taken from PROJ's own rendering of its EPSG record rather than from this repository and checked against `projinfo` on every run, for all fourteen zones and both grids: seventeen places from the Cocos Islands to Norfolk Island and from Darwin to Hobart within a millimetre; the three zone origins, where the easting must be the false easting, the northing the false northing, the convergence zero and the scale factor the declared 0.9996; one row that puts the same coordinates through EPSG:28356 (GDA94 / MGA zone 56) and requires the identical easting and northing, because MGA94 and MGA2020 differ in datum and not in projection; and one forced-zone row that requires `NONSTANDARD_ZONE`.
- `tools/vectors/gen_mga.py` builds the other 5 from GeographicLib's TransverseMercatorProj, a second reference and a different algorithm (the exact projection rather than the series): the convergence, the point scale factor, and the coordinates to a micrometre at four places and on zone 55's central meridian, where the convergence is zero. The tight coordinate tolerance is deliberate — projecting on WGS 84 instead of GRS 80 moves a northing by well under the millimetre the PROJ rows allow, and that silent ellipsoid fallback is exactly what shipped undetected on the Irish grid.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `mga_rounds_trips_picks_its_zone_and_pins_its_origins`: eight places round-trip through the inverse to within a nanodegree and report the same convergence and scale factor at each; the zone the tool picks is the one the longitude falls in and its central meridian is `6n − 183`; every place lands inside its zone's easting band with a scale factor between 0.9996 and 1.0010; all fourteen zone origins give the false origin exactly, zero convergence and the declared 0.9996; the convergence changes sign across a central meridian and is about `(λ − λ₀)·sin φ` in degrees; the Opera House's northing matches GeographicLib on GRS 80 to a micrometre, which is what distinguishes GRS 80 from WGS 84 here; and MGA94 and MGA2020 return identical coordinates while naming different EPSG codes, with MGA94 naming none outside zones 48 to 58.
- `core/crates/gp-geodesy/tests/projections.rs` `mga_refuses_a_zone_outside_australia_and_flags_a_forced_one`: zones outside 46 to 59 are refused with the range named; a longitude outside the Australian zones is refused rather than quietly projected; a forced neighbouring zone warns `NONSTANDARD_ZONE` and its scale factor leaves the in-zone band, which is the cost the warning is about; and a point outside the published extent still answers but carries `OUTSIDE_AREA_OF_USE`.
