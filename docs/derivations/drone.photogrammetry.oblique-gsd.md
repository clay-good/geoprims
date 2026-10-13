<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Oblique GSD and footprint (`drone.photogrammetry.oblique-gsd`)

## Method

The camera is an ideal pinhole at height H over flat ground, tilted θ forward from nadir. A sensor point (x, y), with y toward the far edge, defines a ray from the camera. Dividing its forward and right components by its downward component carries that ray to the ground. The four sensor corners make a trapezoid. Ground sample distance is the ground span between the two rays through a pixel's opposite edges, not a single scale for the whole oblique image. If the top ray has no downward component, the far edge reaches the horizon and has no finite footprint. When a site and heading are given, each flat-ground corner is placed from the point below the camera by the WGS 84 geodesic direct problem.

## Equations

- Optical axis a = (sin θ, 0, cos θ); sensor-up u = (cos θ, 0, −sin θ); sensor-right r = (0, 1, 0).
- Ray through sensor point (x, y): d = f a + y u + x r.
- Ground point: ahead = H d_forward / d_down; right = H d_right / d_down, when d_down > 0.
- The equivalent closed form along the image is ahead(y) = H tan(θ + atan(y/f)).
- One-pixel GSD is the distance between the ground points for the pixel's two edges. At the center, across GSD = Hp/(f cos θ) and along GSD = Hp/(f cos² θ), where p is the pixel pitch.
- A mapped corner is reached from the site at distance hypot(ahead, right) and azimuth heading + atan2(right, ahead).

## Symbols and units

H is camera height above the flat ground in meters; θ is tilt from nadir; f, x, y, and pixel pitch p use the same sensor-length unit. Image dimensions are pixels. GSD is reported in centimeters per pixel, footprint offsets in meters, and mapped corners in degrees.

## Domain

Positive height, sensor dimensions, focal length, and image dimensions; 0° ≤ θ < 90°; a field of view below 170°. The image center, near edge, and their pixel spans must meet the ground. The far outputs are omitted with `BEYOND_HORIZON` when the top of the image reaches or passes the horizon.

## Approximations

The ray intersection is exact for a pinhole camera over a flat plane. It omits lens distortion, roll, terrain relief, buildings, Earth curvature, and haze. A real calibrated camera and a surface model are needed when those effects matter. The mapped corners use the ellipsoidal geodesic for their plan position, but their ahead and right offsets still come from the flat-ground camera model.

## Worked example

- sourcePublisher: Andrew Lassiter
- sourceTitle: UAS oblique photo simulation
- sourceEdition: GitHub commit 6c93edd08609a0a0430d92b744995d074bd6a270, dated 2023-02-09 and read at the pinned source on 2026-10-13
- sourceLocator: `uas_oblique_sim.m`, fixed camera parameters and ground sample distance calculation
- independent: yes
- inputs: the source's fixed Canon camera, 24 mm focal length, 22.3 × 14.9 mm format, 0.0043 mm pixel pitch, 100 m flying height, 45° tilt. Its 14.9 mm height rounds to 3,465 pixel rows, spanning 14.8995 mm.
- outputs: minimum 2.1 cm, median 3.6 cm, and maximum 7.5 cm GSD in the direction of tilt. The tool gives 2.0871 cm at the near edge, 3.5833 cm at the center, and 7.5333 cm at the far edge before the source's one-decimal rounding.
- tolerance: 0.05 cm, half the source program's printed 0.1 cm
- verifiedBy: golden vector v016, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py`: 15 closed-angle cases independent of the core's vector ray casting, including five camera formats, tilts from 0° to 60°, heights from 25 m to 304.8 m, the horizon branch, missing inputs, and Lassiter's independently published row-by-row MATLAB calculation at its printed precision. Golden vector v015 also places all four corners with GeographicLib GeodSolve rather than the Rust GeographicLib port.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `oblique_45_degrees_is_coarser_than_nadir_and_a_trapezoid`: at 45°, center GSD has the closed-form relation to nadir GSD and the far footprint edge is wider than the near edge.
- `core/crates/gp-drone/tests/drone.rs` `oblique_gsd_invariants`: at nadir the center scales match and the footprint is symmetric; increasing tilt worsens center GSD; near GSD stays finer than center and far stays coarser; doubling height doubles every GSD.
