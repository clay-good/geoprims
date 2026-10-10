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

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: geographiclib 2.1 for Python, Geodesic.WGS84.Direct
- sourceEdition: geographiclib 2.1, run 2026-10-10
- sourceLocator: tools/vectors/gen_glide_ring.py, first case: Direct from 39.86°, −104.67° at bearings 0° to 330° for the range each bearing allows
- independent: yes
- inputs: height 5,000 ft AGL, glide ratio 9, TAS 70 kt, headwind 20 kt from 270°, at 39.86°, −104.67°
- outputs: still-air range 7.41 NM, range with wind 5.29 NM, glide angle 6.34°, sink rate 788 ft/min; the ring reaches 39.98353°, −104.67000° due north (the still-air range, across the wind), 39.85982°, −104.46391° due east (9.52 NM downwind), and 39.85994°, −104.78450° due west (5.29 NM upwind)
- tolerance: 1e-9° for each ring point; 1e-9 relative for the ranges, angle, and sink rate
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-10-10

GeographicLib for Python places twelve points of the ring, every 30° of bearing, in each of 15 glides, v006 through v020: from 60° S to 70° N and across the 180° meridian, heights of 1,500 to 35,000 ft, glide ratios of 7 to 45, winds up to 60% of the airspeed, and still air. The range along each bearing, and the glide angle and sink rate, are worked in the generator from the equations above; the library checks where the tool puts a point at a given range and bearing.

## Differential tests

- `tools/vectors/gen_glide_ring.py`: twelve ring points per glide from GeographicLib for Python, with the ranges, glide angle, and sink rate, at 15 glides (within 1e-9° and 1e-9 relative)
- `tools/vectors/gen_aviation.py`: the same glide relations written separately in Python, at five cases with headwind, tailwind, and calm, checking still-air range, range with wind, and sink rate (within 1e-9 relative). It checks the code against the stated equations, not against a separate method
- `core/vectors/aviation.performance.glide.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `the_glide_ring_reaches_farther_downwind`: measured back with the geodesic inverse (the same library the core uses for the direct problem), the ring's upwind point is at the range with wind, its downwind point at D0 × 90 ÷ 70, and its crosswind point at the still-air range (each within 0.01 NM), so downwind to upwind is 9 to 5; a wind without a direction is refused, and with no position the ring is empty
