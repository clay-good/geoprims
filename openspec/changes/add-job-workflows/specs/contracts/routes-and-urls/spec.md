## ADDED Requirements

### Requirement: Workflow routes
Workflows SHALL live at `/workflows/<slug>/` (indexable, route class `learn`), and `/workflows/` SHALL list them by audience. Each `/journeys/<slug>/` SHALL redirect with 301 to its workflow, recorded in `data/redirects.json`.

#### Scenario: Old journey link
- **WHEN** a reader follows a saved link to `/journeys/drone-mapping-day/`
- **THEN** they land on `/workflows/mapping-flight/` with a 301
