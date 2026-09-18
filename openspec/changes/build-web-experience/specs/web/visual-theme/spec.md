## Purpose

Defines the geoprims tactical "retro-HUD" design system (color, type, layout, motion) and guarantees it stays accessible, legible, and printable in every mode.

## ADDED Requirements

### Requirement: Design tokens
All colors, type sizes, spacing, radii, and motion durations SHALL be defined as design tokens and used by every component. The default HUD palette SHALL be monochrome slate backgrounds with phosphor amber as the primary accent and phosphor green as the secondary accent, with a distinct non-red-green-dependent warning and error treatment.

#### Scenario: No hard-coded colors
- **WHEN** the style lint scans component styles
- **THEN** it finds no color literal outside the token definitions

### Requirement: Theme modes
The app SHALL provide at least four modes: `hud-amber` (default), `hud-green`, `high-contrast` (pure black and white plus one accent, no effects), and `paper` (light, print-optimized). The initial mode SHALL follow `prefers-color-scheme` and `prefers-contrast` (dark → `hud-amber`, light → `paper`, more contrast → `high-contrast`) until the user chooses explicitly.

#### Scenario: Light-preference user
- **WHEN** a first-time visitor's OS prefers a light scheme
- **THEN** the app starts in `paper` mode

#### Scenario: Print
- **WHEN** a user prints a tool page from any mode
- **THEN** the print stylesheet renders the `paper` palette with inputs, results, formula, references, and a canvas snapshot

### Requirement: WCAG 2.2 AA conformance
Every mode SHALL meet WCAG 2.2 Level AA. Text SHALL have a contrast ratio of at least 4.5:1 (3:1 for large text) against its actual rendered background, including glow and scanline effects. Non-text UI and data marks that convey meaning SHALL have at least 3:1 contrast. Secondary "dim" text tokens SHALL still meet 4.5:1.

#### Scenario: Contrast audit
- **WHEN** the automated accessibility job renders every component state in every mode
- **THEN** no text falls below 4.5:1 and no meaningful mark falls below 3:1

### Requirement: Meaning never depends on color alone
Warnings, errors, stale results, and status (for example headwind vs tailwind, cut vs fill, pass vs fail) SHALL be conveyed with text, icon shape, or pattern in addition to color, and SHALL remain distinguishable under protanopia, deuteranopia, and tritanopia simulations.

#### Scenario: Cut and fill
- **WHEN** an earthwork cross-section shows cut and fill areas
- **THEN** cut and fill use different hatch patterns and labels as well as colors

### Requirement: Typography for numbers
Numeric values SHALL be set in a monospace or tabular-figure typeface so digits align. The type scale SHALL remain legible at 200% browser zoom and reflow at 320 CSS px width without horizontal scrolling (WCAG 1.4.10), except for the canvas and wide data tables.

#### Scenario: Reflow at 320 px
- **WHEN** a tool page is viewed at 320 CSS px width
- **THEN** the form, results, and docs reflow into one column with no horizontal page scroll

### Requirement: Focus and targets
Every interactive element SHALL have a visible focus indicator of at least 2 CSS px with 3:1 contrast, SHALL not be obscured by sticky HUD bars when focused (WCAG 2.4.11), and SHALL have a target size of at least 24 × 24 CSS px (WCAG 2.5.8).

#### Scenario: Sticky header does not hide focus
- **WHEN** a keyboard user tabs to a field near the top under the sticky header
- **THEN** the page scrolls so the focused field is fully visible

### Requirement: Motion and flashing limits
UI motion SHALL be subtle and purposeful (at most 200 ms for transitions), SHALL respect `prefers-reduced-motion`, and no element SHALL flash more than 3 times in any one-second period.

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
