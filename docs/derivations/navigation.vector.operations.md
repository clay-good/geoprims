<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Vector operations (`navigation.vector.operations`)

## Method

Adds any number of 2D or 3D vectors head to tail, and for exactly two also gives their difference, dot product, cross product, the angle between them, and the projection of one on the other.

The addition is component-wise and needs no explanation. What the tool is for is having the rest in the same place and the same convention: a wind triangle, a set of forces, a pair of offsets. The direction of the sum is reported in the convention asked for, so a set of navigational bearings adds to a navigational bearing rather than to a mathematical angle nobody wanted.

The cross product is defined only in three dimensions. Given two 2D vectors the tool reports the scalar z-component, which is what the 2D "cross product" means and is the signed area of the parallelogram they span.

## Equations

- Sum: componentwise.
- a·b = Σ aᵢbᵢ.
- a × b = (a_y b_z − a_z b_y, a_z b_x − a_x b_z, a_x b_y − a_y b_x).
- angle = acos(a·b / (|a||b|)); projection of a on b = a·b / |b|.
- Unit vector = a / |a|.

## Symbols and units

`vectors` is a list, each with `x`, `y` and optionally `z`. `convention` picks how the direction is reported; `scale` multiplies the result. Out come the sum and its magnitude and direction, the unit vector, and — for two inputs — the difference, dot, cross, angle and projection.

## Domain

Any vectors, any count. A zero sum has no direction, and the angle between a zero vector and anything is undefined; both are reported as such rather than as zero.

## Approximations

None: this is exact arithmetic on the numbers given.

## Worked example

- sourcePublisher: the definitions of the vector operations
- sourceTitle: componentwise addition; the dot, cross and projection formulas
- sourceEdition: definition
- sourceLocator: a·b = Σ aᵢbᵢ; a × b by the determinant; angle = acos(a·b/|a||b|)
- independent: yes
- inputs: 22 sets, including perpendicular unit vectors, a vector and its negation summing to zero, three vectors in a row, a 3-4-5 triangle, 3D triples, and a pair whose dot product is zero
- outputs: the sum, its magnitude, and the dot product where two vectors were given
- tolerance: 1e-12
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The reference computes each quantity from its definition. The cases that carry information are the degenerate ones — a vector plus its negation, a zero input, two perpendicular vectors whose dot product must be exactly zero rather than nearly.

## Differential tests

- `tools/vectors/gen_nav_vector.py`: 12 of the 18 vectors, from the definitions
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.vector.operations.jsonl`: 22 sets in two and three dimensions

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `vector_operations_invariants`: addition is commutative, so reordering the inputs does not move the sum; a vector plus its negation is exactly zero; the dot product of perpendicular vectors is zero and the angle between them exactly 90°; the cross product is perpendicular to both, so its dot with each is zero; |a × b|² + (a·b)² = |a|²|b|², the identity that ties the two products together and fails if either has a sign wrong; the unit vector has magnitude one and the same direction; and scaling multiplies the magnitude and leaves the direction alone
