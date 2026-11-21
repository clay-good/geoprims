<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# RD and NAP to GNSS position by RDNAPTRANS2018 (`geodesy.datum.rdnaptrans-inverse`)

## Method

The same transformation as `geodesy.datum.rdnaptrans`, read the other way. Its forward note carries the background; this one states the steps and what changes.

1. **Off the RD projection.** The inverse double projection: an inverse oblique stereographic projection from the plane back onto the sphere at Amersfoort, then an inverse Gauss conformal projection from the sphere onto Bessel 1841, whose latitude is solved by the fixed-point iteration NSGI gives. That leaves the point in real RD Bessel.
2. **Onto ETRS89.** The rdtrans2018 correction grid is indexed by real RD Bessel coordinates, so here it is read once rather than iterated, and its corrections are added. The nested grid is used the same way: the 0.02° by 0.0125° subgrid over the country where it reaches, the 0.1° parent over the EEZ otherwise.
3. **The height.** NLGEO2018 gives the quasi-geoid height N above the ETRS89 ellipsoid at the ETRS89 position found in step 2, and the ellipsoidal height is h = H + N.

This is the cheaper direction: the horizontal step iterates in the forward tool and not here, and the projection's latitude iteration converges in a handful of steps.

## Equations

- Off the plane: `r = √((x − x₀)² + (y − y₀)²)`, `ψ = 2 arctan(r/2kR)`, `sin α = (x − x₀)/r`, `cos α = (y − y₀)/r`.
- On the sphere: `Φ = arcsin(cos α cos Φ₀ sin ψ + sin Φ₀ cos ψ)`, `Λ = Λ₀ + atan2(sin α sin ψ, cos Φ₀ cos ψ − cos α sin Φ₀ sin ψ)`.
- Onto the ellipsoid: `q = (ln tan(Φ/2 + 45°) − m)/n`, then `φₖ₊₁ = 2 arctan(exp(q + (e/2) ln((1 + e sin φₖ)/(1 − e sin φₖ)))) − 90°` from `φ₀ = 0`, and `λ = λ₀ + (Λ − Λ₀)/n`.
- Correction: `φ_ETRS89 = φ_Bessel + Δφ/3600`, the same for `λ`.
- Height: `h = H + N(φ, λ)`.

## Symbols and units

RD easting and northing in metres; latitude and longitude in degrees, real RD Bessel between the steps and ETRS89 out; the grid's corrections in arcseconds, longitude east positive; heights in metres, `H` above NAP and `h` above the ETRS89 ellipsoid; the convergence in degrees and the scale factor dimensionless.

## Domain

- Inside NSGI's grids, 50°N to 56°N and 2°E to 8°E after the inverse projection; outside them the answer is `OUT_OF_DOMAIN`. The projection itself has an answer for every plane coordinate, so the refusal always comes from the grid.
- The grids are fetched on first use: `ASSET_UNAVAILABLE` without them, `ASSET_INTEGRITY` for a damaged copy.

## Approximations

As in the forward note: NSGI's 0.001 m inside the administrative borders of the Netherlands and 0.05 m outside them, variant 2 for the horizontal step and variant 1 for the vertical. Because the forward tool computes RD at a fixed ellipsoidal height, going out and back is not exactly closed; NSGI puts that at under 0.001 m up to 500 km outside the grids, and the round trip here is measured at a nanodegree and 0.1 mm. The answer is ETRS89 as NSGI recommends it, ETRF2000 at epoch 2010.50, which is the `REALIZATION_ASSUMED` warning on every answer.

## Worked example

- sourcePublisher: Nederlandse Samenwerking Geodetische Infrastructuur, as the national-grids specification quotes it
- sourceTitle: The NSGI check point, read backwards
- sourceEdition: RDNAPTRANS2018, 27 June 2022
- sourceLocator: RD 128410.0958, 445806.4960 at −0.4754 m NAP
- independent: yes
- inputs: easting 128410.0958 m, northing 445806.496 m, nap_height -0.4754 m
- outputs: 52°N 5°E at h 43 m on ETRS89
- tolerance: 2e-9 degrees and 0.0002 m
- verifiedBy: golden vector v001 and `core/crates/gp-geo/tests/rdnap.rs` `the_nsgi_check_point_both_ways`
- verifiedOn: 2026-11-21

## Differential tests

- `tools/vectors/gen_rdnap.py` builds 26 of the 28 vectors in `core/vectors/geodesy.datum.rdnaptrans-inverse.jsonl` from PROJ 9.9.0 applying NSGI's own grids and EPSG 28992's `sterea` definition in reverse, at twenty places from Maastricht to Rottumerplaat and six out in the EEZ and off the dense subgrid, every latitude and longitude within a nanodegree and every height within 0.0001 m. The other two are the NSGI check point and a refusal 1,345 km east of Amersfoort.
- `core/crates/gp-geo/src/proj.rs` `rd_agrees_with_proj_and_round_trips`: the inverse projection alone returns PROJ's own forward coordinates to 1e-10 degrees at six places.

## Invariants

- `core/crates/gp-grids/tests/grids.rs` `rdnaptrans_round_trips_and_its_factors_behave`: at six places the forward answer comes back to a nanodegree and 0.1 mm in height, with the same grid convergence and point scale factor the forward tool reported, and a point far east of the grid is refused.
- `core/crates/gp-geo/src/hgrid.rs` `nested_grids_pick_the_finest_and_the_inverse_closes`: the nested reader picks the finer grid where it reaches, falls back to the parent outside it, refuses points off both, and its inverse closes.
- `core/crates/gp-geo/src/vgrid.rs` `bilinear_inside_none_outside`: the vertical reader interpolates bilinearly and refuses points off the grid.
- `tools/data/rdnaptrans-assets.test.mjs`: the committed packages have the pinned digests and NSGI's own bounds and spacing, parent and subgrid.
