<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ETRS89 UTM to Gauss-Krüger by BeTA2007 (`geodesy.datum.beta2007-inverse`)

## Method

The same transformation as `geodesy.datum.beta2007`, read the other way. Its forward note carries the background on DHDN, the three-degree Gauss-Krüger zones and AdV's grid; this one states the steps and what changes.

1. **Unproject.** UTM on GRS 80 on the zone given — a UTM easting does not say which zone it is on, so the zone is a required input — read backwards by Karney's series, giving a latitude and longitude on ETRS89.
2. **Iterate the grid.** The grid's nodes are indexed by DHDN90 coordinates, so going this way means solving for the DHDN90 point whose offsets land on the ETRS89 position: a fixed-point iteration from that position, until the step is under 1e-12 degrees, as PROJ's hgridshift inverts an NTv2 grid.
3. **Project.** Transverse Mercator on Bessel 1841, scale 1, on the three-degree zone the point lands in, or on one named instead, which warns `NONSTANDARD_ZONE`. The zone number goes in front of the easting, as German data carries it.

Zones overlap half a degree either side of their edges, so a point near one can legitimately be given on either zone; naming a zone is how to choose.

## Equations

- Zone from the easting: `n = ⌊E/1,000,000⌋`, valid for `n` in 2 to 5 with `|E − n·1,000,000 − 500,000| ≤ 300,000`.
- Bilinear offsets: `Δ = (1 − t)(1 − u)Δ₀₀ + t(1 − u)Δ₀₁ + (1 − t)u·Δ₁₀ + tu·Δ₁₁`, for each of `Δφ` and `Δλ`, with `t` and `u` the fractions across the cell.
- `φ_ETRS89 = φ_DHDN + Δφ/3600`, `λ_ETRS89 = λ_DHDN + Δλ/3600`.
- Gauss-Krüger: transverse Mercator with `k₀ = 1`, `λ₀ = 3n`, `FE = n·1,000,000 + 500,000`, on Bessel 1841 (`a = 6,377,397.155 m`, `1/f = 299.1528128`).
- UTM: transverse Mercator with `k₀ = 0.9996`, `λ₀ = 6z − 183`, `FE = 500,000 m`, on GRS 80.
- Ground shift: `√((Δφ·M)² + (Δλ·N cos φ)²)`, with `M` and `N` the meridian and prime-vertical radii of GRS 80 at the point.

## Symbols and units

Easting and northing in metres, Gauss-Krüger with the zone in front of the easting and UTM without; latitude and longitude in degrees, DHDN90 and ETRS89 between the steps; the grid's offsets in arcseconds, longitude east positive; the shift in metres; the convergence in degrees and the scale factor dimensionless, both on the grid the answer is given in.

## Domain

- Inside AdV's grid, 5°30′E to 15°40′E and 47°N to 55°18′N, which is Germany; outside it the answer is `OUT_OF_DOMAIN`.
- The UTM zone is required, 31 to 33, because a UTM easting does not state it; without it the answer is `INVALID_INPUT`.
- The grid is fetched on first use: `ASSET_UNAVAILABLE` without it, `ASSET_INTEGRITY` for a damaged copy.

## Approximations

Because the grid is indexed on DHDN90, this direction iterates where the forward direction reads once; the iteration settles in one to six steps across Germany. Otherwise, AdV states that BeTA2007 reproduces the state transformations it unified within 0.02 m inside Germany, and some states publish their own finer grids for cadastral work. The grid is stored as the float32 values AdV's file carries, about 1e-6 arcseconds, under a millimetre on the ground; its 10-arcminute spacing goes into the package header at full precision, since it is not a round decimal. The projections are Karney's series, exact to nanometres at these distances from the central meridian. The input is taken to be DHDN90 and the answer ETRS89, which is the `REALIZATION_ASSUMED` warning on every answer. Heights are untouched.

## Worked example

- sourcePublisher: Arbeitsgemeinschaft der Vermessungsverwaltungen der Länder der Bundesrepublik Deutschland (AdV)
- sourceTitle: Bundeseinheitliche Transformation für ATKIS (BeTA2007)
- sourceEdition: Documentation version 1.5, grid BETA2007.gsb of 6 November 2009
- sourceLocator: Section 4.2, the printed test records: point 1 record 3 read the other way, ETRS89_UTM32 to DE_DHDN_3GK3
- independent: yes
- inputs: easting 399340.601862 m, northing 5928794.177992 m, utm_zone 32
- outputs: Gauss-Krüger zone 3, 3399371.190396 m east, 5930724.531323 m north
- tolerance: 0.0001 m
- verifiedBy: golden vector v002 and `core/crates/gp-geo/tests/beta2007.rs` `advs_published_records_both_ways`
- verifiedOn: 2026-11-21

## Differential tests

- `tools/vectors/gen_beta2007.py` builds 22 of the 27 vectors in `core/vectors/geodesy.datum.beta2007-inverse.jsonl` from PROJ 9.9.0's hgridshift in reverse applying `de_adv_BETA2007.tif` between each zone's own EPSG record: the same twenty-two places from Aachen to Passau and Flensburg to Konstanz, every easting and northing within 0.01 mm and every zone exact. The other five are AdV's four printed records read backwards and a point south of the grid.
- `core/crates/gp-geo/tests/beta2007.rs` `advs_published_records_both_ways`: AdV's two printed geographic records, the pure grid shift, reproduce within 2e-11 degrees — AdV computed its table with PROJ 4.4.7 and printed twelve decimals.

## Invariants

- `core/crates/gp-grids/tests/grids.rs` `beta2007_crosses_the_german_grids_and_picks_its_zones`: at five places the forward answer returns to 0.1 mm on the Gauss-Krüger zone asked for, with the same datum shift the forward direction reported, and a UTM northing in the Mediterranean is refused.
- `core/crates/gp-geo/tests/beta2007.rs`: the zone an easting states, and the three things it refuses — no zone digits, a zone Germany does not use, and a point 400 km off the zone's meridian; and the zone each longitude falls in, including the three-degree edge at 7°30′E.
- `core/crates/gp-geo/src/hgrid.rs` `bilinear_inside_none_outside_and_the_inverse_closes`: the generic reader interpolates bilinearly, refuses points off the grid, and its inverse closes.
- `tools/data/beta2007-assets.test.mjs`: the committed package has the pinned digest and AdV's extent, with the 10-arcminute spacing exact to 1e-16 degrees and both far edges landing on 15°40′E and 55°18′N.
