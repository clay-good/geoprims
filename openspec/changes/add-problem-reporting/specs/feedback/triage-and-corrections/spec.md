## Purpose

Turns user reports into verified fixes quickly and visibly, so every confirmed error leaves the product permanently more correct and users can see what was wrong and when it was fixed.

## ADDED Requirements

### Requirement: Triage targets
Maintainers SHALL triage every new report (set status to `triaged`, `duplicate`, `not_a_bug`, or `wont_fix`) within 72 hours. Reports of kind `wrong-result` SHALL be triaged within 24 hours. A confirmed wrong result in an aviation, drone, navigation, or height/datum tool SHALL get a known-issues entry within 24 hours of confirmation, even before the fix ships.

#### Scenario: Wrong-result priority
- **WHEN** the triage query lists open reports
- **THEN** `wrong-result` reports appear first, ordered by age, with a flag on any older than 24 hours

### Requirement: Reproduction from the report alone
Every stored report SHALL carry enough context to reproduce the result exactly: the permalink restores the inputs, and the tool, core, and asset versions identify the build. A reproduction script SHALL take a report id and output the recomputed result on the reported build and on the current build.

#### Scenario: Reproduce
- **WHEN** a maintainer runs the reproduction script on a report id
- **THEN** it prints the reported outputs, the recomputation on the same build, and the recomputation on the current build, with differences highlighted

### Requirement: Verification against a primary source before any fix
A report SHALL be marked `confirmed` only after a maintainer checks the claimed correct value against a primary source (standard, reference implementation, or published worked example) and records the source. Reports claiming an error that the primary source contradicts SHALL be closed `not_a_bug`. If the confusion was reasonable, the tool's docs or result sentence SHALL be improved.

#### Scenario: Not a bug, but confusing
- **WHEN** a user reports that density altitude "should be" the rule-of-thumb value
- **THEN** the report is closed `not_a_bug`, and the tool's comparison line is reviewed for clarity

### Requirement: Every confirmed defect adds a regression vector
A fix for a confirmed defect SHALL add at least one golden vector reproducing the reported inputs, with the primary source recorded (per `platform/verification`). It SHALL bump the tool version. If the fix changes numeric output beyond tolerance, it SHALL carry a "result change" changelog label that states the old and new values for the reported case.

#### Scenario: Result-change label
- **WHEN** a fix changes a tool's output beyond its tolerance
- **THEN** the changelog entry is labeled "Result change", shows before/after values, and the tool page shows "Updated <date>: results changed. See changelog" for 90 days

### Requirement: Public known-issues page
The site SHALL publish `/known-issues`, generated at build time from a curated `known-issues.json` in the repository (not read live from D1). Each entry SHALL list the tool, a plain-language description, affected versions, the workaround, status (`investigating`, `confirmed`, `fixed`), and the fix version and date. A tool with an open confirmed issue SHALL show a banner on its page linking to the entry.

#### Scenario: Banner on affected tool
- **WHEN** a confirmed issue is open for `aviation.altimetry.cold-temp-correction`
- **THEN** that tool page shows a banner "Known issue: <summary>. Workaround: …" linking to `/known-issues`

### Requirement: GitHub mirroring only after approval
Maintainers MAY mirror a confirmed report to a GitHub issue using a fine-grained token scoped to issue creation on the geoprims repository. Only structured fields SHALL be mirrored: tool, versions, inputs, outputs, and the maintainer's summary. The reporter's free-text note SHALL NOT be mirrored unless the maintainer rewrites it. Mirroring SHALL never be automatic.

#### Scenario: Structured mirror
- **WHEN** a maintainer mirrors a report
- **THEN** the GitHub issue contains the permalink, versions, inputs, outputs, and maintainer summary, and no reporter free text

### Requirement: GitHub "Wrong answer" issue form
The repository SHALL provide an issue form labeled `correctness` with required fields: tool or URL, inputs, answer received, answer expected, and "the published source that settles it". The report dialog's paused state and the site footer SHALL link to it.

#### Scenario: Required source field
- **WHEN** someone opens a "Wrong answer" issue without the source field
- **THEN** GitHub's form validation blocks submission

### Requirement: Agents can prepare, not send, reports
The MCP server SHALL provide `geoprims_report_problem` with input `{toolId, args, observed, expected?, source?, note?}`. It SHALL return the payload a human would send, a `https://geoprims.com/<route>#…&report=1` link that opens the tool with the inputs restored and the report dialog pre-filled, and the GitHub issue-form link. The server SHALL make no network request, and the tool description SHALL tell the agent to show the link to the user.

#### Scenario: Agent-prepared report
- **WHEN** an agent calls `geoprims_report_problem` after a suspicious result
- **THEN** it receives a link and payload, and no data leaves the machine until the user opens the link and sends the report

### Requirement: Feedback metrics without tracking
The project SHALL publish a monthly correctness summary on the site, derived only from report statuses and the changelog: reports received, confirmed defects, median time to fix, and result changes shipped. The summary SHALL include no per-user data.

#### Scenario: Monthly summary
- **WHEN** a month closes
- **THEN** `/quality` shows that month's counts and median time-to-fix
