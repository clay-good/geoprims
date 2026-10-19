## ADDED Requirements

### Requirement: Workflow inputs filled from profiles
A workflow input MAY declare `profile: {kind, field}`. For each kind a workflow names, the page SHALL offer "Use my <kind>" listing the reader's saved profiles of that kind, and choosing one SHALL fill every input declared for it, visible or under "More options". The assumptions list SHALL then read "<Kind>: from profile <name>" while those values stand. The chain runner SHALL receive the filled values as ordinary inputs.

#### Scenario: Mapping flight from a camera and a drone
- **WHEN** an operator opens "Plan a mapping flight" and chooses "Use my camera" (P1) and "Use my drone" (M3E)
- **THEN** sensor, lens, pixel, flight-time, and reserve inputs fill from the two profiles, the plan recomputes, and the assumptions list names both profiles

#### Scenario: Profile field must exist
- **WHEN** a workflow declares a profile kind whose field map does not include the input's tool field
- **THEN** the workflow lint fails naming the workflow, input, and kind
