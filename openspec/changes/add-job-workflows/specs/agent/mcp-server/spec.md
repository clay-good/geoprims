## ADDED Requirements

### Requirement: Workflows through the meta-tools
The MCP server SHALL expose each workflow as a catalog entry with id `workflow.<slug>`. `geoprims_search` SHALL find it, `geoprims_describe` SHALL return its inputs, steps, and assumptions, and `geoprims_run` SHALL run it through the shared chain runner, returning the summary and each step's tool id, input, and result. No new top-level MCP tools SHALL be added.

#### Scenario: Agent plans a mapping flight
- **WHEN** an agent calls `geoprims_run` with `workflow.mapping-flight` and the workflow page's prefill inputs
- **THEN** the step results equal the workflow page's byte for byte

#### Scenario: Search finds the job
- **WHEN** an agent searches "plan a drone mapping mission"
- **THEN** `workflow.mapping-flight` ranks first
