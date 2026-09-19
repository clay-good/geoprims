# Tasks

## 1. Theme: five modes to two

- [x] 1.1 Cut `sunlight`, `night`, and `high-contrast` token blocks and the `--dim` token from `global.css`, and fold the `prefers-color-scheme` fallback into a plain `paper` default → verify: `test/theme.test.mjs` passes with `MODES = ['paper', 'ink']`, and no rule mentions a removed mode
- [x] 1.2 Simplify the pre-paint script in `Base.astro` to read the saved choice and fall back to `paper`, mapping the removed modes (and the old `hud`/`daylight` aliases) to their replacements → verify: the pre-paint test asserts a dark-OS first visit renders `paper`, and that `night`, `sunlight`, `high-contrast`, `hud`, and `daylight` all map correctly
- [x] 1.3 Delete the `gp-dim` key and the brightness slider (`eraseLocalData` already clears every `gp-` key, so nothing to add there) → verify: `test/prefs.test.mjs` passes and no source file mentions `gp-dim`

## 2. Header and footer

- [x] 2.1 Rebuild the site header as title + description on the left and a light/dark toggle button on the right, not sticky, hairline rule below → verify: `test/template.test.mjs` asserts the header contains exactly those elements on every built page
- [x] 2.2 Move Units, All tools, Agents, Settings, and Source into the footer, and remove the header search button → verify: the same test asserts no link or search control in the header, and the footer carries all five
- [x] 2.3 Wire the toggle: switch mode, persist, and relabel itself with the mode it now offers → verify: a unit test drives the toggle through both modes and checks the label, the accessible name, and the stored value

## 3. Home page

- [x] 3.1 Replace the hero with the centered description (three sentences, naming the catalog's domains), one search field, then the topic categories with counts and a link to `/tools/` → verify: `test/build.test.mjs` asserts the order of the home page's first three sections
- [x] 3.2 Move recent and pinned below the categories and keep them hidden until the browser has any → verify: existing lists test still passes, with the sections after the categories
- [x] 3.3 Point the claims gate at the new description → verify: `test/claims.test.mjs` fails if the description names a domain the catalog does not have

## 4. Catalog page

- [x] 4.1 Add `/tools/`: filter field, every operation grouped by category and group, counts, `ItemList` JSON-LD → verify: a new test asserts every catalog operation appears exactly once and the JSON-LD parses
- [x] 4.2 Add it to the sitemap, the footer, and the home category list → verify: `test/build.test.mjs` finds `/tools/` in the sitemap and both link lists

## 5. Look

- [x] 5.1 Apply the distinguishing rules: hairline-separated rows for tool lists, radii at most 8 px, tabular figures for values, graticule motif behind the home masthead and as the catalog's section rule → verify: the style lint fails on a radius above 8 px, and the theme test still finds no color literal outside the token blocks
- [x] 5.2 Check both modes at 320 px and desktop widths, and keep the print rules → verify: no horizontal scroll at 320 px, and the print block still forces the paper palette and drops the graticule

## 6. Docs and specs

- [x] 6.1 Update `README.md` (theme modes, header, the new page) and `docs/` references to the removed modes → verify: `rg 'sunlight|high-contrast|--dim'` finds nothing outside this change's own spec deltas and the changelog
- [x] 6.2 Record the change in `data/changelog.json` as `changed`, naming the removed modes → verify: `test/trust-pages.test.mjs` passes
