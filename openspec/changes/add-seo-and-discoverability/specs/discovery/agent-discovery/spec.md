## Purpose

Helps AI agents and developers who arrive at geoprims.com find the local MCP server, the tool catalog, and each tool's inputs, without scraping the human pages.

## ADDED Requirements

### Requirement: llms.txt
The site SHALL publish `/llms.txt` in the llmstxt.org format:
- a one-paragraph description of geoprims
- the promise that everything is computed locally with no tracking
- links to the MCP setup page, `/catalog/v1.json`, `/methodology`, `/sources`, and each domain hub

Counts in it SHALL come from the build catalog.

#### Scenario: Counts from build
- **WHEN** the catalog gains a stable tool
- **THEN** the next build's `llms.txt` count reflects it with no manual edit

### Requirement: MCP discovery document
The site SHALL publish `/.well-known/mcp.json` describing the local server: name, version, transport `stdio`, install options (GitHub clone command, `npx` command, MCPB download with SHA-256), the tool and resource names, homepage, and source repository. The build SHALL generate it from the MCP surface golden file.

#### Scenario: Surface parity
- **WHEN** the MCP server's tool list changes
- **THEN** `/.well-known/mcp.json` matches the golden surface file, or the build fails

### Requirement: Site AGENTS.md
The site SHALL publish `/AGENTS.md` telling coding agents:
- how to run the MCP server from a clone
- the tool id scheme
- that results carry `meta` caveats to relay to users
- how to prepare a problem report
- that the site must not be scraped for computation, because the MCP server exists

#### Scenario: AGENTS.md present
- **WHEN** an agent fetches `/AGENTS.md`
- **THEN** it receives the run instructions and the caveat-relay guidance

### Requirement: Field names for developers and agents
Superseded for tool pages by `redesign-field-instrument`, which keeps tool pages to the tool and its proof. Each tool's id, each input's field name with unit and range, and an example `geoprims_run` call that reproduces the worked example SHALL be available to developers and agents through `geoprims_describe` and `/llms.txt`, and SHALL NOT be printed on tool pages.

#### Scenario: Same answer on both surfaces
- **WHEN** an agent runs a tool's worked example through the MCP server
- **THEN** it returns exactly the sentence the tool page shows for that example
