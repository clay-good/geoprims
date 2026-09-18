## Purpose

Makes geoprims work well on phones and tablets in real conditions: bright sun, gloves, a dark cockpit, split-screen beside an EFB, and no signal.

## ADDED Requirements

### Requirement: Numeric input contract
Numeric inputs SHALL use `type="text"` with `inputmode="decimal"`, `autocomplete="off"`, `autocorrect="off"`, `spellcheck="false"`, and an `enterkeyhint` of `next`, or `done` on the last field. They SHALL NOT use `type="number"`. Signed quantities (longitude, temperature, offsets) SHALL provide a ± toggle, because some mobile decimal keypads lack a minus key. The decimal separator SHALL follow the number-format setting (US default: decimal point; decimal comma selectable), per `platform/units-and-quantities`: in decimal-point mode `1,250` means 1250 and `1,25` is rejected with a hint to switch modes. Input font size SHALL be at least 16 px, so iOS does not zoom on focus, and pinch-zoom SHALL never be disabled.

#### Scenario: Negative temperature on iOS
- **WHEN** a user on an iPhone enters −12 °C
- **THEN** they can do so with the decimal keypad plus the ± toggle

#### Scenario: Input attribute gate
- **WHEN** the input-contract gate scans rendered forms
- **THEN** no numeric field uses `type="number"`, and every numeric field has `inputmode="decimal"` and a font size of at least 16 px

### Requirement: Targets and spacing
All interactive targets SHALL be at least 48 × 48 CSS px with 8 px spacing. An optional "Field mode" setting SHALL raise targets to 56 px, raise result text by 25%, and turn on per-field step buttons using each field's `x-step` (small and large steps, e.g. 0.01 and 0.10 inHg, 1 and 10 kt), for gloved or moving-vehicle use.

#### Scenario: Touch target measurement
- **WHEN** the touch-target test measures every control at 390 px width
- **THEN** none is smaller than 48 × 48 CSS px

### Requirement: Sticky answer above the keyboard
On phones, the answer card SHALL collapse to a sticky bar that remains visible above the on-screen keyboard (tracking the visual viewport) while any input is focused, and SHALL update live.

#### Scenario: Keyboard open
- **WHEN** a user edits the OAT field with the keyboard open
- **THEN** the updated answer is visible above the keyboard

### Requirement: Layouts for every width
Pages SHALL reflow without horizontal page scroll at every width from 320 px up, including 200% text zoom. At widths of 900 px or more, and on landscape tablets, inputs and the answer SHALL sit side by side so a typical tool needs no scrolling. Split-screen widths of 320–500 px (a tablet beside an EFB) SHALL use the phone layout without clipping.

#### Scenario: Landscape iPad
- **WHEN** the crosswind tool opens on a landscape tablet at 1180 × 820
- **THEN** inputs, answer card, and runway diagram are all visible without scrolling

### Requirement: Mobile quality gates
CI SHALL check:
- **Every indexable page and every tool route at 320 × 720:** no horizontal overflow.
- **A sample of pages (every hub, the home page, and at least 30 tools spread across domains):** no overflow at 568 × 320 landscape and at 375 px with 200% text zoom.
- **Engines:** both Chromium and WebKit.
- **Status check:** each check first asserts that the page returned 2xx, so an error page cannot pass.

#### Scenario: Overflow regression
- **WHEN** a new tool's table overflows at 320 px
- **THEN** CI fails naming the route and the overflowing element

### Requirement: Sunlight and night use
The `sunlight` and `night` modes (per `web/visual-theme`) SHALL be reachable in one tap from every page's header. `night` mode:
- shall render the canvas in the same low-luminance palette
- shall invert white map backgrounds
- shall never show a bright splash, and shall disable HUD glow
- shall carry every state with text or icon, because red light distorts color

#### Scenario: One-tap night
- **WHEN** a pilot taps the moon icon in the header
- **THEN** the page switches to night mode immediately, with no bright frame

### Requirement: Offline affordances in the field
Each page SHALL show a small status: "Works offline" when the tool and its data are cached, or "Needs data: <pack>" when it is not. Each domain hub SHALL offer "Save this domain for offline", which downloads its modules, docs, and default data with the size shown first.

#### Scenario: Save domain
- **WHEN** a surveyor taps "Save survey tools for offline" on the survey hub
- **THEN** the size is shown, the pack downloads, and every survey tool shows "Works offline"

### Requirement: Printing and sharing from mobile
Tools SHALL offer native share (Web Share API, when available) for the permalink, and the sentence with reference. Printing from mobile SHALL produce a one-page calculation sheet for single-result tools.

#### Scenario: Share sentence
- **WHEN** a user taps Share on a phone
- **THEN** the system share sheet offers the sentence, reference, and permalink
