<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Coordinate system to coordinate system (`geodesy.crs.transform`)

## Method

A transformation between two coordinate systems is three operations applied in turn, as IOGP Guidance Note 7-2 describes concatenated operations: undo the source projection, move the position between datums, and apply the target projection. Both systems are looked up by EPSG code in the CRS index (EPSG v13.102). The source coordinates go to latitude and longitude with the source projection's inverse on its own ellipsoid: the State Plane zone's Lambert, transverse Mercator, or oblique Mercator with this system's own false origin, UTM's Krüger series, or Web Mercator. When the two systems are on different frames, the position goes to ECEF and along the same frame path the datum transformation tool takes at the epoch (NGS HTDP 3.6.0 for NAD83(2011), the IERS parameters between ITRFs, and WGS 84's alignment with its ITRF), and back to latitude and longitude on the target's ellipsoid. The target projection then gives easting and northing in the target's unit. Each step is listed with its stated uncertainty.

## Equations

- Inverse and forward projections: the formulas of the State Plane, UTM, and Web Mercator tools, unchanged.
- A State Plane twin's own origin: E = E_zone − FE_zone + FE_crs, N likewise, with every other parameter equal (checked when the index is generated).
- Frame step i: X ← T_i(X, t), with T_i the 14-parameter transformation at epoch t.
- Total accuracy: σ = √(Σ σ_i²) over the frame steps; projections contribute 0.

## Symbols and units

E and N are easting and northing in the system's unit (meters, US survey feet, or international feet; a bare number is taken in that unit); FE the false easting in meters; X the ECEF position in meters; t the epoch as a decimal year (a date is accepted); σ_i a step's stated 1σ uncertainty in meters.

## Domain

Systems in the CRS index on WGS 84 or NAD83(2011): geographic, Web Mercator, UTM, and State Plane 1983. NAD27 and the original NAD83 are refused with a pointer to the NADCON5 tool, which this does not chain yet; SPCS2022 beta zones are not accepted. Epochs from 1980 to 2100, required only when the frames differ. A point outside the target's area of use is computed with OUTSIDE_ZONE_EXTENT; one the target grid cannot hold is refused.

## Approximations

The projections are exact to well under a millimeter; the frame transformations are the published ones with their stated uncertainties, and those dominate. WGS 84 is taken as its current realization, G2296, with REALIZATION_ASSUMED. The ellipsoid height is carried through the frame steps but is not converted to or from an orthometric height. EPSG defines the feet twins of a State Plane zone with an origin rounded in feet, which differs from the metric zone's origin converted by up to 0.1 mm; this tool follows each EPSG code's own definition, as PROJ does, while the State Plane tools convert the metric answer.

## Worked example

- sourcePublisher: PROJ (OSGeo), EPSG dataset
- sourceTitle: PROJ 9.9.0 cs2cs EPSG:6427 → EPSG:6342 (NAD83(2011) / Colorado Central to NAD83(2011) / UTM zone 13N)
- sourceEdition: PROJ 9.9.0 with EPSG v13.102
- sourceLocator: golden vector v001 of core/vectors/geodesy.crs.transform.jsonl, from tools/vectors/gen_crs_transform.py
- independent: yes
- inputs: E 953,000 m, N 515,000 m on EPSG:6427
- outputs: E 495,733.106 m, N 4,397,344.871 m on EPSG:6342, datum accuracy 0 (one frame)
- tolerance: 1 µm
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

PROJ is the oracle only where it is one: for pairs on a single frame it joins the two systems by conversions alone, so its answer is the projections' and nothing else. Between NAD83(2011) and WGS 84 it applies a near-null ballpark step rather than the frame chain, so the frame-changing path is checked against the separately verified tools it is built from: the add-geodesy-suite composite-path scenario, Colorado Central at E 953,000 m, N 515,000 m, 1,600 m, epoch 2026.0 to WGS 84 UTM 13N, gives E 495,731.771 m, N 4,397,345.388 m, within 1 mm of the State Plane inverse, the datum transformation, and UTM forward run in turn (the first invariant below). The 1.3 m between that and the one-frame answer is the frame change.

## Differential tests

- `tools/vectors/gen_crs_transform.py`: 20 vectors, 15 from PROJ 9.9.0's cs2cs (EPSG v13.102) on pairs that share a frame, in meters, US survey feet, and international feet, north and south, across zones and near the antimeridian, and 5 refusals by rule

## Invariants

- `core/crates/gp-geodesy/tests/crs.rs` `a_frame_change_is_the_three_tools_in_turn`: at three points and epochs the answer is the State Plane inverse, the datum transformation, and UTM forward in turn within 1 mm, with the same steps and accuracy
- `core/crates/gp-geodesy/tests/crs.rs` `there_and_back_returns_the_start`: four pairs, including frame changes and feet, return to the start within 10 µm
- `core/crates/gp-geodesy/tests/crs.rs` `the_same_frame_needs_no_epoch_and_carries_no_datum_error`: one frame needs no epoch and reports 0, and a feet twin differs from its metric zone only by EPSG's origin rounding
