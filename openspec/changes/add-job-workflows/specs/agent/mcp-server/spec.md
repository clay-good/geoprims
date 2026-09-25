## ADDED Requirements

### Requirement: Workflows through the meta-tools
The MCP server SHALL expose each workflow with id `workflow.<slug>`. `geoprims_search` SHALL list the workflows a query names in a separate `workflows` field, so tool ranking is unchanged, `geoprims_describe` SHALL return its inputs, steps, and assumptions, and `geoprims_run` SHALL run it through the shared chain runner, returning the summary and each step's tool id, input, and result. No new top-level MCP tools SHALL be added.

#### Scenario: Agent plans a mapping flight
- **WHEN** an agent calls `geoprims_run` with `workflow.mapping-flight` and the workflow page's prefill inputs
- **THEN** the step results equal the workflow page's byte for byte

#### Scenario: Search finds the job
- **WHEN** an agent searches "plan a drone mapping flight"
- **THEN** `workflow.mapping-flight` is first in the result's `workflows` list

#### Scenario: A failed step
- **WHEN** an agent runs `workflow.preflight-check` with a METAR that does not parse
- **THEN** the error is the decoder's own, its message starts "Step 1 (aviation.weather.metar-decode):", and the later steps are listed as waiting
