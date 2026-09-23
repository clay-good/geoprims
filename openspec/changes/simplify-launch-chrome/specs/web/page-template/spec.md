## MODIFIED Requirements

### Requirement: One page anatomy
Every page SHALL have, in order: the site header, a page header built with the `PageHeader` component (breadcrumbs when the page has a parent, one `h1` in sentence case, an optional status badge, and one purpose line in plain words), the page body, and the site footer.

The site footer SHALL be one centered row of links: All tools, Units, Agents, Source, Privacy, Security, Accuracy, Disclaimer, and Licenses. It SHALL carry nothing else.

Page types and their bodies:

| Type | Examples | Body |
|---|---|---|
| Home | `/` | Centered product description, one search field, the topic categories with counts and a link to `/tools/`, then pinned tools when the browser has any |
| Catalog | `/tools/` | Filter field, then every operation grouped by category and group |
| Topic | `/aviation/` | Filter field, jump chips to each group, one card grid per group |
| Group | `/aviation/wind/` | Filter field, card grid |
| Tool | `/aviation/wind/runway-components/` | Safety note if operational, values and answer side by side (answer first on phones), diagram, map, "How we got this", "For developers and agents", related tools |
| Info | `/methodology/`, `/sources/`, `/security/` | Readable text column (at most 46 rem) |
| Not found | any unknown URL | What happened, the search field, and a few popular tools |

#### Scenario: Footer links
- **WHEN** any page is built
- **THEN** its footer links to all nine destinations and holds no other text or control
