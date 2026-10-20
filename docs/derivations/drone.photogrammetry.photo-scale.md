<!-- Derivation note (trust/correctness-program layer A). -->
# Photo scale and flying height (`drone.photogrammetry.photo-scale`)

## Method

A truly vertical photo is a central projection: the image and the ground are similar triangles meeting at the lens. The scale at a ground point is the focal length over the lens's height above that point. The tool finds that height from whichever one of three things is given (the flying height above a datum with the ground's elevation, a scale number, or a target GSD with the pixel pitch) and reports the scale, both heights, the GSD when the pixels are known, and the ground distance of a photo distance.

## Equations

- S = f / (H − h), written 1:n with n = (H − h) / f (EM 1110-1-1000 eq. 10-1)
- D = d (H − h) / f (eq. 10-3)
- H − h = n f for a scale 1:n; H − h = GSD × f / pitch for a target GSD
- GSD = sensor width × (H − h) / (f × image width), the GSD tool's expression, or pitch × (H − h) / f

## Symbols and units

f is the focal length, H the flying height above the datum, h the ground elevation above the same datum, d a photo distance, D a ground distance, n the scale number, and pitch the size of one pixel. All lengths take any length unit; the scale number is a pure number.

## Domain

The focal length, the height above ground, the scale number, the GSD, and the pixel pitch must be more than zero. Exactly one of the flying height, the scale number, or the target GSD is given; a GSD needs the pixel pitch.

## Approximations

Exact for a truly vertical photo of ground at elevation h. Tilt and relief make the scale vary across a real frame (eq. 10-8 for tilt), so a ground distance from one photo distance is exact only where the ground is at h.

## Worked example

- sourcePublisher: Penn State College of Earth and Mineral Sciences
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems
- sourceEdition: online course text, read 2026-10-19
- sourceLocator: Lesson 4, Geometry of Vertical Image, Scale from Film Camera (courses.ems.psu.edu/geog892/node/657); Designing a Flight Route, Flight Altitude Computations (node/658)
- independent: yes
- inputs: a 6 in lens at 6,000 ft above mean terrain; and a 50 mm lens with 10 µm pixels for a 0.30 m GSD
- outputs: 1:12,000; and 1,500 m above ground
- tolerance: 1e-9 relative
- verifiedBy: golden vectors v001 and v002
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_classic.py` works every vector in Python from the equations above, independent of the core.
- `core/vectors/drone.photogrammetry.photo-scale.jsonl` holds 24 vectors: the two published examples, ten heights with photo distances, five scale numbers, three GSDs from the pixel pitch, and four refusals.

## Invariants

- `core/crates/gp-drone/tests/classic.rs` `photo_scale_invariants`: scale from a height and the height from that scale round-trip; a photo distance to ground and back returns it; the Penn State 1 ft GSD gives 10,000 ft; and from the sensor and image width the GSD equals the GSD tool's bit for bit.
