## MODIFIED Requirements

### Requirement: Result panel
The result panel is the answer card defined in `ux/glanceable-results`; together with its expandable details it SHALL show every output with its unit, a unit switcher, a copy action for the value and for the value with its reference, a share action, downloads of the whole result (including JSON), the provenance of this result (`meta`: tool and core versions, model, accuracy, reference data by name, and citations) in an expandable section, and any warnings prominently above the values. The copy-as-agent-call action is removed from the tool page; the call an agent makes, and how to set one up, live on `/agents/`.

#### Scenario: Provenance of this result
- **WHEN** a user opens "Provenance" under the geoid height answer
- **THEN** it names the tool and core versions, the EGM96 grid by title and version, the model, the accuracy, and the sources cited

#### Scenario: Agent calls live with agents
- **WHEN** a developer wants the call that reproduces a tool's answer
- **THEN** `/agents/` shows how to connect the MCP server and call the tool, and the tool page stays free of developer controls
