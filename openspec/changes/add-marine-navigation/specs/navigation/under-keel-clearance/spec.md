## Purpose

Computes clearance under the keel from charted depth, tide, draft, a squat estimate, and the user's margin, showing the whole stack and never calling the result safe.

## ADDED Requirements

### Requirement: Clearance stack
The tool SHALL compute under-keel clearance = charted depth + tide height − (draft + squat + margin), in meters and feet, from inputs in either unit, with the chart datum named (from the tide tool when sent). It SHALL draw the stack and SHALL state that the master and the port authority govern the margin.

#### Scenario: Stack
- **WHEN** charted depth 12.0 m, tide +1.5 m, draft 10.8 m, squat 0.6 m, margin 1.0 m
- **THEN** the clearance is 1.1 m and every term is shown

### Requirement: Squat estimate
Squat SHALL be estimated by Barrass's simplified formula (open water: S = C_B·V²/100 m with V in knots, valid for depth-to-draft ratios 1.1 to 1.4; confined channel: 2·C_B·V²/100) or the general form with blockage factor S_b (S = C_B·S_b^(2/3)·V^2.08/20), as restated with parameter ranges in USACE CHETN-I-72 and CHETN-IX-19. Inputs outside the published ranges SHALL return `OUT_OF_DOMAIN` naming the range. The result SHALL name the formula and its source and SHALL label squat an estimate.

#### Scenario: Open-water squat
- **WHEN** C_B = 0.80 and speed 10 kn in open water with h/T = 1.2
- **THEN** squat is 0.80 m (±0.001 m) by the simplified formula

#### Scenario: Out of range
- **WHEN** h/T = 2.0 is entered with the simplified open-water formula
- **THEN** the result is `OUT_OF_DOMAIN` naming 1.1 to 1.4

### Requirement: No verdict wording
The result sentence SHALL report the clearance against the user's margin as numbers ("1.1 m under the keel, 0.1 m more than your 1.0 m margin") and SHALL NOT use "safe", "adequate", or similar words.

#### Scenario: Negative clearance
- **WHEN** the computed clearance is negative
- **THEN** the sentence states the shortfall in meters and the result carries `BELOW_MARGIN`
