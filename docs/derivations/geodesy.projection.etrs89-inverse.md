<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# LAEA Europe to latitude and longitude (`geodesy.projection.etrs89-inverse`)

## Method

This is the forward tool read backwards: an easting and northing on LAEA Europe (EPSG 3035) or LCC Europe (EPSG 3034) back to the ETRS89 latitude and longitude. For LAEA it follows Guidance Note 7-2 section 3.3.2 to the authalic latitude `β′`, then recovers the latitude from `q = q_P sin β′` by Newton's method on `q(φ)`. The note instead gives a series in e², e⁴, and e⁶, whose truncation is near a centimeter. For LCC it is the closed-form inverse with the latitude from the isometric latitude by iteration.

Beside the coordinates the tool reports the convergence and the two scales at the point it found, from projecting the answer forward again.

## Equations

- `ρ = √(((E − FE)/D)² + (D(N − FN))²)`, `C = 2 asin(ρ/(2R_q))`.
- `β′ = asin(cos C sin β₀ + D(N − FN) sin C cos β₀/ρ)`.
- `λ = λ₀ + atan2((E − FE) sin C, D·ρ cos β₀ cos C − D²(N − FN) sin β₀ sin C)`.
- `φ` from `q(φ) = q_P sin β′` by Newton's method, `dq/dφ = 2(1 − e²) cos φ/(1 − e² sin² φ)²`.

## Symbols and units

Easting and northing in meters from the grid's false origin; latitude and longitude in degrees on ETRS89; convergence in degrees; the two scales dimensionless.

## Domain

- LAEA Europe (default) or LCC Europe.
- A point beyond the circle that LAEA maps the far side of the Earth to is refused on `/northing`.
- An answer outside the grids' published area carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

None beyond double precision: Newton's method converges to 1e-15 radians. The answer is on ETRS89; that is the `REALIZATION_ASSUMED` warning.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)
- sourceEdition: Revised September 2019
- sourceLocator: Section 3.3.2, Lambert Azimuthal Equal Area, the ETRS89 / ETRS-LAEA reverse example
- independent: yes
- inputs: easting 3962799.45 m, northing 2999718.85 m
- outputs: lat 50, lon 5
- tolerance: 1e-7 degrees, the half centimeter of the printed values
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/projections.rs` `etrs89_grids_round_trip_and_pin_their_origins`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_etrs89.py` builds 34 of the 35 vectors in `core/vectors/geodesy.projection.etrs89-inverse.jsonl` from PROJ 9.9.0 through `cs2cs` in reverse, applying EPSG 3035 and 3034: 17 places on each grid within a nanodegree.
- The first row is Guidance Note 7-2's reverse example as printed.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `etrs89_grids_round_trip_and_pin_their_origins`: four places on each grid go out through the forward tool and come back through this one to a nanodegree, and a point far beyond the antipode's circle is refused on `/northing`.
- `tools/trust/example-pairs.mjs` carries the forward tool's worked example through this one and requires it to land back on the same point.
