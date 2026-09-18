## Why

geoprims must be something you can glance at and know what to do: a pilot at the fuel pump, a surveyor in the sun with gloves on, a drone operator on a phone, a developer in a hurry. The original specs define what each tool computes and how the HUD looks. They do not yet define the fixed, human-readable anatomy that makes 800 tools feel like one simple product.

That anatomy needs:
- the answer first, in a plain sentence
- example values already filled in
- only the inputs that matter visible
- every result framed against something the reader already knows

Mobile and field use needs its own contract: numeric keyboards that work, large targets, readable in sunlight, safe at night in a cockpit, and usable offline.

Research: `docs/research/05` §3–4 (mobile inputs, GOV.UK findings on `type=number`, iOS zoom, glove targets, FAA night-vision guidance, ForeFlight night mode, plain-language results, show-your-work). roughlogic.com's mobile gates are in `docs/research/06` §3: 320 px sweep of every page, 200% text zoom, 48 px targets, answer above inputs on phones.

Depends on: `build-web-experience`, `add-trust-and-proof`.

## What Changes

- **A fixed tool-page anatomy.** Title and one-line purpose, then the answer card, then core inputs, then "More options", then "How we got this", then related tools and report.
- **Answer card:**
  - the big number with its unit
  - a plain-language sentence
  - a comparison to something familiar (field elevation, rule of thumb, typical range)
  - a status phrase when a threshold exists (within, near, or beyond a cited limit, with text and icon, never "SAFE")
  - copy buttons
- **Prefilled worked example on first load.** Visible "Example values" labeling and a one-tap "Clear".
- **Progressive disclosure.** At most 5 core inputs visible. Advanced inputs sit under "More options", with their defaults shown in words ("Using ISA temperature. Change").
- **Plain-language vocabulary.** Every abbreviation is expanded on first use with a tap-to-define glossary, and every field has a help line.
- **A home page built for glancing.** One search box that understands questions, hero tools as large cards, "Start a journey" rows, and recent tools.
- **Mobile input contract:**
  - `type=text` with `inputmode=decimal`, and a ± key for signed values
  - both decimal separators accepted
  - inputs at 16 px or larger (no iOS zoom); targets at least 48 px
  - `enterkeyhint`
  - a sticky answer bar that stays above the keyboard
  - side-by-side layout on landscape tablets (EFB use) and split-screen widths of 320–500 px
- **Field conditions:** `sunlight` and `night` modes, an optional larger-target "Field mode", offline status, and "Save this domain for offline".
- **Usability proof:** 5-second glance tests and task tests with real practitioners before launch, recorded.

## Capabilities

### New Capabilities

- `ux/glanceable-results`: Tool-page anatomy, answer card, result sentences, defaults, disclosure, vocabulary, and home page.
- `ux/mobile-and-field`: Input behavior, layout at small and split widths, field and cockpit conditions, offline affordances, and mobile quality gates.

### Modified Capabilities

None. `web/visual-theme` (unarchived) was updated in place with the five modes and 48 px targets. `web/app-shell` forms follow the mobile input contract here.

## Non-goals

- A native app. The PWA is the mobile product.
- Voice input in v1.
- Gamification, streaks, or engagement features.

## Impact

- Result sentence templates join each tool's manifest (`x-sentence`), shared with MCP results as `summary`.
- A glossary data file with definitions and citations.
- New CI gates: mobile sweep, touch targets, input attributes, answer-above-the-fold, and sentence-template coverage.
