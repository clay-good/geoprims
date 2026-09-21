## Why

geoprims reads like documentation. The people it is for — drone operators, surveyors, pilots — reach for it in the field, and the front door should feel like an instrument: precise, fast, and a pleasure to pick up. Two things got in the way. The home page led with a paragraph and a search that opened a pop-up, so the question-answering engine underneath (95% right on 637 test questions, values filled in) was hidden behind an extra step. And every tool page ended in a block of developer material that most readers never needed.

This change makes the site feel like a field instrument and makes the search do what people expect: ask with your numbers, press Enter, and the right calculator opens already filled in.

Depends on: `redesign-minimal-shell`, `add-glanceable-and-field-ux`, `add-seo-and-discoverability`.

## What Changes

- **Light is the look.** A field-instrument palette tuned for sunlight: cool paper, ink text, a hi-vis accent for action and focus, a survey teal for data and linework. Night (ink) stays as a one-tap toggle for night operations.
- **Home page.** A live contour-terrain hero with a survey reticle and a GNSS-style readout (decoration only, still under reduced motion), a headline, and one big search. Typing lists matches as you go, with the top tool opened with the question's values when it has any; Enter goes straight there. Example questions fill the field. Below: an instrument panel of real answers from each tool's worked example, the topics as tiles with icons, then recent and pinned.
- **Header.** The mark and name on the left; a search button and the night toggle on the right. Sticky, so search is one tap from any tool.
- **Tool pages.** The tool, its worked example, a Report a problem button, and one "How we got this" panel (formula, worked example, sources, constants, terms, proof). The "For developers and agents" block leaves tool pages; that material lives on `/agents/`, in `llms.txt`, and in `geoprims_describe`. Related tools become a row of plates. The action row is Copy, Copy with reference, Share, and Download.
- **Mobile first.** Two readouts per row on a phone, a single scrolling row of example questions, full-width actions, generous targets.

## Capabilities

### Modified Capabilities

- `web/page-template`: the header, the home body, and the tool body.
- `web/visual-theme`: the field-instrument palette and component look.
- `contracts/page-chrome`: region 10 folds into "How we got this"; region 13 leaves tool pages.
- `discovery/agent-discovery`: developer material moves off tool pages.

## Non-goals

- No change to any result, any tool, or the core. The redesign is presentation and navigation only.
- No third-party assets: the terrain, icons, and fonts are drawn or served from this site.
