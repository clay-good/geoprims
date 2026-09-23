<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Sun hotspot in a camera frame (`time.sun.hotspot`)

## Method

The hotspot is the bright patch that appears in aerial imagery where the camera looks directly away from the sun. Every surface scatters most strongly back along the direction the light came from, so the point opposite the sun — the antisolar point — is the brightest place in the frame, and a survey that catches it gets a blown patch that photogrammetry cannot correct.

Finding it is geometry, not photometry. The sun's azimuth and elevation come from the NREL SPA. The antisolar direction is the opposite bearing at the negative of the sun's elevation: if the sun is 40° up in the south-east, the antisolar point is 40° *below* the horizon in the north-west, which is why a nadir camera catches it only when the sun is low.

The camera's look direction is its heading and pitch. Both directions are turned into unit vectors and the angle between them is the arccosine of their dot product. The frame test treats the field of view as a cone: the hotspot is in frame when that angle is within half the diagonal field of view, which is the right test for the corners and slightly generous for the edges.

## Equations

- Antisolar azimuth = (sun azimuth + 180°) mod 360°; antisolar elevation = −(sun elevation).
- A direction (azimuth a, elevation e) as a unit vector: (cos e · sin a, cos e · cos a, sin e).
- Hotspot angle = arccos(look · antisolar), both unit vectors.
- In frame when the hotspot angle ≤ (diagonal field of view) / 2.

## Symbols and units

`lat`, `lon` and `time` place the sun; `camera_heading` (from true north, default 0°) and `camera_pitch` (negative downwards, −90° is nadir) place the camera; `field_of_view` is the diagonal, default 84°. Out come `hotspot_angle`, `in_frame`, the sun's `elevation` and `azimuth`, and the antisolar pair.

## Domain

Any place, any time, any camera attitude. A nadir camera's hotspot angle is exactly 90° minus the sun's elevation, so with the default 84° field of view it falls in frame whenever the sun is below 48° — which is most of the working day outside high summer.

## Approximations

The sun direction is the SPA's, good to about 0.0003°. The cone is the approximation that matters: a real frame is a rectangle, so a hotspot just outside the cone but within the diagonal corner is reported out of frame when it might clip a corner. The cone is the conservative reading at the edges and the exact one at the corners.

## Worked example

- sourcePublisher: National Renewable Energy Laboratory; pvlib community
- sourceTitle: Solar Position Algorithm for Solar Radiation Applications (NREL/TP-560-34302); pvlib-python
- sourceEdition: Revised January 2008; pvlib 0.13.0
- sourceLocator: SPA sections 3.1 to 3.15; pvlib `solarposition.spa_python` at sea level, 101,325 Pa, 12 °C
- independent: yes
- inputs: 23 camera-and-sun cases, including a nadir camera at midday and near sunset, an oblique camera pointed at and away from the sun, both hemispheres, the equator at equinox, and Reykjavik at midnight in June
- outputs: the hotspot angle, both sun angles, the antisolar azimuth, and the in-frame verdict
- tolerance: 0.003° on every angle, exact on the verdict
- verifiedBy: golden vectors v001 to v023, run by the core on every build
- verifiedOn: 2026-09-23

The sun comes from pvlib, a separate implementation of the same NREL algorithm; `core/crates/gp-time/tests/spa_parity.rs` compares the two at 250 committed points and agrees within two arcseconds, so the tolerance here is a few of those rather than a number picked to make the test pass. Everything above the sun — the antisolar direction, the dot product, the cone test — is worked out in the generator from the definitions, not read back from the core.

## Differential tests

- `tools/vectors/gen_sun_pvlib.py`: 17 of the 23 vectors, sun from pvlib and geometry from first principles
- `core/crates/gp-time/tests/spa_parity.rs`: the sun itself against pvlib at 250 points over 1990–2060
- `core/vectors/time.sun.hotspot.jsonl`: 23 vectors across both hemispheres and every camera attitude

## Invariants

- `core/crates/gp-time/tests/time.rs` `hotspot_invariants`: for a nadir camera the hotspot angle is exactly 90° minus the sun's elevation, which is the whole geometry stated as one identity and fails if the antisolar elevation is not negated; the antisolar azimuth is 180° from the sun's, wrapped, at every case tried; a camera pointed straight at the sun is exactly 180° from the hotspot at every elevation, since the antisolar point is the sun's antipode on the sphere and not a reflection of it in the horizon — a distinction easy to get wrong, and one the test now holds; the verdict follows the cone, being yes exactly when the angle is within half the field of view, checked either side of the boundary by widening the field of view rather than moving the sun; and a sun exactly overhead puts the antisolar point at nadir, so a nadir camera sees it and a horizontal one does not
