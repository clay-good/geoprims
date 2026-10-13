<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Climb gradient and vertical speed (`aviation.performance.climb-gradient`)

## Method

A unit conversion. A gradient is rise over horizontal run, so the vertical speed is the groundspeed times the gradient. Give exactly one of a gradient (in ft/NM, percent, or any slope unit), an angle, or a vertical speed; an angle becomes its tangent, and a vertical speed is divided by the groundspeed. The tool returns the vertical speed, the gradient in ft/NM and in percent, and the path angle.

## Equations

- From an angle θ: gradient G = tan θ
- From a vertical speed V: G = V / GS
- Vertical speed = GS × G; in pilot units, fpm = kt × ft/NM / 60
- Percent = 100 × G; ft/NM = G × 1,852 / 0.3048; angle = atan(G)

## Symbols and units

G is dimensionless rise over run; 1 ft/NM = 0.3048 / 1,852. GS is groundspeed in knots by default; vertical speed in ft/min; angle in degrees. Inside, speeds are m/s.

## Domain

A groundspeed above zero. Exactly one of gradient, angle, or vertical speed (otherwise INVALID_INPUT). An angle strictly between 0° and 90° (otherwise OUT_OF_DOMAIN). The resulting gradient must be positive.

## Approximations

None in the conversion. The answer is a steady average and must use groundspeed, not airspeed; a tailwind raises the rate a gradient needs. It does not know whether the aircraft can climb at that rate.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Procedures Handbook
- sourceEdition: FAA-H-8083-16B (2017)
- sourceLocator: Chapter 1, Figure 1-17, highlighted rate-of-climb example
- independent: yes
- inputs: path angle 2.8°, groundspeed 180 kt
- outputs: required climb rate 892 fpm; the table also labels the rounded gradient as 297 ft/NM
- tolerance: 0.5 fpm, because the source prints whole feet per minute
- verifiedBy: golden vector v006 and `core/crates/gp-aviation/tests/slice2.rs` `climb_gradient_invariants`
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_climb_gradient.py`: a separate Python evaluation of the pilot-unit, ratio, angle, and inverse forms across varied gradients and groundspeeds, plus refused ambiguous, zero, and out-of-domain inputs
- `core/vectors/aviation.performance.climb-gradient.jsonl`: those cases and the printed FAA example, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `climb_gradient_to_fpm`: 200 ft/NM at 120 kt gives 400 fpm, and 400 fpm at 120 kt gives back 200 ft/NM, each within 1e-9
- `core/crates/gp-aviation/tests/slice2.rs` `climb_gradient_invariants`: doubling groundspeed doubles the required vertical speed; equivalent ft/NM, percent, and angle inputs agree; converting the resulting vertical speed back recovers the gradient
- `core/crates/gp-aviation/tests/slice3.rs` `glidepath_vertical_speed`: a 3° path at 120 kt gives about 637 fpm (within 0.5 fpm)
