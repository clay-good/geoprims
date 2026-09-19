# Design: the minimal shell

## Decision 1: two display modes, `paper` default

**Chosen**: `paper` (warm off-white, ink text) and `ink` (blue-graphite). A first-time visitor always starts in `paper`, whatever their OS prefers; the header toggle switches, and the choice is remembered in that browser. Without JavaScript the page renders `paper`.

**Alternatives considered**:
- *Keep five modes, toggle switches the two common ones.* Rejected: the three specialist modes cost a contrast audit, a luminance gate, a brightness slider, and 60 lines of tokens each, for modes that a settings page would hide anyway.
- *Follow the OS preference for the default.* Rejected on the same ground the accent color is fixed: on a dark-mode machine the site would open looking like every other dark developer tool, including roughlogic. The signature light surface is part of what makes geoprims recognizable, and one click changes it.
- *Dark default.* Rejected: closest of all options to roughlogic, and the paper surface reads better for the long printed-math pages that make up most of the site.

**Consequence**: `night`'s luminance gate and the `--dim` slider are deleted, not disabled. `test/theme.test.mjs` shrinks to the two modes and keeps the contrast and pre-paint assertions. The OS `prefers-color-scheme` no longer selects a mode, so the pre-paint script gets simpler: read the saved choice, else `paper`.

## Decision 2: header carries identity and one control

**Chosen**: title and description left, theme toggle right, hairline rule below, not sticky.

The header answers "what is this?" for someone arriving from a search result, and offers exactly one control. Search moves into the page body: on the home page it is the centered primary action, and on every other page the palette is still one keystroke (`/`) or one tap on the page's own filter field. The four utility links move to the footer, where Units, Agents, Settings, and Source already sit beside the disclaimer and version line.

**Alternatives considered**:
- *Keep the search in the header.* Rejected: it forced a three-part header that wrapped on phones, and duplicated the page-body search on the one page that matters most.
- *Sticky header.* Rejected: it costs vertical space on phones and a `backdrop-filter`, and the pages are short enough to scroll back.

**Consequence**: `ux/mobile-and-field` "Sticky answer above the keyboard" is unaffected (that is the tool page's answer bar, not the header). The palette keyboard model is unchanged; only its button moves.

## Decision 3: `/tools/` holds the whole catalog

**Chosen**: one page, grouped by category then group, every operation linked, with the same filter field the topic pages use. The home page links to it from the category list, and the footer links to it.

**Alternatives considered**:
- *List every tool on the home page.* Rejected: ~195 rows pushes the description and search off the first screen and makes the most-visited page the heaviest.
- *Rely on the topic pages alone.* Rejected: there is no single URL to answer "what does this site have?", which is also the page an agent or a crawler wants.

**Consequence**: `/tools/` is a second crawlable index alongside the topic pages. It carries an `ItemList` JSON-LD listing, and the sitemap includes it.

## Decision 4: what makes it geoprims and not roughlogic

Both sites are one quiet column of text on a plain surface, and that shared restraint is the point. The differences are deliberate and cheap to hold:

| | roughlogic.com | geoprims.com |
|---|---|---|
| Default surface | near-black `#0a0a0a` | warm paper `#f7f6f3` |
| Accent | blue `#5aa9ff` | signal orange `#c2410c` |
| Header | sticky, blurred, wordmark + tagline | static, hairline rule, title + description + theme toggle |
| Search | full pill, 999 px radius | square field, 6 px radius, hairline border |
| Lists | rounded cards, arrow on hover | hairline-separated rows, no card fill |
| Numbers | body figures | Geist Mono tabular figures, answer largest on the page |
| Motif | none | graticule hairlines in the home masthead and section rules |
| The thing itself | no canvas | live 2D map, 3D globe, and vector diagrams on tool pages |

The graticule motif is one repeating linear-gradient of `--graticule` hairlines at the spacing of a 15° grid, used only behind the home masthead and as the section rule on `/tools/`. It is decoration with no meaning: it never moves, it is `aria-hidden`, and it is dropped in print.

## Decision 5: what the home page says

The description is one sentence of what the site does, one of how it runs. It names the domains, because that is what a visitor is searching for:

> Exact, cited calculators for geodesy, navigation, aviation, drones, surveying, time, and units. Every answer shows its formula, its sources, and how far it can be trusted. Everything runs on your device: no ads, no accounts, no tracking, and no network after your first visit.

**Consequence**: the `claims.test.mjs` gate checks public claims against the build, so this wording is checked like any other claim: the domain list must match the catalog's domains, and the privacy sentence must match the fetch policy.
