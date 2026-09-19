## MODIFIED Requirements

### Requirement: Theme modes
The app SHALL provide exactly two modes:
- `paper` (light: warm off-white surfaces, ink text, signal-orange accent): the product's signature look and the default
- `ink` (dark: deep blue-graphite surfaces, soft white text, the same signal orange lifted for contrast)

A visitor with no saved choice SHALL get `paper`, whatever the operating system prefers, and a page rendered without JavaScript SHALL be `paper`. The site SHALL NOT override the browser's own contrast or colour settings. A single control in the site header SHALL switch between the two modes, SHALL state which mode it switches to, and SHALL persist the choice in that browser only. Switching modes SHALL never flash a bright frame. No mode SHALL apply visual effects (glow, scanlines, blur, or noise) to text or result values. Clarity always wins over style.

#### Scenario: First visit on a dark-mode machine
- **WHEN** a first-time visitor whose OS prefers a dark scheme opens any page
- **THEN** the page renders in `paper`, and the header control offers to switch to dark

#### Scenario: Choice persists
- **WHEN** a visitor switches to `ink` and opens another page
- **THEN** that page renders in `ink` before first paint, with no bright frame

#### Scenario: No JavaScript
- **WHEN** a page is rendered with JavaScript disabled
- **THEN** it renders in `paper` with full contrast and no layout shift

#### Scenario: Print
- **WHEN** a user prints a tool page from either mode
- **THEN** the print stylesheet renders the `paper` palette with inputs, results, formula, references, and a canvas snapshot, and drops the graticule motif

### Requirement: WCAG 2.2 AA conformance
Both modes SHALL meet WCAG 2.2 Level AA. Text SHALL have a contrast ratio of at least 4.5:1 (3:1 for large text) against its actual rendered background, including map fills beneath overlay text. Non-text UI and data marks that convey meaning SHALL have at least 3:1 contrast. Secondary "dim" text tokens SHALL still meet 4.5:1.

#### Scenario: Contrast audit
- **WHEN** the automated accessibility job renders every component state in both modes
- **THEN** no text falls below 4.5:1 and no meaningful mark falls below 3:1

### Requirement: Design tokens
All colors, type sizes, spacing, radii, and motion durations SHALL be defined as design tokens and used by every component. Each mode SHALL use one accent color, the signal orange, tuned per mode for contrast; everything else is neutral. Surfaces SHALL be flat (no gradients, glows, or textures on UI chrome) and SHALL be separated by hairline rules and spacing, not by card fills or shadows. Corner radii SHALL be at most 8 px, so no control reads as a pill. Warnings and errors SHALL use a distinct treatment (icon and text plus color) that does not depend on red/green discrimination. Badges and status words SHALL use sentence case ("Experimental", "Result out of date"), never all caps.

#### Scenario: No hard-coded colors
- **WHEN** the style lint scans component styles
- **THEN** it finds no color literal outside the token definitions

#### Scenario: No pills
- **WHEN** the style lint scans component styles
- **THEN** no rule sets a border radius above 8 px

## ADDED Requirements

### Requirement: A look of its own
The design SHALL be minimal in the same spirit as its sister site roughlogic.com (one quiet column, plain surfaces, no illustration or marketing furniture) while staying visually distinct from it. The following SHALL hold, and each is the geoprims side of that distinction:
- the default surface is light paper, not near-black
- the accent is the signal orange, and no blue is used as an accent
- lists of tools are hairline-separated rows, not filled cards
- numeric values use the monospace companion face with tabular figures, and a tool's primary answer is the largest type on its page
- a graticule motif (evenly spaced hairlines in `--graticule`, at rest, `aria-hidden`, dropped in print) MAY appear behind the home masthead and as the catalog page's section rule, and nowhere else
- the 2D map, 3D globe, and vector diagrams remain on tool pages, where they carry meaning

#### Scenario: Motif is decoration only
- **WHEN** a screen reader or a print stylesheet processes the home page
- **THEN** the graticule contributes no announced content and does not print

#### Scenario: Numbers align
- **WHEN** a result panel lists several numeric rows
- **THEN** their digits are set in tabular figures and align in a column

## REMOVED Requirements

### Requirement: Sunlight, night, and high-contrast modes
**Reason**: Three specialist modes each carried their own token block, contrast audit, and (for `night`) a luminance gate and brightness slider, to serve cases that one obvious light/dark control plus the operating system's own accessibility settings cover well enough. Keeping five modes correct cost more than it returned, and a five-option select buried in the footer was not a usable field control anyway.

**Migration**: `sunlight` and `high-contrast` map to `paper`; `night` maps to `ink`; a saved `gp-theme` of any removed mode is read once and rewritten to its replacement, as the existing `hud` and `daylight` aliases already are. The `gp-dim` key and the `--dim` token are deleted. Visitors who need higher contrast are served by their browser and OS settings, which the site no longer overrides.
