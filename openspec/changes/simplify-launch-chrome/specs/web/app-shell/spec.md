## ADDED Requirements

### Requirement: Security page
The site SHALL publish `/security/`, which says how to report a vulnerability privately, what is in and out of scope, and what the design already rules out, matching `SECURITY.md`.

#### Scenario: Private reporting
- **WHEN** a reader opens `/security/`
- **THEN** it links the repository's private advisory form and asks that vulnerabilities not be filed as public issues

### Requirement: Report a problem from every page
Every page SHALL offer "Report a problem". On a tool page it SHALL open that tool's report, with the inputs and results as today. On any other page it SHALL open a page report: the page's path without its fragment, the site's core version and build, the kind of problem, and an optional note, sent with `toolId` `site` and no inputs, outputs, warnings, or assets. The Worker SHALL accept a page report for any site path and SHALL refuse one that carries inputs, outputs, warnings, or assets. No page and no MCP result SHALL send a reader to a GitHub issue to report a problem; when reporting is paused or the reader is offline, the dialog SHALL offer to copy the report instead.

#### Scenario: A report from the privacy page
- **WHEN** a reader on `/privacy/` presses "Report a problem" and sends a note
- **THEN** the Worker stores a report with `tool_id` `site` and `page_path` `/privacy/`

#### Scenario: No GitHub detour
- **WHEN** the report dialog opens with reporting paused
- **THEN** it offers to copy the report, and links nowhere else

## REMOVED Requirements

### Requirement: Settings
**Reason**: The unit choice already lives on each tool, and the other preferences did not earn a page. The site keeps nothing a browser's own "clear site data" does not remove.
**Migration**: Stored preferences keep applying. `/settings/` is not linked anywhere and is not built.
