<!-- Derivation note (trust/correctness-program layer A). -->
# Relief displacement and object height (`drone.photogrammetry.relief-displacement`)

## Method

On a truly vertical photo, a point above the datum images farther from the principal point than the point below it, along the radial line through the principal point. For a vertical object, the image of its top is displaced outward from the image of its base. Similar triangles relate that displacement to the object's height and the flying height above its base, so the tool solves the relation for whichever one of the four quantities is left out.

## Equations

- d = r h / (H − h_base) (EM 1110-1-1000 eq. 10-6, with H measured above the object's base)
- h = d (H − h_base) / r (eq. 10-7)
- r = d (H − h_base) / h; H = r h / d + h_base

## Symbols and units

d is the displacement on the photo from the base's image to the top's, r the radial distance from the principal point to the top's image, h the object's height, H the flying height above the datum, and h_base the elevation of the object's base above the same datum. Photo lengths and ground lengths may use different units; only the ratios d / r and h / (H − h_base) matter.

## Domain

Exactly three of d, r, h, and H are given. The flying height must be above the base; the divisor of the solved form must be more than zero; and d must be less than r, since the base's image lies between the principal point and the top's.

## Approximations

Exact for a truly vertical photo and an object standing straight up. Tilt moves the radial lines; lens distortion shifts r.

## Worked example

- sourcePublisher: McGraw-Hill
- sourceTitle: Elements of Photogrammetry with Applications in GIS (Wolf, Dewitt, and Wilkinson)
- sourceEdition: 4th edition (2014)
- sourceLocator: Example 6-7 (relief displacement of a tower), as cited in the add-photogrammetry-classics design; not read at the source, which is not open
- independent: no
- inputs: H = 535 m above datum, base elevation 259 m, d = 54.1 mm, r = 121.7 mm
- outputs: h = 122.7 m (122.69 m unrounded), matching EM 1110-1-1000 eq. 10-7
- tolerance: 0.05 m (the printed precision)
- verifiedBy: golden vector v001 and `relief_displacement_invariants`
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_classic.py` works every vector in Python from the equations above.
- `core/vectors/drone.photogrammetry.relief-displacement.jsonl` holds 28 vectors: the textbook tower, seven objects each solved for height, displacement, and radial distance, three flying heights, and three refusals.

## Invariants

- `core/crates/gp-drone/tests/classic.rs` `relief_displacement_invariants`: the tower's height to 0.1 m, and solving for each of the four unknowns returns the others within 1e-9.
