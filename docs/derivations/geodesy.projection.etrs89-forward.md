<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to LAEA Europe (`geodesy.projection.etrs89-forward`)

## Method

Two grids cover Europe on ETRS89, both centred on 52°N 10°E on GRS 80:

| Grid | EPSG | Method | What it keeps | False origin |
|---|---|---|---|---|
| LAEA Europe | 3035 | Lambert Azimuthal Equal Area (9820), oblique | areas | 4,321,000 m E, 3,210,000 m N |
| LCC Europe | 3034 | Lambert Conic Conformal 2SP (9802), parallels 35°N and 65°N | angles and shapes | 4,000,000 m E, 2,800,000 m N |

LAEA Europe is the grid of European statistics: Eurostat's population grid, the INSPIRE grid cells, and the European Environment Agency's reference grid are squares in it, because a square kilometer on the grid is a square kilometer on the ground. LCC Europe is the conformal map of the continent.

LAEA follows Guidance Note 7-2 section 3.3.2: the latitude to the authalic latitude `β` through `q`, then the easting and northing in closed form. LCC is the closed form the Lambert-93 tool uses. Because LAEA is not conformal, the scale along the meridian and the scale along the parallel differ, so the tool returns both, with the convergence. For LAEA they are measured from the projection itself, from the images of 100 m chords each way with Richardson extrapolation, the same way the Hotine tools measure theirs.

## Equations

- `q(φ) = (1 − e²)[sin φ/(1 − e² sin² φ) − ln((1 − e sin φ)/(1 + e sin φ))/(2e)]`, `q_P = q(90°)`, `β = asin(q/q_P)`.
- `R_q = a√(q_P/2)`, `D = a·m₀/(R_q cos β₀)`, `B = R_q√(2/(1 + sin β₀ sin β + cos β₀ cos β cos(λ − λ₀)))`.
- `E = FE + B·D cos β sin(λ − λ₀)`, `N = FN + (B/D)(cos β₀ sin β − sin β₀ cos β cos(λ − λ₀))`.
- LCC as in `geodesy.projection.lambert93-forward`, with `φ₀ = 52°`, `λ₀ = 10°`, `φ₁ = 35°`, `φ₂ = 65°`.

## Symbols and units

Latitude and longitude in degrees on ETRS89, north and east positive; easting and northing in meters from the grid's false origin; convergence in degrees; the two scales dimensionless.

## Domain

- LAEA Europe (default) or LCC Europe.
- The far side of the Earth from 52°N 10°E maps to a circle on LAEA, not a point, and is refused with `OUT_OF_DOMAIN`; so is the South Pole on LCC.
- Outside EPSG's area for the grids, about 36°W to 45°E and 25°N to 85°N, the answer carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

None in the coordinates, which are closed form. The LAEA convergence and scales come from 100 m chords, good to about 1e-10. The coordinates are taken to be on ETRS89, the `REALIZATION_ASSUMED` warning: a WGS 84 or ITRF position differs from it by about 84 cm at Dublin in 2026, which `geodesy.datum.etrf` handles.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)
- sourceEdition: Revised September 2019
- sourceLocator: Section 3.3.2, Lambert Azimuthal Equal Area, the ETRS89 / ETRS-LAEA example
- independent: yes
- inputs: lat 50, lon 5
- outputs: E 3962799.45 m, N 2999718.85 m
- tolerance: 0.005 m, the half centimeter of the printed values
- verifiedBy: golden vector v001, `core/crates/gp-geo/src/proj.rs` `laea_guidance_note_example_and_round_trip`, and `core/crates/gp-geodesy/tests/projections.rs` `etrs89_grids_round_trip_and_pin_their_origins`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_etrs89.py` builds 34 of the 43 vectors in `core/vectors/geodesy.projection.etrs89-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, applying EPSG 3035 and 3034 (checked against `projinfo` on every run): 17 places from the Azores to Cyprus and Lisbon to Svalbard on each grid, within a millimeter.
- `tools/vectors/gen_etrs89.py` builds 8 from PROJ's `proj -V` factors on the same definitions: the convergence and the scales along the meridian and the parallel at four places on each grid, to 1e-8.
- The first row is Guidance Note 7-2's example as printed.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `etrs89_grids_round_trip_and_pin_their_origins`: on both grids the origin gives the false easting and northing with zero convergence; four places across Europe round-trip to a nanodegree; on LCC the two scales are equal; on LAEA one grows as the other shrinks, within 3 percent; LAEA is the default; and the antipode of the origin is refused both ways.
- `core/crates/gp-geo/src/proj.rs` `laea_guidance_note_example_and_round_trip`: the projection itself reproduces Guidance Note 7-2's example, round-trips five places from the origin to the southern hemisphere to 1e-10 degrees, and refuses the antipode's circle and anything beyond it.
