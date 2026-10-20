<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Climb requirement check (`aviation.performance.climb-requirement`)

## Method

A departure's climb gradient is in feet per nautical mile. At a groundspeed in knots, the airplane covers groundspeed ÷ 60 nautical miles each minute, so the rate of climb needed is the gradient times that distance. This is the relation in the Instrument Procedures Handbook, chapter 1. The gradient is the one the user enters, or else the altitude to reach less the starting altitude, divided by the distance. The groundspeed is the one entered, or else the true airspeed less the headwind. When the user gives a rate of climb, the tool also returns the gradient that rate makes and the margin. A negative margin adds the CLIMB_GRADIENT_SHORT caution.

## Equations

- Gradient from a climb: G = (altitude − start) ÷ distance, with start = 0 ft when not given.
- Groundspeed from the wind: GS = TAS − headwind (a tailwind is a negative headwind).
- Rate needed: R_need = G × GS ÷ 60.
- Gradient you can make: G_made = R × 60 ÷ GS.
- Margin: M = R − R_need; M < 0 adds CLIMB_GRADIENT_SHORT.

## Symbols and units

G in ft/NM by default (any slope unit accepted, such as percent); altitudes in feet; distance in nautical miles; GS, TAS, and headwind in knots; R, R_need, and M in ft/min. A gradient entered directly takes precedence over an altitude and distance, and a groundspeed takes precedence over an airspeed and wind.

## Domain

A gradient, or an altitude and a distance, else INVALID_INPUT at `/gradient`. A distance > 0, else INVALID_INPUT at `/distance`. The gradient must be a climb, > 0, else INVALID_INPUT at `/gradient`. A groundspeed, or a true airspeed, else INVALID_INPUT at `/groundspeed`. The groundspeed used must be > 0, else INVALID_INPUT at `/groundspeed`. The rate of climb is optional.

## Approximations

The check is exact for one steady climb at one constant groundspeed. A real climb rate falls with altitude, heat, and weight, and a tailwind raises the rate needed, so the flight manual's numbers for the conditions and a conservative wind decide the margin. The standard 200 ft per nautical mile assumes crossing the departure end of the runway at 35 ft or more; the published procedure governs.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Instrument Procedures Handbook
- sourceEdition: FAA-H-8083-16B
- sourceLocator: Chapter 1, Figure 1-17 (2.8° at 180 kt requires 892 ft/min, with the rounded gradient labeled 297 ft/NM)
- independent: yes
- inputs: gradient 297 ft/NM, groundspeed 180 kt, rate of climb 1,000 ft/min
- outputs: rate needed 891 ft/min; gradient made 333.3 ft/NM; margin 109 ft/min
- tolerance: exact in the vector (1e-12 relative, 1e-9 absolute); 1 ft/min against the figure's 892, which comes from the unrounded 2.8° path
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/physiology.rs` `climb_requirement_invariants`
- verifiedOn: 2026-10-20

The figure prints the gradient, groundspeed, and rate needed. The 1,000 ft/min rate of climb, and so the gradient made and the margin, are added for the example and are not from the figure.

## Differential tests

- No generator script in `tools/vectors/` names this tool. The vectors' source field says they were worked in Python from the IPH chapter 1 relation, rate = gradient × groundspeed ÷ 60. The arithmetic is direct, so this checks the input paths, units, and the caution, not a separate method.
- `core/vectors/aviation.performance.climb-requirement.jsonl`: 20 vectors. The 297 ft/NM example (v001); a gradient from 3,000 ft less 1,221 ft over 5 NM, 355.8 ft/NM (v002); a groundspeed from an airspeed with a tailwind (v003) and with a headwind (v017); a gradient in percent (v016); ten cases that carry CLIMB_GRADIENT_SHORT; and three INVALID_INPUT refusals: no gradient, no groundspeed, and a zero distance.

## Invariants

- `core/crates/gp-aviation/tests/physiology.rs` `climb_requirement_invariants`: 297 ft/NM at 180 kt needs exactly 891 ft/min; 3,000 ft from 1,221 ft in 5 NM is 355.8 ft/NM; a 20 kt tailwind with a 450 ft/min climb at 250 ft/NM carries CLIMB_GRADIENT_SHORT; and the gradient made, taken back through the rate relation at 120 kt, gives the 700 ft/min entered.
