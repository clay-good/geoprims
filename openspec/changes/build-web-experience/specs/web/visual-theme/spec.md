## Purpose

Defines the geoprims design system, "Atlas": a minimal, modern interface in the spirit of well-made product tools, with quiet paper-and-ink surfaces, one signal color, precise type, and maps that feel alive. It covers color, type, layout, and motion, and guarantees the design stays accessible, legible, and printable in every mode.

## ADDED Requirements

### Requirement: Design tokens
All colors, type sizes, spacing, radii, and motion durations SHALL be defined as design tokens and used by every component. Each mode SHALL use one accent color, the signal orange, tuned per mode for contrast; everything else is neutral. Surfaces SHALL be flat (no gradients, glows, or textures on UI chrome), separated by hairline borders and spacing rather than heavy shadows. Warnings and errors SHALL use a distinct treatment (icon and text plus color) that does not depend on red/green discrimination. Badges and status words SHALL use sentence case ("Experimental", "Result out of date"), never all caps.

#### Scenario: No hard-coded colors
- **WHEN** the style lint scans component styles
- **THEN** it finds no color literal outside the token definitions

### Requirement: Theme modes
The app SHALL provide five modes:
- `paper` (light: warm off-white surfaces, ink text, signal-orange accent): the product's signature look
- `ink` (dark: deep blue-graphite surfaces, soft white text, the same signal orange lifted for contrast)
- `sunlight` (maximum contrast for outdoor field use: black on white, heavier weights, no effects, primary results at ≥ 40 px)
- `night` (for cockpit and dark-adapted use: amber or red on black at very low luminance, with an extra dimming slider, and meaning never carried by color)
- `high-contrast` (per `prefers-contrast: more`)

The initial mode SHALL follow OS preferences: light → `paper`, dark → `ink`, more contrast → `high-contrast`. It SHALL persist the user's explicit choice locally. Switching modes SHALL never flash a bright frame. No mode SHALL apply visual effects (glow, scanlines, blur, or noise) to text or result values. Clarity always wins over style. `night` mode text tokens SHALL have relative luminance between 0.175 and 0.25 (meeting 4.5:1 on its background), background tokens at most 0.005, and no navigation or transition frame SHALL render a background above 0.01.

#### Scenario: Light-preference user
- **WHEN** a first-time visitor's OS prefers a light scheme
- **THEN** the app starts in `paper` mode

#### Scenario: Night mode transition
- **WHEN** a pilot navigates between pages in `night` mode
- **THEN** no frame renders a background with relative luminance above 0.01 during navigation

#### Scenario: Print
- **WHEN** a user prints a tool page from any mode
- **THEN** the print stylesheet renders the `paper` palette with inputs, results, formula, references, and a canvas snapshot

### Requirement: WCAG 2.2 AA conformance
Every mode SHALL meet WCAG 2.2 Level AA (for `night` mode, measured at the dimming slider's maximum; lowering the slider is an explicit, disclosed user choice). Text SHALL have a contrast ratio of at least 4.5:1 (3:1 for large text) against its actual rendered background, including map fills beneath overlay text. Non-text UI and data marks that convey meaning SHALL have at least 3:1 contrast. Secondary "dim" text tokens SHALL still meet 4.5:1.

#### Scenario: Contrast audit
- **WHEN** the automated accessibility job renders every component state in every mode
- **THEN** no text falls below 4.5:1 and no meaningful mark falls below 3:1

### Requirement: Meaning never depends on color alone
Warnings, errors, stale results, and status (for example headwind vs tailwind, cut vs fill, pass vs fail) SHALL be conveyed with text, icon shape, or pattern in addition to color, and SHALL remain distinguishable under protanopia, deuteranopia, and tritanopia simulations.

#### Scenario: Cut and fill
- **WHEN** an earthwork cross-section shows cut and fill areas
- **THEN** cut and fill use different hatch patterns and labels as well as colors

### Requirement: Typography for numbers
Prose SHALL use one self-hosted neo-grotesque sans (Geist Sans, SIL OFL 1.1) and numeric values SHALL use its companion monospace (Geist Mono) or tabular figures, so digits align. Primary results SHALL be the largest type on the page, with units set smaller and muted beside the number. The type scale SHALL remain legible at 200% browser zoom and reflow at 320 CSS px width without horizontal scrolling (WCAG 1.4.10), except for the canvas and wide data tables.

#### Scenario: Reflow at 320 px
- **WHEN** a tool page is viewed at 320 CSS px width
- **THEN** the form, results, and docs reflow into one column with no horizontal page scroll

### Requirement: Focus and targets
Every interactive element SHALL have a visible focus indicator of at least 2 CSS px with 3:1 contrast, SHALL not be obscured by sticky bars when focused (WCAG 2.4.11), and SHALL have a target size of at least 48 × 48 CSS px with at least 8 px spacing (exceeding WCAG 2.5.8's 24 px minimum, for field and gloved use).

#### Scenario: Sticky header does not hide focus
- **WHEN** a keyboard user tabs to a field near the top under the sticky header
- **THEN** the page scrolls so the focused field is fully visible

### Requirement: Motion and flashing limits
UI motion SHALL be subtle and purposeful: at most 200 ms for UI transitions, and at most 800 ms with ease-out for canvas camera moves (per `web/map-canvas`). Motion SHALL respect `prefers-reduced-motion`, and no element SHALL flash more than 3 times in any one-second period.

#### Scenario: Reduced motion transitions
- **WHEN** reduced motion is requested
- **THEN** panel and palette transitions are instant

### Requirement: Self-hosted fonts and icons
Fonts and icons SHALL be self-hosted (no third-party font services), subset to the characters used, with `font-display: swap`, and total font payload SHALL be at most 80 KB compressed on first load.

#### Scenario: Font budget
- **WHEN** the performance job measures font transfer on a cold load
- **THEN** it is at most 80 KB compressed and all from the geoprims origin

### Requirement: Right-to-left and localization readiness
All UI strings SHALL be externalized, number and date formatting SHALL use the user's chosen number format, and layouts SHALL use logical CSS properties so a future right-to-left locale requires no layout rewrite. Launch locale is US English.

#### Scenario: String externalization
- **WHEN** the i18n lint scans components
- **THEN** no user-visible literal string exists outside the message catalog
