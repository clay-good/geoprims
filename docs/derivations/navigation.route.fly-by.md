<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fly-by turn lead distance (`navigation.route.fly-by`)

## Method

An aircraft turning from one course to another follows an arc, and that arc is tangent to both courses. So the turn has to begin before the waypoint, by exactly the distance from the waypoint to where the arc touches the inbound course — the lead distance. Two lengths settle everything: the radius of the turn, and how sharp the course change is. In a coordinated level turn the horizontal component of lift provides the centripetal force while the vertical component holds the aircraft up, which fixes the radius from the speed and the bank angle alone. A turn rate may be given instead of a bank, in which case the radius comes from the rate and the bank that would produce it is reported. The tangent geometry then gives the lead, and the arc and the time in the turn follow.

## Equations

- Radius from bank: R = V² / (g tan φ), the coordinated-turn relation.
- Radius from turn rate: R = V / ω, and the bank that produces it, tan φ = V ω / g.
- Lead distance: L = R · tan(Δψ / 2), the tangent length from the waypoint.
- Turn angle Δψ, the course change, taken the short way round; the direction is which way that is.
- Arc length R · Δψ and turn time Δψ / ω, with ω = V / R.

## Symbols and units

V is the true airspeed (ground speed in still air), φ the bank angle, ω the turn rate, Δψ the course change, R the radius, L the lead distance, g = 9.80665 m/s². Courses are true bearings in degrees; distances are reported in the chosen unit.

## Domain

Any inbound and outbound course, any positive speed, and either a bank angle or a turn rate. As the course change approaches 180° the tangent grows without bound and the lead distance with it, which is why a large course change is flagged as better flown as a fly-over than as a fly-by. A zero course change needs no turn.

## Approximations

None in the geometry: the tangent construction and the coordinated-turn relation are both exact for the model. The model is the approximation, and it is still air, constant speed, level flight, and an instantaneous roll. Real rolls take a second or two at each end, which lengthens the lead slightly, and any wind moves the ground track away from this entirely.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Flying Handbook, FAA-H-8083-15B
- sourceEdition: FAA-H-8083-15B
- sourceLocator: Chapter 5, the standard-rate turn — 3° per second, 360° in two minutes
- independent: yes
- inputs: 120 kt, a 90° course change from 360° to 090°, at the standard rate of 3°/s
- outputs: radius 1179.0198184247606 m, lead distance the same, bank 18.24263059046449°, arc 1852.0000000000002 m, turn time 30 s
- tolerance: exact to the last bit of a double
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-23

The standard-rate turn is a definition, not a measurement, and that is what makes it a good check: it fixes the answer without using the tool's formula. If 360° takes two minutes, then at 120 knots the aircraft flies 4 NM around a full circle, so the radius is V·T/(2π) = 1179.0198184247606 m. The tool returns that number in every digit a double carries, reached by a different route — V/ω. The bank angle it reports for that rate, 18.24263059046449°, is atan(Vω/g) to the last bit as well.

Two more things fall out exactly and are worth stating because they use none of the same arithmetic. A 90° turn at 3°/s takes 30 seconds, which the tool gives as 30.000000000000004 s. And 120 knots for 30 seconds is one nautical mile: the reported arc length is 1852.0000000000002 m. Neither was arranged; they follow from the published definition of the turn.

The tool's own example uses a bank angle rather than a rate — 25° at 120 kt — where R = V²/(g tan 25°) = 833.3860599902922 m, and since the course change is 90° the lead is R·tan 45°, which is R again. Both figures are reproducible by hand from the two equations above.

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `fly_by_matches_the_standard_rate_turn`: at eight speeds, the radius against V·T/(2π) from the two-minute definition and the bank against atan(Vω/g), with the arc length checked as speed times time rather than as R·Δψ
- `tools/vectors/gen_route.py`: 22 vectors from the coordinated-turn relation evaluated in Python, across speeds, banks, and course changes
- `core/vectors/navigation.route.fly-by.jsonl`: 23 vectors, the last this worked example

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `fly_by_invariants`: giving a bank angle and giving the turn rate that same bank produces yield the same radius, which ties the two entry points to each other; the lead distance is R·tan(Δψ/2), so a 90° turn has a lead exactly equal to its radius and a 60° turn one of R/√3; the radius falls as the bank steepens and grows as the square of the speed, checked by doubling rather than by the formula; the arc length is the speed times the turn time, computed from outputs that reach it by different paths; the direction is the short way round and reverses when the inbound and outbound courses are swapped, leaving the radius and lead unchanged; and a course change large enough to make a fly-by impractical raises `FLY_OVER_RECOMMENDED` while an ordinary one does not
