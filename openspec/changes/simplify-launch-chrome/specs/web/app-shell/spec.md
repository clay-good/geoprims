## ADDED Requirements

### Requirement: Security page
The site SHALL publish `/security/`, which says how to report a vulnerability privately, what is in and out of scope, and what the design already rules out, matching `SECURITY.md`.

#### Scenario: Private reporting
- **WHEN** a reader opens `/security/`
- **THEN** it links the repository's private advisory form and asks that vulnerabilities not be filed as public issues

## REMOVED Requirements

### Requirement: Settings
**Reason**: The unit choice already lives on each tool, and the other preferences did not earn a page. The site keeps nothing a browser's own "clear site data" does not remove.
**Migration**: Stored preferences keep applying. `/settings/` is not linked anywhere and is not built.
