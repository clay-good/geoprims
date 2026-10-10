<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Glide range with wind (`aviation.performance.glide`)

## Method

A steady glide at the given glide ratio covers height × glide ratio over still air. True airspeed is taken as the horizontal airspeed, so a steady along-track headwind scales the range by the groundspeed over the airspeed, (TAS − headwind) ÷ TAS. The sink rate is TAS ÷ glide ratio and the time aloft is height ÷ sink rate. With a position, the tool draws a reachable ring: 72 bearings at 5° steps, each at the range the wind component along that bearing allows, placed by the geodesic direct problem on WGS84 (Karney 2013, via geographiclib-rs). A headwind with a position needs the wind direction.

## Equations

- Still-air range: D0 = h × L/D.
- Range with wind: D = D0 × (V − HW) ÷ V.
- Glide angle: γ = atan(1 ÷ L/D).
- Sink rate: w = V ÷ L/D; time aloft: t = h ÷ w.
- Ring: for bearing β = 0°, 5°, …, 355°, along-track headwind HW_β = HW · cos(β − W_from); range D_β = max(D0 × (V − HW_β) ÷ V, 0), placed by the geodesic direct problem from the position.

## Symbols and units

h height above terrain (feet by default, computed in meters); L/D glide ratio (dimensionless); V best-glide true airspeed and HW headwind (knots by default; a negative headwind is a tailwind); W_from the direction the wind blows from, in degrees. Ranges are in nautical miles (1,852 m), sink rate in ft/min, time in minutes.

## Domain

h > 0, V > 0, L/D from 1 to 80; the headwind must be less than V (otherwise OUT_OF_DOMAIN). For the ring, both latitude and longitude, and the wind direction whenever the headwind is not zero.

## Approximations

TAS stands in for the horizontal airspeed, which is under 1% off for glide ratios of 7 or more (cos γ ≥ 0.99). One glide ratio, a straight glide, and a steady wind that does not change with height; turns, a windmilling propeller, and flaps all shorten the real glide. The ring's wind is the plain along-bearing component, with no crosswind drift.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite flight-performance spec, scenario "Glide into headwind"
- sourceEdition: add-aviation-suite change; method from FAA-H-8083-3C (2021)
- sourceLocator: openspec/changes/add-aviation-suite/specs/aviation/flight-performance/spec.md, requirement "Glide range" (still-air range ≈ 7.41 NM, headwind range ≈ 5.29 NM); glide angle, sink rate, and time computed in Python from the equations above
- independent: no
- inputs: height 5,000 ft AGL, glide ratio 9, TAS 70 kt, headwind 20 kt
- outputs: still-air range 7.41 NM, range with wind 5.29 NM, glide angle 6.34°, sink rate 788 ft/min, time aloft 6.3 min
- tolerance: 0.005 NM on the ranges (spec); 1e-9 relative in the golden vector
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/slice2.rs` `glide_into_headwind`
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: the same glide relations written separately in Python, at five cases with headwind, tailwind, and calm, checking still-air range, range with wind, and sink rate (within 1e-9 relative). It checks the code against the stated equations, not against a separate method
- `core/vectors/aviation.performance.glide.jsonl`: those five vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `the_glide_ring_reaches_farther_downwind`: measured back with the geodesic inverse (the same library the core uses for the direct problem), the ring's upwind point is at the range with wind, its downwind point at D0 × 90 ÷ 70, and its crosswind point at the still-air range (each within 0.01 NM), so downwind to upwind is 9 to 5; a wind without a direction is refused, and with no position the ring is empty
