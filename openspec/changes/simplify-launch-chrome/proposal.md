## Why

Before launch the chrome still asked a visitor to read too much. Every footer carried a disclaimer paragraph, a privacy sentence, a version line, and an offline chip. The home page kept a Recent list with its own clear button. A Settings page held preferences that either already live on each tool (units) or that few people need, plus an "Erase all local data" button for a site that stores nothing but a few preferences in the browser.

Depends on: `redesign-minimal-shell`, which this narrows further.

## What Changes

- **Footer**: one centered row of links and nothing else: All tools, Units, Agents, Source, Privacy, Security, Accuracy, Disclaimer, Licenses. The disclaimer, privacy, and accuracy policies stay one click from every page; the disclaimer paragraph, version line, offline chip, and footer search button go.
- **Security page**: new at `/security/`, the reader-facing version of `SECURITY.md`: how to report privately, what is in scope, and what the design rules out.
- **Settings page removed**: `/settings/` is deleted, with its palette entry and its "Clear recent tools" and "Erase all local data" controls. The unit choice stays on each tool page. Clearing the site's data in the browser clears everything geoprims keeps.
- **Home page**: the Recent list and its clear button are removed. Pinned tools stay.

## Impact

Number format, coordinate format, default map view, the motion override, sounds, and Field mode lose their only on-screen control. Their stored values still apply if already set, and the OS reduced-motion preference still applies. The version and changelog links remain on tool pages ("How we got this").
