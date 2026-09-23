<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Plate motion between epochs (`geodesy.datum.plate-motion`)

## Method

A point on a tectonic plate moves because the plate rotates about a pole through the Earth's center. Given that rotation vector, the velocity of any point is the cross product of the rotation vector with the point's geocentric position — a rigid body turning, nothing more. The ITRF2020 plate motion model publishes such a vector for each of thirteen plates, together with an origin rate bias that accounts for the frame's own origin drifting relative to the Earth's center of mass. The position is converted to geocentric Cartesian coordinates, the velocity is formed, the bias added, the whole multiplied by the elapsed years and added on, and the result converted back. A site velocity, where one is known from a nearby station, replaces the model entirely, because a measured velocity beats a modelled one.

## Equations

- Plate velocity: v = ω × X + ORB, with ω the plate's rotation vector and ORB the origin rate bias.
- Propagation: X(t₂) = X(t₁) + v · (t₂ − t₁).
- Written out: v_x = ω_y Z − ω_z Y, v_y = ω_z X − ω_x Z, v_z = ω_x Y − ω_y X, each plus its ORB component.
- The displacement is resolved into east, north, and up at the position; the reported figure is the horizontal part.

## Symbols and units

ω is the plate's rotation vector in arcseconds per year about the X, Y, and Z axes, converted to radians per year before use; X is the geocentric position in meters on GRS80; ORB is in meters per year; t is the epoch in decimal years. Velocities are reported in millimeters per year, displacements in meters.

## Domain

Thirteen plates — AMUR, ANTA, ARAB, AUST, CARB, EURA, INDI, NAZC, NOAM, NUBI, PCFC, SOAM, SOMA — or a site velocity in place of any of them, between epochs 1980 and 2100. Outside that span the linear velocity is extrapolation rather than model. Positions inside Bird's PB2002 orogens are flagged: those are the belts where the plates are not rigid and the model does not describe the ground.

## Approximations

Rigid rotation, linear in time, which is the model as published. It fits stable plate interiors to about 0.2 mm a year and can be wrong by centimeters a year in a deforming zone. Nothing episodic is in it — an earthquake, subsidence, a landslide — and no approximation in the arithmetic comes close to those in size.

## Worked example

- sourcePublisher: PROJ contributors; rotation pole and origin rate bias from Altamimi and others
- sourceTitle: PROJ `+proj=helmert` through pyproj; ITRF2020 Plate Motion Model, Geophysical Research Letters 50
- sourceEdition: PROJ 9.3.0, pyproj 3.6.1; GRL e2023GL106373
- sourceLocator: `+proj=helmert +convention=position_vector +t_epoch=2010.0 +drx=0.000045 +dry=-0.000666 +drz=-0.000098 +dx=0.00037 +dy=0.00035 +dz=0.00074`, the NOAM pole of Table 1 with the origin rate bias of Table 2
- independent: yes
- inputs: 38.5, −98, 500 m on NOAM, from epoch 2010.0 to 2026.7
- outputs: latitude 38.49999948673457, longitude −98.00000278813535, height 500.00225875193865 m, a displacement of 0.2498206860 m
- tolerance: 1e-8 m, twice the largest difference seen
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-23

PROJ was given only the published constants and left to do the work: it forms the cross product, scales it by the elapsed years, and carries the position through the Cartesian round trip in its own code. It lands 4.0 nanometres from the tool in latitude, 1.2 in longitude, and 2.4 in height.

The check distinguishes the origin rate bias, which is the part most easily left out. Run with the rotation pole alone and PROJ comes back 13.8 mm north and 5.3 mm east of the tool over these 16.7 years — six orders of magnitude larger than the agreement with the bias included. So the example does not merely confirm that a cross product was computed; it confirms which model was computed.

## Differential tests

- `tools/vectors/gen_datum.py`: 19 vectors from PROJ's `+proj=helmert` with the plate motion rates of its `data/ITRF2020`, through pyproj, across plates, positions, and epoch spans
- `core/crates/gp-geodesy/tests/datum.rs` `plate_motion_invariants`: the linearity, the reversibility, the rigid-rotation structure, and the deformation-zone flag
- `core/vectors/geodesy.datum.plate-motion.jsonl`: 22 vectors, of which v001 is this worked example

## Invariants

- `core/crates/gp-geodesy/tests/datum.rs` `plate_motion_invariants`: propagating to the same epoch moves nothing; the displacement is exactly linear in the elapsed time, so ten years is ten times one year, which is what distinguishes a velocity model from anything with curvature in it; propagating forward and then back returns the position to within nine nanometres; the reported velocity does not depend on the epochs at all, only on the position and the plate; a site velocity given directly is reproduced in the output and used in place of the plate; the horizontal displacement is east and north recomposed, to 3e-17 m; the Pacific plate moves a point 1.19 m over the same span the North American one moves it 0.25 m, nearly five times further, which is the ordering the published poles imply; and a point on the San Andreas system raises the deformation-zone warning while one in Kansas does not
