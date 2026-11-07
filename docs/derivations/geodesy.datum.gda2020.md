<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GDA94 and GDA2020 (`geodesy.datum.gda2020`)

## Method

Australia's datum moved. GDA94 was fixed to the continent as it sat in 1994; GDA2020 is fixed to where it sits in 2020, and the plate carried it about 1.8 m north-east in between. A GDA94 coordinate read as GDA2020 is therefore out by about a car's length, which matters as soon as a phone's position is compared with a plan.

ICSM publishes two ways across. The **conformal** transformation moves the whole country as one rigid piece: three translations, three rotations and a scale, applied to earth-centred Cartesian coordinates. The **conformal-plus-distortion** grid does that and then adds the local distortion of GDA94's own network, up to about 0.1 m in places, which a rigid motion cannot carry.

This tool applies the conformal transformation, in the seven-parameter form EPSG registers as operation 8048, in the coordinate-frame convention. The reverse direction is the exact inverse of the forward transformation rather than the negated parameters, which is the same choice the other datum tools here make. What the tool does not do is the distortion, so it says so on every answer and names the grid that carries it.

## Equations

- Geographic to earth-centred Cartesian on GRS 80, the Helmert of EPSG 8048, then back:
  `V₂ = T + (1 + dS)·R(r)·V₁`, with the coordinate-frame rotation matrix, `T = (0.06155, −0.01087, −0.04019) m`, `r = (−0.0394924, −0.0327221, −0.0328979)″` and `dS = −0.009994 ppm`.
- The shift reported is the east-north offset between the two coordinates of the point, which is what moves on the ground.

## Symbols and units

Latitude and longitude in degrees; ellipsoidal height in meters above GRS 80; translations in meters, rotations in arc-seconds, scale in parts per million; the shift in meters and its direction in degrees from true north.

## Domain

Any position; EPSG's published area of use for operation 8048 is Australia and its offshore territories, and outside it the result carries `OUTSIDE_AREA_OF_USE` because the transformation is then extrapolation. Heights are ellipsoidal: AHD heights are a separate step, by AUSGeoid2020.

## Approximations

The conformal transformation is a rigid motion, so it cannot carry the distortion in GDA94's own network — up to about 0.1 m in places. EPSG states operation 8048 at 0.01 m, which is how well the parameters reproduce the conformal grid, not how well the conformal grid matches a cadastral survey; `DISTORTION_NOT_MODELLED` is on every answer for that reason. The rotations are linearized, as the EPSG method defines them, which is what the published parameters are fitted against.

## Worked example

- sourcePublisher: IOGP, from the Intergovernmental Committee on Surveying and Mapping, through PROJ
- sourceTitle: EPSG Geodetic Parameter Dataset, operation 8048 (GDA94 to GDA2020 (1))
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The operation's own parameters, applied by PROJ to the ALIC station coordinates of the change's scenario
- independent: yes
- inputs: direction to-gda2020, lat −23.670124, lon 133.885513, height 603.3466 m
- outputs: the point moves 1.744 m at 29.1° true, with the new latitude, longitude and ellipsoidal height
- tolerance: 1e-9° and 0.1 mm
- verifiedBy: golden vector v001 and `core/crates/gp-geodesy/tests/datum.rs` `gda2020_moves_the_continent_and_inverts`
- verifiedOn: 2026-11-07

ICSM's own manual prints a worked example, but icsm.gov.au now redirects its technical-manual PDFs to a landing page and the document could not be read here; the parameters and the area of use come from EPSG, which registers ICSM's transformation, and the check is PROJ applying that same operation.

## Differential tests

- `tools/vectors/gen_gda2020.py` builds the 21 vectors in `core/vectors/geodesy.datum.gda2020.jsonl` from PROJ 9.9.0 through `cct`, holding it to the conformal operation rather than letting it choose the grid, so both sides apply the same published parameters: ten places from Cape York to Macquarie Island, each way, within a nanodegree and a tenth of a millimetre of height, and a point outside the area of use, which is answered and flagged.

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `gda2020_moves_the_continent_and_inverts`: across the mainland the shift is between 1.45 m and 1.85 m, measured from Hobart and Sydney in the south-east (1.49 m) to Darwin (1.80 m), and its direction is north-east everywhere; the two directions are exact inverses, returning a point to within a nanodegree; and every answer carries `DISTORTION_NOT_MODELLED`, with `OUTSIDE_AREA_OF_USE` added beyond the published area.
