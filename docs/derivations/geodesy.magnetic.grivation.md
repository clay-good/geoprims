<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Grid variation (`geodesy.magnetic.grivation`)

## Method

There are three norths and this tool is about the two nobody agrees on. True north is the meridian. Magnetic north is where the compass points, a declination D away. **Grid north** is the direction the map's grid runs, which is only the meridian at the zone's central meridian and elsewhere differs by the grid convergence γ.

Grivation — grid variation — is the angle from grid north to magnetic north:

**G = D − γ**, both east positive.

It matters where the two corrections are comparable, which is anywhere far from a central meridian and above all near the poles, where a UTM zone is narrow on the ground and the convergence grows quickly. A military map or a polar navigation system works in grid bearings, and G is what turns a compass reading into one.

The declination comes from the magnetic model. The convergence comes from the projection: for UTM it is the bearing of grid north from true north at that point in that zone, and the zone is chosen by the same rules `geodesy.utm.zone` uses, exceptions and all. Beyond the UTM latitudes the grid is UPS and the convergence is the longitude itself.

## Equations

- G = D − γ, east positive for both.
- UTM convergence: γ = arctan(tan Δλ · sin φ) to first order, with Δλ the longitude from the zone's central meridian; the exact ellipsoidal value is what the projection reports.
- UPS convergence: γ = λ in the north, −λ in the south.
- Grid bearing = magnetic bearing + G; magnetic bearing = grid bearing − G.

## Symbols and units

`lat`, `lon` and `date` place the point; `grid` picks UTM or UPS, and `model` and `height` the magnetic model. Out come `grivation`, its text form, the `declination` and `convergence` it was made from, the `grid_used`, and the sign convention in words.

## Domain

Any place and date the magnetic model covers. Within the UTM latitudes the grid is a UTM zone; beyond them it is UPS.

## Approximations

The convergence is exact for the projection. The declination is not: the WMM carries about half a degree of uncertainty and drifts year to year, so the grivation inherits that and no more.

## Worked example

- sourcePublisher: pygeomag contributors; PROJ contributors
- sourceTitle: pygeomag, an independent implementation of the World Magnetic Model; PROJ's `Proj.get_factors`, which reports a projection's meridian convergence
- sourceEdition: WMM2025; PROJ 9.3.0
- sourceLocator: G = D − γ, east positive; the meridian convergence of the UTM zone at the point
- independent: yes
- inputs: 25 points from 78° north to 60° south, including a Norway and a Svalbard point where the zone exception moves the convergence, and places on both sides of the agonic line
- outputs: the grivation, the declination and the convergence
- tolerance: 1e-4° on the declination and grivation, 1e-6° on the convergence
- verifiedBy: golden vectors v001 to v025, run by the core on every build
- verifiedOn: 2026-09-23

Both halves have their own outside reference and neither is the core. At Pittsburgh on 2026-07-02 pygeomag gives a declination of −9.2389527 against the core's −9.2389552, and PROJ gives a convergence of 0.6603064307551 against the core's 0.6603064306784 — three millionths of a degree on one and eight hundred-billionths on the other. The grivation is then the difference of the two, computed in the generator rather than read back.

## Differential tests

- `tools/vectors/gen_magnetic_more.py`: 16 of the 25 vectors, declination from pygeomag and convergence from PROJ
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.magnetic.grivation.jsonl`: 25 points across both hemispheres and both grids

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `grivation_invariants`: the grivation is exactly the declination minus the convergence, at every point tried, which is the definition and the one thing that can be checked without a second model; the declination agrees with `geodesy.magnetic.declination` at the same place and date, so the two tools do not carry separate copies of the model; on a zone's central meridian the convergence is zero and the grivation is the declination itself; the convergence changes sign either side of that meridian, positive to the east; it grows with latitude for the same offset from the meridian, since it goes as sin φ; and the grid named in the answer is the one `geodesy.utm.zone` would pick for the point
