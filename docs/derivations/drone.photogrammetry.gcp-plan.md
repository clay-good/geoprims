<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GCP and checkpoint plan (`drone.photogrammetry.gcp-plan`)

## Method

The checkpoint count is the ASPRS Positional Accuracy Standards, Edition 2, Annex C.3, Table C.1 ("Recommended Number of Checkpoints Based on Area"), read in Version 1.0 (February 2023). As C.3 directs, the area is split by land cover: the open (non-vegetated) share gets the table's count for horizontal and NVA testing, and a vegetated share gets the table's count again for VVA. The accuracy the ground control needs is section 7.9: horizontally half the product's RMSE_H; vertically half the elevation product's RMSE_V, or, for planimetric work only, no more than the product's RMSE_H. Checkpoints must be at least twice as accurate as the product (7.12). The standard does not say how many ground control points to set or where; the suggested layout is a rule of thumb after Pix4D's guidance (at least 5, placed evenly, one in the center, not exactly at the edges): a point inset from each corner, one at the point farthest from every edge, and an optional grid. Checkpoints are laid on an even grid between them. Every point passes the even-odd point-in-polygon test the buffer tools use (`gp_geo::buffer::inside_rings`), with holes counted as outside, and stays at least the inset from every edge.

## Equations

- Table C.1, area A in whole km²: 30 when A ≤ 500; 30 + 5 × ⌈(A − 500) / 250⌉ up to 2,500 (35, 40, … 70)
- Checkpoints = C1(A × (1 − v)) + C1(A × v) when v > 0
- RMSE_H(GCP) ≤ ½ RMSE_H(product); RMSE_V(GCP) ≤ ½ RMSE_V(DEM), or ≤ RMSE_H(product) without an elevation product
- RMSE(checkpoint) ≤ ½ RMSE(product), per component
- Corner point: along the corner's bisector, at inset / sin(half the corner angle), kept only if it is inside and at least the inset from every edge

## Symbols and units

The area as latitude and longitude rows (ring 0 the boundary, rings 1 on holes); v the vegetated share; the accuracy classes as RMSE in cm; the grid spacing and inset in m (inset 10 m by default, a choice of this tool, not of either source). Out come the counts, the area in km², the accuracies in cm, and the points as latitude and longitude rows.

## Domain

A polygon of at least 3 corners within 50 km of its center, between 85° S and 85° N, with each land cover's share no larger than 2,500 km², where Table C.1 ends; past it the standard leaves the horizontal count to the client, so the tool stops with OUT_OF_DOMAIN.

## Approximations

The area and layout are on a local transverse Mercator plane, true to about 1e-7 over a few kilometers; near a row boundary of Table C.1 that is well under a square kilometer. The layout cannot see the ground: it knows nothing of roofs, slopes over 10%, vegetation, access, or whether a target is visible from the air, which 7.12 and 7.13 also require.

## Worked example

- sourcePublisher: American Society for Photogrammetry and Remote Sensing
- sourceTitle: ASPRS Positional Accuracy Standards for Digital Geospatial Data, Edition 2
- sourceEdition: Version 1.0.0, February 2023 (as distributed by aagsmo.org; read 2026-09-25)
- sourceLocator: Annex C.3, the worked case under Table C.1
- independent: yes
- inputs: an area of 1,500 km², 500 km² of it heavily vegetated; the vector uses a 38.6 km square of about 1,490 km² with a third vegetated, so each share stays inside its table row
- outputs: 40 checkpoints for the open 1,000 km², 30 for the vegetated 500 km², 70 in all, as C.3 prints
- tolerance: exact counts
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-09-25

## Differential tests

- `tools/vectors/gen_mission_planning.py`: Table C.1 typed from the standard, with areas from GeographicLib's Planimeter (geodesic polygon area) for squares of about 16 ha, 400, 600, 1,100, and 2,400 km², and the C.3 case; each asserted to sit at least 2 km² inside its table row; and a 2,700 km² square refused.
- `core/crates/gp-drone/tests/planning.rs` `gcp_table_c1_rows`: every row boundary of Table C.1 (500, 501, 750, 751, … 2,500) and the refusal past it.

## Invariants

- `gcp_points_inside_an_l_shape`: on an L-shaped area, with no grid, a 60 m grid, and a 25 m grid, every ground control point and all 30 checkpoints fall inside the L by a ray test written apart from the core's, and none in the notch; the test counts the points it checked.
- `gcp_holes_are_outside`: no point falls in a hole.
