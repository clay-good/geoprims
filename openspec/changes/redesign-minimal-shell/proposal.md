## Why

A visitor landing on geoprims today reads chrome before they read the product. The header carries a brand, a search button, and four links that wrap to two lines on a phone. The display control is a five-option select at the very bottom of every page. The home page stacks a hero, example chips, pinned, recent, popular, topics, and a status line before a first-time visitor learns what the site is for. And there is no single page that lists every calculator, so "show me everything you have" has no answer.

This change makes the front door calm and obvious: the header says what this is and offers light or dark, one centered paragraph explains the product, one search field is the way in, and one page lists every tool by category. The map, globe, and diagram canvas stay exactly where they earn their keep, on the tool pages.

Depends on: `build-web-experience`, `add-glanceable-and-field-ux`.

## What Changes

- **Header**: the site title and a one-line description on the left, a light/dark toggle on the right. Nothing else. The utility links (Units, Agents, Settings, Source) move to the footer, and the search moves into the page body where it is the primary action.
- **Two display modes, not five**: `paper` (light) and `ink` (dark). `sunlight`, `night`, and `high-contrast` are removed, along with the brightness slider. A first-time visitor gets `paper` whatever their OS prefers; the toggle is the one control, and the choice persists in that browser.
- **Home page**: a centered description of the product, then one search field, then the topic categories with their tool counts and a link to the full catalog. Recent and pinned tools stay, quietly, below the categories.
- **A catalog page at `/tools/`**: every operation on one crawlable page, grouped by category and group, with a filter field.
- **A visual identity of its own**: the minimal, single-column restraint of roughlogic.com, but paper-light by default with a signal-orange accent, hairline rules instead of heavy cards, tabular figures, and a graticule motif. geoprims keeps what roughlogic cannot have: a live 2D map, 3D globe, and vector diagrams on tool pages.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web/visual-theme`: two modes instead of five, a fixed `paper` default, and the distinguishing visual rules (geometry, rules, figures, motif).
- `web/page-template`: the header and footer anatomy, and the home and catalog page bodies.
- `web/app-shell`: the `/tools/` catalog page, and where the display control lives.
- `ux/mobile-and-field`: the sunlight and night requirements are removed; outdoor legibility is carried by `paper`'s contrast and the existing target sizes.

## Non-goals

- No change to tool pages below the page header: the values-and-answer layout, canvas, "How we got this", and citations stay as specified.
- No change to the palette's behavior or keyboard model; only where its entry point sits.
- No new color beyond the existing signal orange, and no imagery, gradients, or illustration in the chrome.
- No marketing home page: no testimonials, feature grids, or calls to sign up. There is nothing to sign up for.

## Trade-offs

Dropping `sunlight`, `night`, and `high-contrast` removes two field affordances the earlier specs promised (outdoor maximum contrast, dark-adapted cockpit use) in exchange for one obvious control and far less surface to keep accessible. `paper` already meets WCAG AA at 4.5:1 and stays legible outdoors; `ink` covers dark-adapted use imperfectly, and this change records that as a deliberate loss rather than a silent one. Users who need more contrast than `paper` offers are served by their OS and browser contrast settings, which the site no longer overrides.
