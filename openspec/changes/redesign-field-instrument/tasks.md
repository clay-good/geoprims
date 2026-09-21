## 1. Look and feel

- [x] 1.1 Replace the tokens with the field-instrument palette in both modes, keeping WCAG AA and the token-only rule (`apps/web/test/theme.test.mjs`)
- [x] 1.2 Restyle cards, inputs, buttons, the answer readout, the safety strip, and lists as plates
- [x] 1.3 Draw the home terrain hero (seeded contours, reticle, readout; still under reduced motion; paused offscreen)

## 2. Search that answers

- [x] 2.1 Share one resolver between the home search and the palette (`apps/web/src/lib/ask.js`)
- [x] 2.2 Make the home search a combobox that opens the best match, with the question's values, on Enter; keep it a working form without JavaScript
- [x] 2.3 Put a search button in the sticky header on every page

## 3. Simpler tool pages

- [x] 3.1 Remove the developer block; fold Terms into "How we got this"; related tools as plates; trim the action row
- [x] 3.2 Round table cells to their column's declared precision, gated cell for cell (`apps/web/test/tables.test.mjs`)

## 4. Mobile

- [x] 4.1 Two readouts per row, a scrolling row of example questions, full-width actions below 40 rem
- [x] 4.2 Sweep every page type at 320, 375, and 768 px in a real browser and fix what breaks (home, catalog, topic, group, five tool pages, methodology, sources, settings, agents, verification, units, and not-found, measured in Chromium: no page scrolls sideways. The sweep found two real faults, both fixed: the sources page was 11 px too wide at 320 px because long identifiers could not wrap, and a list input's worked example printed as raw JSON)
