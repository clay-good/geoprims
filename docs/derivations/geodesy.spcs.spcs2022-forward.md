<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# State plane coordinates (SPCS2022 beta), forward (`geodesy.spcs.spcs2022-forward`)

## Method

The selected NGS zone supplies 6 exact defining parameters on the zone's 2022 terrestrial reference frame and the GRS 80 ellipsoid. LC1 uses Lambert conformal conic with 1 standard parallel. TM uses the sixth-order Krüger transverse Mercator. OMC uses Hotine oblique Mercator, center form (EPSG variant B), with the NGS skew azimuth as both the center-line azimuth and rectified-grid angle.

## Equations

- LC1: `n = sin φ₀`, `F = k₀ m₀ / (n t₀ⁿ)`, `r = aF tⁿ`, and `θ = n(λ - λ₀)`. Easting is `FE + r sin θ`; northing is `FN + r₀ - r cos θ`.
- TM: Krüger's series through sixth order in `n = f / (2 - f)`, offset from the latitude and longitude of origin.
- OMC: IOGP Guidance Note 7-2 Hotine oblique Mercator variant B, with false coordinates at the projection center.

## Symbols and units

`a` and `f` are the GRS 80 semi-major axis and flattening. `φ` and `λ` are latitude and longitude. `k₀` is the exact NGS projection-axis scale. `FE` and `FN` are false easting and northing in meters. The output can be converted exactly to international feet.

## Domain

The input must already be in the selected zone's NATRF2022, PATRF2022, MATRF2022, or CATRF2022 frame. A coordinate outside the published bounding box is computed with `OUTSIDE_ZONE_EXTENT`.

## Approximations

The Krüger TM series is accurate to nanometers at SPCS zone widths. Convergence and scale for Hotine zones use Richardson-extrapolated numerical derivatives. The beta zone definitions are used exactly as NGS publishes them.

## Worked example

- sourcePublisher: National Geodetic Survey, NOAA
- sourceTitle: SPCS2022 Example Coordinates and Distortion Values
- sourceEdition: Beta, June 1, 2026
- sourceLocator: Gulf zone 001001 through Puerto Rico zone 720001, 1 published point in every zone
- independent: yes
- inputs: 953 NGS latitude and longitude check points
- outputs: easting, northing, point scale, and convergence
- tolerance: 0.001 m, 2.1e-9 scale, and 0.01 arcsecond convergence
- verifiedBy: automated whole-registry comparison
- verifiedOn: 2026-09-28

## Differential tests

- `core/crates/gp-geodesy/src/spcs2022.rs`: compares all 953 zones with the official NGS beta examples; the committed golden vectors exercise 20 zones through the public tool and both hosts.

## Invariants

- `core/crates/gp-geodesy/src/spcs2022.rs` `every_zone_matches_the_official_ngs_example`: forward followed by inverse returns each NGS check point within `1e-11` degree. Every result names the zone's frame, registry status, and definition publication date.
