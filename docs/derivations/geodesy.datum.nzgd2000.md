<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# NZGD1949 to NZGD2000 by LINZ's grid (`geodesy.datum.nzgd2000`)

## Method

New Zealand's mapping moved from NZGD1949 to NZGD2000, and the old datum was not just offset from the new one but bent: it was built from a triangulation adjusted in pieces. Land Information New Zealand (LINZ) publishes three ways to move between them: a three-parameter shift (5 m), a seven-parameter similarity (4 m), and a distortion grid. The grid is stated at 0.1 to 1.0 m when it was calculated, and LINZ says it becomes less accurate as the datums keep deforming. This tool applies the grid. The seven-parameter set is in `geodesy.datum.legacy`.

The grid, nzgd2kgrid0005, gives a latitude offset and a longitude offset in arcseconds at nodes 0.1° apart, from 166°E to 180°E and from 34°S to 48°S. The tool reads it through `gp_geo::hgrid`, a generic reader for single-grid shifts of this kind:

1. **Interpolate.** The four nodes around the NZGD1949 point, weighted bilinearly by where the point sits in the cell.
2. **Add.** The offsets are added to the NZGD1949 latitude and longitude to give NZGD2000.
3. **Reverse.** From NZGD2000 the offsets are taken away at the latest estimate, repeatedly, until the estimate moves by under 1e-12 degrees. This is how PROJ's hgridshift inverts an NTv2 grid.

The shift on the ground is the north and east offset converted to meters on GRS 80 at the point, with its direction.

## Equations

- `x = (λ − 166°)/0.1°`, `y = (φ + 48°)/0.1°`, `c = ⌊x⌋`, `r = ⌊y⌋`, `t = x − c`, `u = y − r`.
- `Δ = (1 − t)(1 − u)Δ₀₀ + t(1 − u)Δ₀₁ + (1 − t)u·Δ₁₀ + tu·Δ₁₁`, for each of `Δφ` and `Δλ`.
- `φ₂₀₀₀ = φ₁₉₄₉ + Δφ/3600`, `λ₂₀₀₀ = λ₁₉₄₉ + Δλ/3600`.
- Reverse: `φₖ₊₁ = φ₂₀₀₀ − Δφ(φₖ, λₖ)/3600`, the same for `λ`, from `(φ₀, λ₀) = (φ₂₀₀₀, λ₂₀₀₀)`.
- Ground shift: `north = Δφ·M`, `east = Δλ·N cos φ`, with `M` and `N` the meridian and prime-vertical radii of GRS 80.

## Symbols and units

Latitude and longitude in degrees; the grid's offsets in arcseconds, longitude east positive; the ground shift in meters and its direction in degrees from true north.

## Domain

- Inside the grid, 166°E to 180°E and 34°S to 48°S; outside it the answer is `OUT_OF_DOMAIN`. The Chatham Islands, at 176°30′W, are outside.
- The grid is fetched on first use: `ASSET_UNAVAILABLE` without it, `ASSET_INTEGRITY` for a damaged copy.

## Approximations

The grid's own 0.1 to 1.0 m at the time it was calculated, degrading since, as LINZ states. That is the `LOW_ACCURACY_TRANSFORM` warning on every answer. The grid is stored as the float32 values LINZ's file carries, about 1e-6 arcseconds, under a millimeter on the ground. NZGD2000 is itself fixed at epoch 2000.0; bringing a position to today's epoch needs LINZ's deformation model, which this tool does not apply.

## Worked example

- sourcePublisher: Land Information New Zealand, as the national-grids specification quotes it
- sourceTitle: The NZGD1949 to NZGD2000 distortion grid example
- sourceEdition: Grid nzgd2kgrid0005
- sourceLocator: 36°30′00″S 175°00′00″E on NZGD1949
- independent: yes
- inputs: lat 36°30'00"S, lon 175°00'00"E
- outputs: 36°29'53.48482"S 175°00'00.69469"E on NZGD2000
- tolerance: 0.0001 arcseconds
- verifiedBy: golden vector v001 and `core/crates/gp-grids/tests/grids.rs` `the_nzgd2000_grid_round_trips_and_moves_new_zealand_north_east`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_nzgd2000.py` builds 26 of the 28 vectors in `core/vectors/geodesy.datum.nzgd2000.jsonl` from PROJ 9.9.0's hgridshift applying `nz_linz_nzgd2kgrid0005.tif`, the same LINZ grid the asset is packed from: twenty places from Cape Reinga to Stewart Island and Milford Sound to East Cape, forward, and six of them in reverse, all within a nanodegree. The other two are the LINZ example and the Chatham Islands refusal.

## Invariants

- `core/crates/gp-grids/tests/grids.rs` `the_nzgd2000_grid_round_trips_and_moves_new_zealand_north_east`: at five cities the move is north and east and between 165 and 210 m (174 to 203 m at the twenty vector places); each reverses back onto its NZGD1949 point to 1e-11 degrees; and points off the grid, the Chatham Islands and north of it, are refused.
- `core/crates/gp-geo/src/hgrid.rs` `bilinear_inside_none_outside_and_the_inverse_closes`: the generic reader interpolates bilinearly, refuses points off the grid, and its inverse closes.
- `tools/data/nzgd2000-grid-assets.test.mjs`: the committed package has the pinned digest and LINZ's extent.
