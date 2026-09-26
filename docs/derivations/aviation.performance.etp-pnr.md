<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Equal time point and point of no return (`aviation.performance.etp-pnr`)

## Method

At the equal time point (ETP), the time to go on equals the time to turn back: (D − x) / O = x / H, so x = D · H / (O + H). The point of no return (PNR) is where the time out plus the time back uses the safe endurance E: t + t · O / H = E, so t = E · H / (O + H), and the PNR is t · O from departure. The groundspeeds come from the true airspeed and the wind by the wind triangle of `aviation.wind.heading-groundspeed`, on the course out and its reciprocal back, or are given directly. The share H / (O + H) is computed once, so with no wind it is exactly one half.

## Equations

- O = GS on the course out; H = GS on the reciprocal (back), both by GS = TAS · cos WCA − W · cos(WD − course)
- ETP distance x = D · H / (O + H); time to the ETP = x / O
- Safe endurance E = (usable fuel − reserve) / burn, or given
- PNR time t = E · H / (O + H); PNR distance = t · O

## Symbols and units

D, x, and the PNR distance in nautical miles; O, H, TAS, and W in knots; course and wind direction in degrees true; E and t in hours internally, reported in minutes; fuel in US gallons and burn in US gallons per hour.

## Domain

A leg distance above zero. Both groundspeeds above zero: a headwind at least the TAS, or a crosswind stronger than it, has no answer (`NO_SOLUTION`). A wind needs a course. The fuel inputs need a burn, and the reserve must leave some endurance. A PNR past the destination is reported as it is; the diagram says so.

## Approximations

One steady wind over the whole leg, the same both ways, and one true airspeed and burn going on and turning back. A diversion after an engine failure or a loss of pressurization flies at another altitude, speed, and burn, and needs its own groundspeeds (CASA's worked example does exactly that, at 10,000 ft). The PNR is back to departure, not to an en route alternate.

## Worked example

- sourcePublisher: Civil Aviation Safety Authority (Australia)
- sourceTitle: Guidelines for aircraft fuel requirements, AC 91-15, Annex B (sample fuel calculations, Beechcraft B200)
- sourceEdition: AC 91-15 v1.2
- sourceLocator: Annex B pages B9 and B10, Table 9 (calculation of critical point)
- independent: yes
- inputs: route 906 NM (Darwin to Cairns), GS 250 kt on to Cairns and 290 kt back to Darwin (TAS 270 kt with a 20 kt wind)
- outputs: critical point 487 NM from Darwin and 419 NM from Cairns (the core gives 486.56 NM and 419.44 NM)
- tolerance: 0.5 NM (the table prints whole miles)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-25

The annex works no point-of-no-return example. The PNR is checked by its defining property instead: the time out plus the time back equals the safe endurance.

## Differential tests

- `tools/vectors/gen_flight_plan.py`: groundspeeds from the law-of-cosines wind triangle, a different form from the core's, and the two ratios in Python; within 1e-9
- `core/vectors/aviation.performance.etp-pnr.jsonl`: those vectors, the published example, the no-wind half-way case, and the refused cases, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/flight_planning.rs` `etp_with_no_wind_is_exactly_halfway`: with no wind the ETP is exactly D / 2
- `core/crates/gp-aviation/tests/flight_planning.rs` `etp_moves_toward_destination_in_a_headwind_out`: a headwind out puts the ETP past the midpoint and a tailwind before it
- `core/crates/gp-aviation/tests/flight_planning.rs` `pnr_leaves_the_reserve_on_return`: the time out to the PNR plus the time back equals the safe endurance
