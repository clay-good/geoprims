<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# MGA2020 to latitude and longitude (`geodesy.projection.mga-inverse`)

## Method

This is the forward tool read backwards: an easting and northing on a Map Grid of Australia zone, back to the latitude and longitude the projection started from. It is the same Krüger series in Karney's arrangement, run in the inverse direction with the same zone parameters — not a separate approximation of the inverse, which is why the round trip closes to a nanodegree rather than to whatever a fitted inverse happens to manage.

The zone is an input, and it has to be. An MGA easting and northing carries no record of which of the fourteen zones it belongs to: 334,799 m east and 6,252,086 m north is a real place in every one of them, and they are hundreds of kilometres apart. So the tool asks rather than guesses.

As on the forward side, the grid named selects the datum of the answer, not the arithmetic. MGA94 and MGA2020 read the same numbers to the same latitude and longitude; the answer is on GDA94 in the first case and GDA2020 in the second, and those two place the same ground mark about 1.8 m apart. The step between them is `geodesy.datum.gda2020`.

Beside the coordinates the tool reports the **grid convergence** and the **point scale factor** at the point it found, which is what turns a grid bearing back into a true one and a grid distance back into a ground one. They come from projecting the answer forward again, so they describe the same point both tools describe.

## Equations

- `ξ = (N − FN + N₀)/(k₀·B₁·a)`, `η = (E − FE)/(k₀·B₁·a)`, then the inverse series in ξ and η to the conformal latitude and the longitude from the central meridian.
- `φ` from the conformal latitude by the series; `λ = λ₀ + Δλ`, with `λ₀ = 6n − 183` for zone n.
- Convergence and point scale factor from the forward projection of the recovered point, so `γ ≈ (λ − λ₀)·sin φ` and `k = k₀·B₁·k′·|p + iq|`.
- Parameters: `φ₀ = 0`, `k₀ = 0.9996`, `FE = 500,000 m`, `FN = 10,000,000 m`, GRS 80.

## Symbols and units

Zone a whole number 46 to 59; easting and northing in meters from the zone's false origin; latitude and longitude in degrees on the datum named, north and east positive, so Australia's latitudes come back negative; convergence in degrees; the point scale factor dimensionless.

## Domain

- Zones 46 to 59, the ones EPSG registers for Australia; the field's declared range refuses anything outside, naming it.
- Any easting and northing the inverse series reaches, which is 3,900 km from the central meridian — far wider than a six-degree zone.
- An easting and northing that is not a place on the grid is refused with `INVALID_INPUT`.
- Outside the published Australian extent the answer carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

The answer is on the datum named and nothing checks that the coordinates were. Read a set of MGA94 numbers as MGA2020 and the latitude and longitude come back 1.8 m from where the mark is — the `REALIZATION_ASSUMED` warning, and a metre-scale caveat on a nanometre-scale inverse.

Naming the wrong zone is the other way to be wrong by a large amount, and the tool cannot detect it: every zone reads every easting and northing to a perfectly ordinary place. The answer's longitude is the check — if it is nowhere near the job, the zone was wrong.

## Worked example

- sourcePublisher: Geoscience Australia and IOGP, through PROJ
- sourceTitle: Map Grid of Australia 2020, GDA2020 / MGA zone 56, registered as EPSG 7856
- sourceEdition: Geoscience Australia's datums and projections guidance, read 2026-11-14; EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The zone's own parameters, applied by PROJ in reverse to E 334799.2334757 m N 6252086.0999992 m on MGA zone 56
- independent: yes
- inputs: zone 56, easting 334799.2334757 m, northing 6252086.0999992 m
- outputs: lat -33.858611, lon 151.214167
- tolerance: 1e-9 degrees against PROJ, and 1e-11 degrees against GeographicLib
- verifiedBy: golden vectors v001 and v018 and `core/crates/gp-geodesy/tests/projections.rs` `mga_rounds_trips_picks_its_zone_and_pins_its_origins`
- verifiedOn: 2026-11-14

## Differential tests

- `tools/vectors/gen_mga.py` builds 18 of the 23 vectors in `core/vectors/geodesy.projection.mga-inverse.jsonl` from PROJ 9.9.0 through `cs2cs` in reverse, with each zone's parameters taken from PROJ's own rendering of its EPSG record and checked against `projinfo` on every run: seventeen places across all fourteen zones read back within a nanodegree, and one row that reads the same numbers through EPSG:28356 (GDA94 / MGA zone 56) and requires the same latitude and longitude, because the two grids differ in datum and not in projection.
- `tools/vectors/gen_mga.py` builds the other 5 from GeographicLib's TransverseMercatorProj, a second reference and a different algorithm: the place to 1e-11 degrees with the convergence and the point scale factor beside it, at four places and on zone 55's central meridian, where the convergence is zero. Nothing had pinned a convergence before the Irish grid shipped one 57.3 times too large, which is why these rows exist.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `mga_rounds_trips_picks_its_zone_and_pins_its_origins`: eight places go out through the forward tool and come back through this one to within a nanodegree, with both reporting the same convergence and the same point scale factor at each; and MGA94 and MGA2020 read identical numbers to identical coordinates while naming different EPSG codes.
- `core/crates/gp-geodesy/tests/projections.rs` `mga_refuses_a_zone_outside_australia_and_flags_a_forced_one`: a zone outside 46 to 59 is refused with the range named, and a point outside the published extent answers with `OUTSIDE_AREA_OF_USE`.
- `tools/trust/example-pairs.mjs` carries the forward tool's worked example through this one and requires it to land back on the same point, which is the check that would have caught the Irish pages naming a place 1.1 m from where they said it was.
