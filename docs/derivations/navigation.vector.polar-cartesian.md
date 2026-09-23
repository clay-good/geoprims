<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Polar and Cartesian components (`navigation.vector.polar-cartesian`)

## Method

A magnitude and a direction into x, y and z components, or back. The arithmetic is one line of trigonometry; the **convention** is the whole problem.

Navigators measure angles **clockwise from north**. Mathematicians measure them **counterclockwise from east**. The two are a reflection of each other, and code written in one and read in the other produces a vector that is wrong in a way that looks plausible: at 45° the two conventions agree in magnitude and differ in which axis got which component, so a wind that should push you east pushes you north instead.

So the tool takes the convention explicitly and reports which one it used:

- navigational: x = m·cos e·sin θ, y = m·cos e·cos θ
- mathematical: x = m·cos e·cos θ, y = m·cos e·sin θ

With an elevation angle e the vector is three-dimensional and z = m·sin e, with the horizontal components scaled by cos e so the magnitude is preserved.

## Equations

- Navigational: x = m cos e sin θ, y = m cos e cos θ, z = m sin e.
- Mathematical: x = m cos e cos θ, y = m cos e sin θ, z = m sin e.
- Back: m = √(x² + y² + z²); θ = atan2 in the convention's own order; e = asin(z/m).

## Symbols and units

Give `magnitude` and `direction` (and `elevation` for 3D), or `x` and `y` (and `z`). `convention` is `navigational` (the default) or `mathematical`. Out come all of them, plus `convention_used`.

## Domain

Any magnitude and any angle. A zero vector has no direction, and the tool says so rather than returning an arbitrary one.

## Approximations

None. The only inexactness is the sine and cosine of the angle you gave.

## Worked example

- sourcePublisher: the definitions of the two angle conventions
- sourceTitle: navigational bearings clockwise from north; mathematical angles counterclockwise from east
- sourceEdition: definition
- sourceLocator: x = m sin θ, y = m cos θ for a bearing; x = m cos θ, y = m sin θ for a mathematical angle
- independent: yes
- inputs: 25 cases, both conventions at the cardinal directions and at 45°, elevations from −75° to +90°, and a bearing a degree short of north
- outputs: the components, and the magnitude they reconstruct
- tolerance: 1e-9
- verifiedBy: golden vectors v001 to v025, run by the core on every build
- verifiedOn: 2026-09-23

Every case is computed in the generator from the formula for its own convention. The pairs that matter are the same angle under both conventions: at 45° they differ only in which component is which, which is exactly the mistake that survives a casual test.

## Differential tests

- `tools/vectors/gen_nav_vector.py`: 16 of the 25 vectors, from the definitions
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.vector.polar-cartesian.jsonl`: 25 cases across both conventions

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `polar_cartesian_invariants`: the two conventions are reflections, so a bearing θ navigational gives the components a mathematical angle of 90° − θ gives — one identity that catches a swap anywhere; the round trip holds, components back to magnitude and direction and out again; the magnitude is preserved with an elevation, since the horizontal part is scaled by cos e; due north in the navigational convention is +y and due east is +x, while in the mathematical one due east is +x and 90° is +y; and the reported convention is the one that was asked for
