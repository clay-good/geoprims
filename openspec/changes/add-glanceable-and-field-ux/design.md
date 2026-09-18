## Context

Research: `docs/research/05` §3–4 and `docs/research/06` §3. Key facts:

- **GOV.UK dropped `type=number`** after testing: spinners, scroll-wheel changes, and a missing minus key on some keypads.
- **iOS zooms** into inputs under 16 px.
- **Target size.** WCAG 2.5.8 requires 24 px; comfortable touch targets are 44–48 px.
- **Night vision.** The FAA AIM says dim red preserves dark adaptation but distorts chart colors. ForeFlight uses a dark theme plus a dim slider.
- **roughlogic.com** uses 48 px targets, sweeps every shell at 320 px, samples 200% zoom and landscape, runs WebKit, and puts the answer above the inputs on phones.

## Goals / Non-Goals

**Goals:**
- A stranger understands any tool in 5 seconds.
- A practitioner in the field can use it one-handed, in sun or dark, offline.

**Non-Goals:**
- Novel interaction paradigms. Familiar patterns win.

## Decisions

### U1. Sentence templates live in manifests
Templates are data with typed placeholders (`{da:ft}`, `{delta:ft}`) plus conditional clauses. They are rendered by the core, so web and MCP produce identical text. A readability lint (Flesch-Kincaid grade ≤ 8) runs over rendered examples.

### U2. Comparison lines are chosen per tool
Each manifest names its comparison type:
- `vs-input` (e.g. DA vs field elevation)
- `vs-rule-of-thumb`
- `vs-typical-range` (with source)
- `none`

Tools without a meaningful comparison say nothing, rather than inventing one.

### U3. Visual language
The signature HUD mode remains, but the layout is calm: generous spacing, one accent color, and big numerals in a tabular monospace. Effects stay off the numbers. `daylight` is the default for light-preference users. The HUD becomes an identity accent (canvas, header, icons) rather than a filter over everything, which answers the "delightful, simple" goal.

### U4. Field mode as a setting, not a separate app
Field mode is a CSS token override (targets, type scale, step buttons) plus behavior (step buttons, larger sticky bar). It is kept in local preferences.

### U5. Usability testing method
Remote unmoderated 5-second tests plus moderated task tests with 5 users per domain: pilots from a flying club, surveyors from a state society, drone pilots from a Part 107 community, and developers. Recruitment is by invitation, and no analytics are involved.

## Risks / Trade-offs

- **[Prefilled examples mislead a hurried user into thinking the numbers are theirs]** → A persistent "Example values" chip. The chip and label clear on the first edit, and the answer card reads "Example:" while untouched.
- **[Sentence templates oversimplify]** → Templates carry the key caveat, and the proof panel sits one tap away.
- **[Five core inputs is too few for some tools (W&B)]** → Tools with repeating rows (stations, traverse courses) count the row group as one core input.

## Migration Plan

Not applicable.

## Open Questions

None that affect the specs.
