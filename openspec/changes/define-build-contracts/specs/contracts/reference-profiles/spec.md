## Purpose

Defines the single set of devices, networks, viewports, and engines against which every performance, mobile, and interaction budget is measured, so budgets in different specs mean the same thing.

## ADDED Requirements

### Requirement: Reference profile
Every budget in any geoprims spec SHALL be measured on this profile unless it names another:

| Aspect | Profile |
|---|---|
| CPU | Desktop Chromium on the CI runner with 4× CPU slowdown (a proxy for a mid-tier 2024 Android phone), plus a quarterly manual check on a real mid-tier Android device and a 3-year-old iPhone |
| Network (cold loads) | 9 Mbps down, 1.5 Mbps up, 150 ms round-trip time, no cache |
| Warm loads | Service worker installed, same network |
| Engines | Chromium, WebKit, Firefox (current stable); Node.js active LTS for MCP and the core |
| Measurement | Median of 3 runs per route; page metrics from Playwright traces and the web-vitals library (in CI only) |

#### Scenario: Budget uses the profile
- **WHEN** any performance gate runs
- **THEN** it applies this profile, and its report names the profile version

### Requirement: Reference viewports
Mobile and layout gates SHALL use these viewports:
- 320 × 720 (smallest phone)
- 390 × 844 (typical phone; the answer-above-the-fold check)
- 568 × 320 (phone landscape)
- 768 × 1024 (tablet portrait)
- 1180 × 820 (tablet landscape, EFB)
- 1440 × 900 (desktop)

Text zoom at 200% SHALL be checked at 375 px.

#### Scenario: Above-the-fold check
- **WHEN** the anatomy gate runs at 390 × 844
- **THEN** the answer value is inside the initial viewport for every hero tool

### Requirement: Profile changes are versioned
Changes to the reference profile SHALL be versioned, recorded in `docs/performance.md`, and SHALL reset the performance baselines with a comparison run on both the old and new profiles.

#### Scenario: Profile bump
- **WHEN** the CPU slowdown changes from 4× to 6×
- **THEN** the profile version increments, and the baseline report shows both runs
