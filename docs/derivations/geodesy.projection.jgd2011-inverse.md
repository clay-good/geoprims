<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Japan plane rectangular to latitude and longitude (`geodesy.projection.jgd2011-inverse`)

## Method

This is the forward tool read backwards: X and Y on one of Japan's nineteen plane rectangular systems, back to the JGD2011 latitude and longitude the projection started from. It is the same Krüger series in Karney's arrangement, run in the inverse direction with the same system parameters from MLIT Notice No. 9 of 2002, so the round trip closes to a nanodegree.

The system is an input, and it has to be. Every system puts its origin at X = 0, Y = 0, so the same pair of numbers is a real place in each of the nineteen, and they are hundreds of kilometers apart. The tool asks rather than guesses.

X is the northing and Y the easting, the reverse of most grids' order, and coordinates south or west of the origin are negative. The inputs are named for both, so a column read the wrong way round is easy to spot.

Beside the coordinates the tool reports the **grid convergence** and the **point scale factor** at the point it found. They come from projecting the answer forward again, so they describe the same point both tools describe.

## Equations

- The inverse of `Y = k₀·B₁·a·η`, `X = k₀·B₁·a·ξ − X₀`: from `ξ = (X + X₀)/(k₀·B₁·a)` and `η = Y/(k₀·B₁·a)` by the inverse series to the conformal latitude, then to the geodetic latitude by Newton's method.
- Parameters: the system's `φ₀` and `λ₀`, `k₀ = 0.9999`, no false easting or northing, GRS 80.

## Symbols and units

X (northing) and Y (easting) in meters from the system's origin, negative to the south and west; the system a whole number 1 to 19; latitude and longitude in degrees on JGD2011, north and east positive; convergence in degrees; the point scale factor dimensionless.

## Domain

- Systems 1 to 19. Any other is refused by the field's declared range, which names it.
- X and Y must be finite meters; a pair the inverse cannot place is refused on `/northing`.
- An answer outside JGD2011's published area carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

The answer is on JGD2011. X and Y read off a JGD2000 drawing give a JGD2000 position, which in eastern Japan differs from JGD2011 by up to about 5 m because of the 2011 Tohoku earthquake; that is the `REALIZATION_ASSUMED` warning. Tokyo-datum X and Y are on the Bessel ellipsoid and are not this grid.

## Worked example

- sourcePublisher: Ministry of Land, Infrastructure, Transport and Tourism of Japan and IOGP, through PROJ
- sourceTitle: JGD2011 / Japan Plane Rectangular CS IX, registered as EPSG 6677
- sourceEdition: MLIT Notice No. 9 of 2002, last amended 2010, read 2026-11-16; EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: System IX's own parameters, applied by PROJ in reverse to X -35363.2377449 m, Y -5992.9195704 m
- independent: yes
- inputs: zone 9, northing -35363.2377449 m, easting -5992.9195704 m
- outputs: lat 35.681236, lon 139.767125
- tolerance: 1e-9 degrees against PROJ, and 1e-11 degrees against GeographicLib
- verifiedBy: golden vectors v011 and v023 and `core/crates/gp-geodesy/tests/projections.rs` `jgd2011_round_trips_every_system_and_pins_the_notices_origins`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_jgd.py` builds 22 of the 27 vectors in `core/vectors/geodesy.projection.jgd2011-inverse.jsonl` from PROJ 9.9.0 through `cs2cs` in reverse, applying each system's own EPSG record (checked against `projinfo` and the notice's origins on every run): places in all nineteen systems read back within a nanodegree.
- `tools/vectors/gen_jgd.py` builds the other 5 from GeographicLib's TransverseMercatorProj, a second reference: the place to 1e-11 degrees with the convergence and the point scale factor beside it.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `jgd2011_round_trips_every_system_and_pins_the_notices_origins`: a point in each of the nineteen systems goes out through the forward tool and comes back through this one to within a nanodegree, with both reporting the same convergence and point scale factor.
- `core/crates/gp-geodesy/tests/projections.rs` `jgd2011_refuses_a_system_outside_the_nineteen_and_flags_a_point_off_japan`: systems outside 1 to 19 are refused on `/zone`.
- `tools/trust/example-pairs.mjs` carries the forward tool's worked example through this one and requires it to land back on the same point.
