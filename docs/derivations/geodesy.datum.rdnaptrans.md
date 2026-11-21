<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GNSS position to RD and NAP by RDNAPTRANS2018 (`geodesy.datum.rdnaptrans`)

## Method

Dutch mapping is published in RD, a grid in metres east and north of a false origin near Amersfoort, and in NAP, a height datum near mean sea level at Amsterdam. RDNAPTRANS™2018 is the transformation NSGI publishes between those and ETRS89, so it is what defines the right RD coordinates for a GNSS position rather than an approximation of them.

The horizontal and vertical halves are independent, and NSGI offers two implementation variants of each. This tool takes variant 2 for the horizontal half and variant 1 for the vertical, the pair PROJ applies; NSGI states the two horizontal variants agree to well under 0.001 m inside the grid bounds.

1. **Onto real RD Bessel.** The rdtrans2018 grid gives a latitude and a longitude correction at nodes indexed by *real RD Bessel* coordinates, and adding them carries a point to ETRS89; in variant 2 those corrections hold both the 0.25 m distortion of the 1888–1928 triangulation and the ~100 m datum difference. Going the other way therefore means solving for the Bessel point whose corrections land on the ETRS89 position, which is a fixed-point iteration from the ETRS89 position itself. The grid is nested: a 0.1° parent over the Netherlands and its EEZ and a 0.02° by 0.0125° subgrid over the country, and the finer one is used where it reaches, as NSGI says.
2. **The RD map projection.** A double projection. First a Gauss conformal projection from Bessel 1841 onto a sphere of radius √(R<sub>M</sub>R<sub>N</sub>) at Amersfoort, then an oblique stereographic projection of that sphere onto the plane at scale 0.9999079, with the false origin 155,000 m east and 463,000 m north. This is EPSG method 9809, written generically in `gp_geo::proj::Sterea`.
3. **The height.** NLGEO2018 gives the quasi-geoid height N above the ETRS89 ellipsoid at nodes indexed by ETRS89 coordinates, bilinearly, and the NAP height is H = h − N.

The grid convergence and the point scale factor are reported with the answer, taken at the real RD Bessel point the projection acts on.

## Equations

- Correction grid, bilinear in a cell: `c = (1 − t)(1 − u)c₀₀ + t(1 − u)c₀₁ + (1 − t)u·c₁₀ + tu·c₁₁`, for each of `Δφ` and `Δλ` in arcseconds.
- ETRS89 to real Bessel: `φₖ₊₁ = φ − Δφ(φₖ, λₖ)/3600`, the same for `λ`, from `(φ₀, λ₀) = (φ, λ)`, until the step is under 1e-12 degrees.
- Isometric latitude: `q = ln tan(φ/2 + 45°) − (e/2) ln((1 + e sin φ)/(1 − e sin φ))`.
- Onto the sphere: `Φ = 2 arctan(exp(n q + m)) − 90°`, `Λ = Λ₀ + n(λ − λ₀)`, with `n = √(1 + e² cos⁴φ₀/(1 − e²))` and `m = w₀ − n q₀`.
- Onto the plane: `sin(ψ/2) = √(sin²((Φ − Φ₀)/2) + sin²((Λ − Λ₀)/2) cos Φ cos Φ₀)`, `r = 2kR tan(ψ/2)`, `x = x₀ + r sin α`, `y = y₀ + r cos α`, with `α` the azimuth from the central point.
- Height: `H = h − N(φ, λ)`.

## Symbols and units

Latitude and longitude in degrees, ETRS89 in and real RD Bessel between the steps; the grid's corrections in arcseconds, longitude east positive; RD easting and northing in metres; heights in metres, `h` above the ETRS89 ellipsoid and `H` above NAP; the convergence in degrees and the scale factor dimensionless.

## Domain

- Inside NSGI's grids, 50°N to 56°N and 2°E to 8°E, which is the European Netherlands and its North Sea EEZ; outside them the answer is `OUT_OF_DOMAIN`.
- The grids are fetched on first use: `ASSET_UNAVAILABLE` without them, `ASSET_INTEGRITY` for a damaged copy.

## Approximations

NSGI states the transformation at 0.001 m inside the administrative borders of the Netherlands and 0.05 m outside them. Variant 2's horizontal path differs from variant 1's by well under 0.001 m inside the grid bounds, by NSGI's own statement, and is bounded by the grid where variant 1 is not. Variant 1 computes RD at a fixed ellipsoidal height of 43 m so that RD does not depend on height; variant 2's grid carries that choice, which is why the easting and northing here do not move with the height either. The grids are stored as the float32 values NSGI's files carry. The input is taken to be ETRS89 as NSGI recommends it, ETRF2000 at epoch 2010.50, which is the `REALIZATION_ASSUMED` warning on every answer.

## Worked example

- sourcePublisher: Nederlandse Samenwerking Geodetische Infrastructuur, as the national-grids specification quotes it
- sourceTitle: The NSGI check point
- sourceEdition: RDNAPTRANS2018, 27 June 2022
- sourceLocator: 5°E 52°N at h 43 m on ETRS89
- independent: yes
- inputs: lat 52.0, lon 5.0, height 43 m
- outputs: RD 128410.0958 m east, 445806.4960 m north, −0.4754 m NAP
- tolerance: 0.0001 m
- verifiedBy: golden vector v001 and `core/crates/gp-geo/tests/rdnap.rs` `the_nsgi_check_point_both_ways`
- verifiedOn: 2026-11-21

## Differential tests

- `tools/vectors/gen_rdnap.py` builds 26 of the 31 vectors in `core/vectors/geodesy.datum.rdnaptrans.jsonl` from PROJ 9.9.0 applying NSGI's own grids and EPSG 28992's `sterea` definition, and the grid convergence and point scale factor of each from PROJ's own `proj -V` factors at the real RD Bessel point: twenty places from Maastricht to Rottumerplaat and six out in the EEZ and off the dense subgrid, every value within 0.0001 m, 1e-7 degrees, or 1e-8. The other five are the NSGI check point and four refusals, one off each edge of the grid.
- `core/crates/gp-geo/src/proj.rs` `rd_agrees_with_proj_and_round_trips`: the projection alone against PROJ's `sterea` at six places, within 10 µm, with the scale factor equal along the meridian and the parallel, and the origin's scale and convergence pinned.

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `rdnaptrans_round_trips_and_its_factors_behave`: the central point lands within 200 m of the false origin with the projection's own scale and no convergence; at six places the answer is in RD's first quadrant, the quasi-geoid is within NLGEO2018's 39 to 49 m range, H = h − N exactly, the convergence leans the way the meridian does and is within a tenth of a degree of the spherical (λ − λ₀) sin φ (a convergence in the wrong units would be 57 times this), the scale factor stays within 0.9999 to 1.0004, and the inverse returns the point to a nanodegree and 0.1 mm with the same factors; and points off every edge of the grid are refused.
- `core/crates/gp-geo/src/hgrid.rs` `nested_grids_pick_the_finest_and_the_inverse_closes`: the nested reader picks the finer grid where it reaches, falls back to the parent outside it, refuses points off both, and its inverse closes.
- `core/crates/gp-geo/src/vgrid.rs` `bilinear_inside_none_outside`: the vertical reader interpolates bilinearly and refuses points off the grid.
- `tools/data/rdnaptrans-assets.test.mjs`: the committed packages have the pinned digests and NSGI's own bounds and spacing, parent and subgrid.
