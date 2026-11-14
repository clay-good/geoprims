<!-- Derivation note (trust/correctness-program layer A). -->
# Ellipsoidal height to orthometric height (`geodesy.height.ellipsoidal-to-orthometric`)

## Method

A GNSS height is measured from the WGS 84 ellipsoid; a map or sea-level height is measured from the geoid. The two differ by the geoid height N at the point. The operation interpolates N from the EGM96 15-minute grid and applies it in one direction, using the same grid reader and interpolation as `geodesy.height.convert`.

## Equations

- H = h − N
- N from the egm96-15 grid by GeographicLib's 12-point cubic fit (or bilinear interpolation when chosen)
- With `frame` NAD83(2011) and an `epoch`, the ellipsoidal height is moved between NAD 83 (2011) and WGS 84 (G2296, aligned with ITRF2020) through ECEF on GRS 80 by NGS's HTDP parameters at that epoch before or after the geoid step, and `frame_step` reports h(WGS 84) − h(NAD 83 (2011)).

## Symbols and units

h ellipsoidal height, H orthometric height, N geoid height (positive where the geoid is above the ellipsoid). Heights keep the unit they are entered in; N is reported in meters.

## Domain

Latitude from −90° to 90°, any longitude, and the ellipsoidal height h. The EGM96 grid is an on-demand asset; when it is missing the operation says so with ASSET_UNAVAILABLE.

## Approximations

EGM96 approximates global mean sea level to about half a meter to a meter and is not a national vertical datum; ORTHOMETRIC_AS_ELLIPSOIDAL says so with every result.

EGM96 is referred to WGS 84. A NAD 83 (2011) ellipsoidal height, the kind OPUS returns, sits 0.24 m (the northwest) to 1.72 m (the southeast) above the WGS 84 one across the lower 48 at 2026.0, measured with this tool over a 1° grid; NAD 83 is tilted against ITRF, so the gap grows from northwest to southeast. Named with an epoch, the height is moved by HTDP's parameters and the step is reported. Named without one, it is not moved, because the step depends on the date and guessing it would hide that; the answer carries FRAME_MISMATCH with the gap sized at 2010.0, NAD 83 (2011)'s reference epoch. GEOID18, which NAVD 88 needs, is not used: it waits on the asset pipeline.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: GeographicLib, GeoidEval utility
- sourceEdition: GeographicLib 2.7, with the egm96-15 grid (2009-08-29)
- sourceLocator: `GeoidEval -d assets/data/egm96-15/2009-08-29 -n egm96-15` at the point, then H = h − N
- independent: yes
- inputs: 46.452° N, 6.583° E, h = 448.2 m
- outputs: the geoid height and converted height of golden vector v001
- tolerance: 0.05 mm, GeoidEval's printed precision
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_height_direction.py`: 20 points including both poles, heights from −400 m to 9,000 m in meters and feet, two by bilinear interpolation, each N from the GeoidEval program rather than the core's interpolator.
- `core/vectors/geodesy.height.ellipsoidal-to-orthometric.jsonl`: those vectors and two refusal cases.
- `core/crates/gp-geodesy/tests/geodesy.rs` `directional_height_conversions_match_the_general_tool` requires the general tool's answer and the round trip.
- `tools/vectors/gen_height_frames.py`: the frame step from NGS HTDP 3.6.0 itself, compiled from its Fortran source by `gen_htdp.py`'s own functions (menu option 4, NAD83(2011) and WGS84(G2296) at equal dates), and N from GeoidEval on the egm96-15 grid, at five places from Seattle to Miami; within 1.5 mm, HTDP's printed millimetre plus its arithmetic. Also the no-epoch FRAME_MISMATCH case.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `height_views_invariants`: h − H = N at four places, in meters and in feet; the height keeps the unit it was entered in; and converting back with `geodesy.height.orthometric-to-ellipsoidal` returns it.
- `core/crates/gp-geodesy/tests/height_frames.rs`: a WGS 84 height, named or not, takes no step and no frame warning, so every earlier result stands; a NAD 83 (2011) height round-trips through H to a micrometre with the same step both ways; the step lies in the measured band and grows from Seattle to Miami; and without an epoch FRAME_MISMATCH names WGS 84 and the size in metres.
