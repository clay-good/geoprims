# Tasks

- [x] 1 Footer as one centered row of the nine links → verify: `apps/web/test/template.test.mjs` asserts all nine on every page, and `trust-pages.test.mjs` asserts every page links the disclaimer, privacy, accuracy, and security pages
- [x] 2 `/security/` from `SECURITY.md` → verify: the route is classed `trust` in `scripts/routes.mjs`, and the footer test reaches it
- [x] 3 Remove `/settings/`, its palette entries, and the erase and clear-recent helpers → verify: no source or test mentions `/settings/`, and `routes.test.mjs` uses `/offline/` for the app class
- [x] 4 Remove Recent from the home page → verify: `build.test.mjs` asserts the home page has no recent list and pinned still comes last
