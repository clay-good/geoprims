<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Under-keel clearance with squat (`navigation.vessel.under-keel-clearance`)

## Method

Under-keel clearance is a stack of four measured or chosen numbers and one estimate. The water over the chart datum is the charted depth plus the tide height at the time. What the ship draws is her static draft plus squat, the bodily sinkage and trim that shallow water and speed produce. What is left is the water under the keel, and what is left after the margin the master or the port requires is the clearance this tool reports. Every term is shown, because the useful question is not the single number but which term to change: slow down, wait for the tide, or take less draft.

Squat is the only term the tool estimates. It uses Barrass's formula, a regression on more than 600 model and ship measurements, as the 2016 "Mircea cel Batran" bulletin prints it with its parameter ranges. The channel enters through the blockage factor: the midship section area over the channel's cross-section area. Open water is a blockage factor of 0.10, where the coefficient K is 1 and the formula is the familiar C_B V² / 100; a restricted channel is 0.25, where K is 2 and the squat doubles for the same speed. Between them, K = 5.74 S^0.76. Barrass's block coefficient also says which end goes down: a full-form ship (C_B above 0.7) squats by the bow, a fine-form ship by the stern, and the other end's sinkage follows his [1 − 40(0.7 − C_B)²] relation.

If the reader has a squat figure of their own, from a trial, a port's table, or another formula, they give it and the tool does the arithmetic with it instead, with no range check of its own.

## Equations

- Water depth = charted depth + tide height.
- Draft with squat = static draft + squat.
- Water under the keel = water depth − (static draft + squat).
- Clearance = water under the keel − margin = charted depth + tide − (draft + squat + margin).
- Squat (Barrass): S_max = K · C_B · V_K² / 100, with V_K in knots.
- Blockage coefficient: K = 5.74 · S^0.76, where S = A_ship / A_channel. S = 0.10 gives K = 1.00 (open water); S = 0.25 gives K = 2.00 (a restricted channel).
- The other end of the ship: S_other = [1 − 40 (0.7 − C_B)²] · S_max.

## Symbols and units

Charted depth, tide, draft, squat, margin, and every output length in meters by default; any length unit is accepted, and the clearance is reported in feet beside meters. Speed through the water V_K in knots, because Barrass's constant 100 carries the knots in it. C_B is the block coefficient, dimensionless. S is the blockage factor, dimensionless. Depth-to-draft ratio h/T is water depth over static draft.

## Domain

The arithmetic of the stack holds for any numbers: the draft must be positive, the margin not negative, and the water depth positive. The squat estimate is held to the ranges the source publishes, and refuses outside them with `OUT_OF_DOMAIN` naming the range:

- Block coefficient C_B from 0.5 to 0.85 (Table 1).
- Depth-to-draft ratio h/T from 1.1 to 1.4 (Table 1). Above 1.4 squat is small and the formula is outside its fit; below 1.1 the ship is deeper than the data.
- Blockage factor S from 0.10 to 0.25, for the blockage-factor channel.

A clearance below the reader's margin is not an error: it is answered, with the shortfall in the sentence and the `BELOW_MARGIN` warning.

## Approximations

Squat is an estimate, not a measurement, and this one is deliberately the cautious member of its family: published comparisons of the PIANC formulas put Barrass's prediction above most of the others for the same ship. It knows the ship only through her block coefficient and the channel only through the blockage factor, so it carries nothing of heel, list, initial trim, sea state, swell, passing ships, bank effect, water density, or the state of the dredged bottom. The charted depth is the chart's, not today's. The margin is the reader's own, and the tool makes no judgment about it: the master and the port authority decide what is acceptable.

## Worked example

- sourcePublisher: "Mircea cel Batran" Naval Academy Scientific Bulletin (Constanta), Deleanu and Panaitescu
- sourceTitle: Comparison between formulas of maximum ship squat
- sourceEdition: Volume XIX, Issue 1, 2016, pp. 105-111 (DOI 10.21279/1454-864X-16-I1-018)
- sourceLocator: Tables 3 and 4 and equations (34) to (36): a 7,800 tdw cargo ship, C_B 0.667, draft 7.76 m, beam 17.1 m, at 8 knots in a rectangular canal 123 m wide and 10 m deep, so A_S = 0.98 B T = 130.0421 m², A_C = 1,230 m², S = 0.105725, and Barrass's squat is printed as 0.40842 m
- independent: no
- inputs: charted_depth 10.0 m, tide 0 m, draft 7.76 m, margin 0.3 m, speed 8 kt, block_coefficient 0.667, channel blockage-factor, blockage_factor 0.105725 (golden vector v017)
- outputs: squat 0.4442 m, other end 0.4249 m, water under the keel 1.7958 m
- tolerance: the paper's printed 0.40842 m is 0.017 m below the 0.42486 m its own equations (2) to (4) give for that canal, because its arithmetic takes K as 1.0 where equation (3) gives 1.0406 for S = 0.105725; the tool follows the printed equations
- verifiedBy: golden vector v017 and `core/crates/gp-navigation/tests/vessel.rs` `under_keel_clearance_invariants`
- verifiedOn: 2026-11-06

The example is marked **not independent** on purpose. The source publishes the formula, its coefficients, and its parameter ranges, and the tool reproduces those exactly; but the one worked case it prints cannot be reproduced from them, since its K does not follow its own equation (3). Until a worked case turns up that the printed formula reproduces (the USACE notes that restate Barrass with examples, CHETN-I-72 and CHETN-IX-19/20, are behind the ERDC and DTIC bot checks), this tool stays experimental rather than claiming a verification that did not happen.

## Differential tests

- `tools/vectors/gen_ukc.py` builds the 24 vectors in `core/vectors/navigation.vessel.under-keel-clearance.jsonl` from a second implementation of the stack and of Barrass's equations in Python: eight stacks with the squat given (including the spec's Stack and Negative clearance scenarios), five open-water squats, three confined-channel squats, four worked from a blockage factor (the first of them the 2016 paper's own canal), and four refusals.
- The two implementations agree to 1e-9 m on every vector.

## Invariants

- `core/crates/gp-navigation/tests/vessel.rs` `under_keel_clearance_invariants`: the clearance is linear in the tide with slope +1 and in the margin with slope −1; the clearance is always the water under the keel less the margin; squat grows with the square of the speed (doubling the speed quadruples it) in both channel types; a confined channel squats twice as deep as open water; open water is exactly C_B V² / 100; the other end is never deeper than the maximum, and the two ends agree at C_B 0.7.
- `core/crates/gp-navigation/tests/vessel.rs` `clearance_stack_scenarios`: the spec's Stack scenario to 1e-9 m, the Negative clearance scenario with `BELOW_MARGIN`, and a sentence that carries no verdict word.
- `core/crates/gp-navigation/tests/vessel.rs` `published_ranges_are_refused_outside`: h/T = 2.0, C_B = 0.95, and S = 0.05 each refuse with `OUT_OF_DOMAIN` naming the published range, while the same water with the squat given still answers.
