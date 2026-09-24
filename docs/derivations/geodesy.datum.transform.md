<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Transform between reference frames (`geodesy.datum.transform`)

## Method

Moving a position between two named frames often takes more than one published transformation. WGS 84 from a receiver reaches NAD 83 (2011) through the ITRF it is aligned with and then NGS's HTDP parameters. An old ITRF reaches a newer one through ITRF2020. This tool keeps the frames and the published transformations between them as a graph and finds the path whose steps' stated uncertainties add up (as variances) to the least. It applies each step to the earth-centered coordinates at the given epoch and reports every step with its accuracy, so the answer carries its own error budget.

The frames are the ITRF realizations from ITRF88 to ITRF2020, WGS 84 (G1150, G1674, G1762, G2139, G2296), and NAD 83 (2011, PA11, MA11). The edges and their stated uncertainties (1σ):

- ITRF2020 to each older ITRF by the IERS 14-parameter transformation: 3 mm for ITRF2014 and ITRF2008, 5 mm for ITRF2005 and ITRF2000, 1 cm for ITRF94 to ITRF97, and 2 cm for the oldest.
- Each WGS 84 realization to the ITRF it is aligned with, with no change in coordinates: 2 cm, the few-centimeter alignment NGA states.
- NAD 83 (2011, PA11, MA11) to each other and to the ITRF and WGS 84 frames HTDP carries, by the NGS HTDP 3.6.0 14-parameter sets: 2 cm, the transformation's own accuracy in the conterminous United States.

## Equations

- Earth-centered X from latitude, longitude, and height on GRS 80
- Each IERS step: X′ = T + (1 + D)·X + R·X, with every parameter at the epoch (P(t) = P + Ṗ(t − 2015.0)), inverted exactly for the reverse direction
- Each HTDP step: HTDP's toit94 then frit94, with coordinate-frame rotations and additive scale, as HTDP applies them
- Path: Dijkstra's shortest path with edge weight σ²; stated accuracy σ = √(Σ σᵢ²)

## Symbols and units

`lat`, `lon` in degrees and `height` (ellipsoidal) in any length unit; `from` and `to` frames; `epoch` as a decimal year or a date. Out come `lat`, `lon`, and `height` in the target frame, the horizontal `shift`, the path `accuracy`, and `steps`, each with its frames, method, and accuracy.

## Domain

Epochs from 1980 to 2100. An unqualified "WGS84" is taken as G2296, with REALIZATION_ASSUMED. NAD 27 and the older NAD 83 realizations need the NADCON5 grids and are not part of this graph.

## Approximations

None in the steps, which apply the published parameters exactly. The stated accuracy is the combination of the steps' published or stated uncertainties, not an error estimate for any particular point, and it leaves out the coordinates' own accuracy.

## Worked example

- sourcePublisher: National Geodetic Survey, NOAA
- sourceTitle: Horizontal Time-Dependent Positioning (HTDP)
- sourceEdition: HTDP 3.6.0 (2025-04-07), compiled from htdp.f and initbd.f
- sourceLocator: menu option 4, transforming positions between frames
- independent: yes
- inputs: 38.5° N, 98° W, 500 m, from WGS 84 (G2296) to NAD 83 (2011) at epoch 2026.7
- outputs: 38.4999943559° N, 97.9999861999° W, 501.023 m
- tolerance: 1e-8° and 1.5 mm in height
- verifiedBy: golden vectors v004 onward, run by the core on every build
- verifiedOn: 2026-09-24

The graph takes WGS 84 (G2296), aligned with ITRF2020, and HTDP's set to NAD 83 (2011), which is the path HTDP itself uses, so HTDP's printed answer applies.

## Differential tests

- `tools/vectors/gen_transform_vectors.py`: the vectors pinned from NGS HTDP (where NAD 83 is one of the frames) and from PROJ's ITRF2020 parameter file for the dedicated tools, carried over with their sources, since the graph takes the same path; HTDP's own sets between ITRF and WGS 84 frames are not carried over, since the graph uses the IERS parameters and NGA's alignment there, which differ from HTDP's by up to 2 cm
- `core/vectors/geodesy.datum.transform.jsonl`: 43 vectors, the two scenarios (unqualified WGS 84, coincidence), the epoch range, and 40 reference values

## Invariants

- `core/crates/gp-geodesy/tests/frame_path.rs` `frame_path_invariants`: for all 484 pairs of frames, the stated accuracy is the root sum of squares of the steps, the steps chain from the source to the target, the path back is as accurate as the path there, the round trip returns the point within 10 µm (HTDP's reverse, which negates its parameters, leaves up to 1.5 µm), and a path of one step gives exactly what the dedicated ITRF or NAD 83 tool gives
