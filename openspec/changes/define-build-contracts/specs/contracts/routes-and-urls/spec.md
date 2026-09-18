## Purpose

Fixes every URL the site serves, which pages search engines see, and the exact permalink format, so the web app, the MCP server, and the build all produce and accept the same addresses.

## ADDED Requirements

### Requirement: Route map
The site SHALL serve exactly these route classes:

| Class | Route | Indexable | Sitemap |
|---|---|---|---|
| Home | `/` | yes | `pages` |
| Domain hub | `/<domain>/` | yes | `<domain>` |
| Group hub | `/<domain>/<group>/` | yes | `<domain>` |
| Tool (operation) | `/<domain>/<group>/<operation>/` | yes if stable, or experimental with full content | `<domain>` |
| Generated endpoint, high-intent | `/<domain>/<group>/<pair>/` | yes | `<domain>` |
| Generated endpoint, other | `/<domain>/<group>/<pair>/` | no (canonical to parent) | none |
| Alias slug | `/<domain>/<alias>/` | no (canonical to parent) | none |
| Explainer | `/learn/<slug>/` | yes | `learn` |
| Journey | `/journeys/<slug>/` | yes | `learn` |
| Trust pages | `/sources/`, `/methodology/`, `/verification/<version>/`, `/changelog/`, `/known-issues/`, `/quality/`, `/licenses/`, `/privacy/` | yes | `pages` |
| Agents | `/agents/` (setup), `/llms.txt`, `/AGENTS.md`, `/.well-known/mcp.json`, `/catalog/v1.json` | `/agents/` only | `pages` |
| App | `/settings/`, `/offline/` | no | none |
| API | `/api/reports`, `/api/reports/config` | no | none |

Every non-indexable HTML route SHALL carry `rel="canonical"` to its parent (or `noindex` for app routes). The build SHALL fail on any route outside this map.

#### Scenario: Route outside the map
- **WHEN** a build step emits `/tools/density-altitude/`
- **THEN** the route-map gate fails naming the path

#### Scenario: Alias canonical
- **WHEN** `/aviation/crosswind-calculator/` is built
- **THEN** it renders the runway-components tool with a canonical URL of `/aviation/wind/runway-components/`

### Requirement: Stable URL rules
URLs SHALL be lowercase, hyphenated, and end with a trailing slash. Tool routes SHALL be derived mechanically from tool ids (`aviation.altimetry.density-altitude` → `/aviation/altimetry/density-altitude/`). A renamed or deprecated tool SHALL keep a redirect from its old route for at least 24 months.

#### Scenario: Deprecated tool redirect
- **WHEN** a tool id is deprecated in favor of another
- **THEN** its old route redirects to the replacement's route, and the redirect is listed in the redirects file

### Requirement: Permalink fragment grammar
Fragments SHALL follow this grammar (ABNF):

```
fragment = example-frag / state-frag
example-frag = "example" [ flags ]
state-frag = "v1:" payload [ flags ]
payload = 1*( ALPHA / DIGIT / "-" / "_" )   ; base64url, no padding
flags = *( ";" flag )
flag = "report" / "nofx"
```

`payload` is `base64url(deflate-raw(canonical-json))`. The JSON object has exactly these keys: `i` (inputs, keyed by manifest field name, values as unit-tagged strings, or for list fields an array of rows of them), `u` (unit overrides), `v` (canvas view), and `e` (pinned epoch for `x-clock-default: allowed` tools). Keys are sorted, and numbers are serialized by the core serializer. `report` opens the report dialog after load. `nofx` disables HUD effects for the view. The core SHALL provide the encoder and decoder, used by both the web app and the MCP server (`geoprims_report_problem` links). A shared vector file SHALL pin byte-exact encodings.

#### Scenario: Shared encoding
- **WHEN** the web app and the MCP server encode the same inputs
- **THEN** both produce the identical fragment string, matching the shared vector file

#### Scenario: Unknown version
- **WHEN** a fragment starts with `v9:`
- **THEN** the page loads the tool with example values and shows "This link was made by a newer version of geoprims"
