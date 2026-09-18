## 1. Anatomy and answer card

- [ ] 1.1 Build the fixed tool-page layout with the answer card above inputs and sticky on phones; verify the answer-first and anatomy scenarios
- [ ] 1.2 Build the answer card (value, sentence, comparison, warnings, status phrase, copy actions); verify the density-altitude sentence scenario
- [ ] 1.3 Add `x-sentence` templates and comparison types to every stable manifest, with the readability lint; verify the template-coverage and MCP-summary scenarios
- [ ] 1.4 Implement status phrases with icons, cited thresholds, and a banned-word lint; verify the crosswind scenario
- [ ] 1.5 Implement prefilled examples with the "Example values" chip, Clear, and Try the example; verify the first-load scenario
- [ ] 1.6 Implement progressive disclosure with worded defaults; verify the dry-air scenario
- [ ] 1.7 Build the glossary file and tap-to-define popovers, plus the help-text gate; verify the HAE scenario

## 2. Home page

- [ ] 2.1 Build the glanceable home (question search, hero cards with no-JS answers, journeys, recents, four promises); verify the question-on-home scenario and no-JS rendering

## 3. Mobile and field

- [ ] 3.1 Implement the numeric input contract with ± toggle and decimal-comma support, plus the attribute gate; verify both input scenarios
- [ ] 3.2 Implement 48 px targets and Field mode (56 px, larger results, step buttons); verify the touch-target scenario
- [ ] 3.3 Implement the sticky answer bar tracking the visual viewport; verify the keyboard-open scenario on iOS and Android emulation
- [ ] 3.4 Implement responsive layouts (320 px up, side-by-side from 900 px and on landscape tablets, split-screen); verify the landscape-iPad scenario
- [ ] 3.5 Build the mobile quality gates (full 320 px sweep, sampled landscape and 200% zoom, Chromium and WebKit, 2xx guard); verify the overflow scenario
- [ ] 3.6 Wire one-tap sunlight and night modes, with the night canvas palette and no bright frames; verify the one-tap scenario and a luminance check
- [ ] 3.7 Implement offline status chips and "Save this domain for offline"; verify the save-domain scenario
- [ ] 3.8 Implement Web Share and the one-page mobile calculation sheet; verify the share scenario

## 4. Usability validation

- [ ] 4.1 Recruit 5 testers per domain and run 5-second and task tests on hero tools; verify results recorded in `docs/usability/` and the 80% bars met, or pages revised and retested
