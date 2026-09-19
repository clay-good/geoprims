<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# State plane coordinates (SPCS83), forward (`geodesy.spcs.spcs83-forward`)

## Method

Each of the 124 SPCS83 zones is defined in the EPSG dataset by its projection method and parameters on GRS 80. Transverse Mercator uses Krüger's series to sixth order (Karney 2011); Lambert Conic Conformal (2SP) and Hotine Oblique Mercator (variant A) follow IOGP Guidance Note 7-2.

## Equations

- LCC 2SP: n = (ln m₁ − ln m₂)/(ln t₁ − ln t₂), F = m₁/(n t₁ⁿ), r = aF tⁿ, θ = n(λ − λ₀); E = FE + r sin θ, N = FN + r_F − r cos θ; k = r n/(a m).
- m(φ) = cos φ / √(1 − e² sin² φ), t(φ) = tan(π/4 − φ/2) / ((1 − e sin φ)/(1 + e sin φ))^(e/2).
- TM: Krüger series in n = f/(2 − f) with false origin at the zone's latitude of origin.
- Hotine variant A: the G7-2 forward with u and v on the rectified grid, rotated by γc.

## Symbols and units

a, f GRS 80 semi-major axis and flattening; φ, λ latitude and longitude; λ₀ central meridian; FE, FN false easting and northing (m); k point scale; γ convergence.

## Domain

NAD83 latitude and longitude. A point outside the zone's area of use is computed but warned (OUTSIDE_ZONE_EXTENT).

## Approximations

None beyond double precision: the Krüger series is accurate to nanometers within the zone widths.

## Worked example

- sourcePublisher: PROJ (OSGeo), EPSG dataset
- sourceTitle: PROJ 9.3.0 transformation EPSG:4269 → EPSG:32129 (Pennsylvania South, meters)
- sourceEdition: PROJ 9.3.0 with the EPSG dataset it ships
- sourceLocator: core/crates/gp-geodesy/tests/data/spcs83_diff.csv, every zone
- independent: yes
- inputs: 2,480 random points, 20 per zone
- outputs: easting and northing in meters
- tolerance: 1 mm (observed 0.05 µm); convergence 1e-6°, scale 1e-7 (PROJ's numerical factors)
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-geodesy/tests/spcs.rs`: all 124 zones against PROJ, round trips, and the Pennsylvania South scenario (±0.003 ftUS)

## Invariants

- `core/crates/gp-geodesy/tests/spcs.rs` `tools_round_trip_in_every_zone`: forward then inverse returns the point within 1e-9° in every zone, with matching convergence and scale
