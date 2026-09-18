# Research: feedback backend, SEO, mobile and field UX, trust patterns, MCP distribution

> Research brief gathered 2026-09-18. Items marked uncertain or unverified still need a primary-source check.


## 1. "Report a problem" backend on Cloudflare

**Platform choice.** Start on **Workers with static assets**, not Pages. Workers now does everything Pages does, and Cloudflare's advice is to put new projects on Workers ([migration guide](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/), [cogley.jp](https://cogley.jp/articles/cloudflare-pages-to-workers-migration)). Set `assets.run_worker_first: ["/api/*"]` so the Worker only runs for the API route. All other requests are served straight from static assets, which are "free and unlimited" on every plan ([routing](https://developers.cloudflare.com/workers/static-assets/routing/worker-script/), [pricing](https://developers.cloudflare.com/workers/platform/pricing/)). *Uncertain:* the docs example shows an exact path, not the `/api/*` wildcard. Test the wildcard before relying on it.

**Limits and prices, verified:**

| | Free | Paid ($5/mo) |
|---|---|---|
| Worker requests | 100,000/day, 10 ms CPU per request | 10M/mo included, then $0.30/M |
| D1 rows read / written | 5M / 100,000 per day (resets 00:00 UTC) | 25B / 50M per month included |
| D1 storage | 5 GB total, 500 MB per database, 10 databases | 10 GB per database |
| D1 queries per request | 50 | 1,000 |
| D1 Time Travel (restore window) | 7 days | 30 days |
| Workers Logs | 3-day retention, 200K events/day | 7 days |

Sources: [D1 pricing](https://developers.cloudflare.com/d1/platform/pricing/), [D1 limits](https://developers.cloudflare.com/d1/platform/limits/), [Workers Logs](https://developers.cloudflare.com/workers/observability/logs/workers-logs/). A feedback form will never get near these limits.

**Migrations.** Use `wrangler d1 migrations create | list | apply --local/--remote`. Migrations are numbered `.sql` files, and applied ones are tracked in the `d1_migrations` table. `migrations_dir` and `migrations_table` are configurable ([docs](https://developers.cloudflare.com/d1/reference/migrations/)). Run `apply --remote` in CI before `wrangler deploy`.

**Turnstile (bot check):**
- **Modes:** managed (recommended; shows a checkbox only when a visitor looks risky), non-interactive, and invisible.
- **Free plan:** unlimited challenges, 20 widgets, 10 hostnames per widget ([plans](https://developers.cloudflare.com/turnstile/plans/)).
- **Accessibility:** meets WCAG 2.2 AA ([overview](https://developers.cloudflare.com/turnstile/)).
- **What it collects:** client IP, TLS fingerprint, User-Agent, sitekey and origin. Cloudflare acts as a *processor* for the site owner, but also as a *controller* under "legitimate interests" when it uses the data to improve bot detection ([Turnstile privacy addendum](https://www.cloudflare.com/turnstile-privacy-policy/)). That controller role is the part that conflicts with a strict "no tracking" promise.
- **Cookies:** Turnstile itself sets none. The `cf_clearance` cookie appears only if you enable Pre-Clearance ([pre-clearance docs](https://developers.cloudflare.com/turnstile/reference/pre-clearance-support)).
- **GDPR:** the usual legal position is legitimate interest plus the ePrivacy "strictly necessary for security" exemption, so no consent banner. Invisible mode requires your privacy policy to reference the Turnstile addendum ([flowconsent](https://www.flowconsent.com/en/services/security/cloudflare-turnstile), [Friendly Captcha](https://friendlycaptcha.com/insights/cloudflare-turnstile-gdpr/)). *This is secondary-source legal analysis, not legal advice.*
- **Recommendation:** load Turnstile only when the user opens the report dialog, never site-wide, and disclose it in the dialog.

**Rate limiting:**
- **Workers Rate Limiting binding:** the period can only be 10 or 60 seconds. Counters are "permissive, eventually consistent," so it is not an exact count. Cloudflare warns against keying on IP because many users can share one ([docs](https://developers.cloudflare.com/workers/runtime-apis/bindings/rate-limit/)). *Pricing and GA status weren't stated on that page.*
- **WAF rate-limiting rules:** the Free plan gets 1 rule, and periods from 10 seconds upward are supported ([parameters](https://developers.cloudflare.com/waf/rate-limiting-rules/parameters/)).
- **Best combination:** a WAF rule on `POST /api/report` (Cloudflare counts per IP at the edge, so you never store it), plus the binding keyed on a coarse key such as the tool slug, plus a global daily cap enforced in D1.

**IP handling.**
- **Store no IPs at all.** Hashed IPs are still personal data under GDPR, because IPv4 has only about 4B values and a hash can be brute-forced. A daily salt doesn't really fix that.
- **Turn off invocation logs** (`[observability.logs] invocation_logs = false`) or sample them near 0. They record request metadata and are on by default, which would quietly break the no-IP promise. *The exact logged fields aren't listed in the docs, so treat them as including IP-derived data.*

**Spam patterns to expect:** link spam, SEO spam in free-text fields, and scripted POSTs. Countermeasures:
- Cap free text at about 2,000 characters.
- Reject submissions containing URLs, or flag them.
- Require a structured field (tool ID, inputs JSON, expected vs. actual).
- Add a honeypot field.
- Verify the Turnstile token server-side with siteverify.
- Validate Origin.

**Retention:** delete reports automatically 90–180 days after they are resolved, using a Cron Trigger that runs a `DELETE`. Publish the policy.

**Keeping the "no tracking" promise with user reports:**
1. Nothing is sent without an explicit click.
2. Before sending, show the **exact JSON payload** in a `<pre>` block, with checkboxes to include or exclude the inputs, user agent, and app version.
3. No email field, or an optional one clearly labeled.
4. No IP, cookie, or fingerprint stored.
5. A link to a public page describing what is stored and for how long.

**Mirroring reports to GitHub issues:**
- **Pros:** reports become public and easy to triage, can be linked from the "known issues" page, and cost nothing.
- **Cons:** you're pushing user-written text into public view, which brings PII risk and exposes spam to the world. A PAT has to live in Worker secrets. GitHub's secondary limits cap content creation at roughly 80 requests per minute and 500 per hour, with lower limits on some endpoints ([GitHub rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)).
- **Recommendation:** write to D1 first. Then either open an issue with only the structured fields (tool, inputs, expected vs. actual, no free text), or open it only after a maintainer approves the report. Use a fine-grained PAT scoped to Issues:write on one repo, or a GitHub App.

## 2. SEO for an ~800-page calculator site

**Scaled content abuse.** Google's spam policy (updated Aug 28, 2026) defines it as "many pages are generated for the primary purpose of manipulating search rankings and not helping users" ([spam policies](https://developers.google.com/search/docs/essentials/spam-policies)). What triggers it is pages that add nothing, not programmatic generation as such. To keep 800 pages from reading as thin:
- **Each page must do real, different work.** A distinct computation, distinct inputs, and a worked example with numbers specific to that tool.
- **Don't make one page per parameter value** ("density altitude at 5,000 ft", "at 6,000 ft"...). Combine near-duplicates into one tool with presets. If a variant page exists, it needs its own purpose, such as a different standard or edition.
- **Give each page substance:** a citation to the standard, "last verified" date, test vectors, and a limitations section. These are also the E-E-A-T signals Google looks for.
- **Link related tools** with purpose-driven links ("next: convert to pressure altitude"), not template link farms.

**Structured data in 2026:**

| Type | Status | Use? |
|---|---|---|
| `WebApplication` / `SoftwareApplication` | Rich result requires `offers.price` **and** a rating or review ([docs](https://developers.google.com/search/docs/appearance/structured-data/software-app)) | Add for meaning, with `price: 0`. You won't get stars unless you have genuine reviews. **Don't fake ratings.** |
| `BreadcrumbList` | Still active | Yes |
| FAQPage | FAQ rich results stopped appearing **May 7, 2026** ([Search updates](https://developers.google.com/search/updates)) | No |
| HowTo | Deprecated since 2023 | No |
| Practice Problem | Removed Jan 2026 | No |
| Dataset | Used only by Dataset Search, not Google Search (Nov 2025) | Only for real data pages (geoid grids, test vectors) |
| MathSolver | Still documented, but must be on the **home page** with a `SolveMathAction` that accepts a math *expression* ([docs](https://developers.google.com/search/docs/appearance/structured-data/math-solvers)) | Probably not. geoprims tools don't accept arbitrary expressions, and misusing it risks a manual action. *Judgment call.* |

**Core Web Vitals** (75th percentile of real users): LCP ≤ 2.5 s, **INP ≤ 200 ms**, CLS ≤ 0.1 ([web.dev](https://web.dev/articles/vitals)). For Wasm:
- Lazy-load and compile the module in the background (`WebAssembly.compileStreaming`).
- Render server-generated example results in the HTML so the largest element paints before Wasm loads.
- Keep heavy work off the main thread with a Worker so input handling stays fast.

**AI Overviews and LLMs.** Google's May 15, 2026 guide says its AI features draw on the normal Search index and ranking, and need no special files or markup. llms.txt is "not needed for Google Search" (June 2026 update) ([Search updates](https://developers.google.com/search/updates), [Search Engine Land](https://searchengineland.com/google-publishes-guide-on-optimizing-for-generative-ai-features-477671)). About 10% of sites have one, and one analysis found 97% of those files got no requests in the month studied ([digitalapplied](https://www.digitalapplied.com/blog/llms-txt-in-practice-adoption-evidence-2026); *third-party data*). Coding agents such as Cursor and Claude Code do read `/llms.txt`, so it's worth having for the MCP and developer docs. It is cheap to add and does nothing for Google.

**Sitemaps.** 50,000 URLs or 50 MB per sitemap. An index can list 50,000 sitemaps ([large sitemaps](https://developers.google.com/search/docs/crawling-indexing/sitemaps/large-sitemaps)). 800 URLs fit in one file, but splitting by suite (aviation, drone, survey...) gives per-section coverage reports in Search Console. Only change `lastmod` when the page content actually changes.

**hreflang:** not needed while the site is English-only.

**URL state.** Google "generally doesn't support URL fragments" ([URL structure](https://developers.google.com/search/docs/crawling-indexing/url-structure)). That suits geoprims:
- Keep user inputs in the `#fragment`. It never reaches the server, which is good for privacy, and it creates no duplicate URLs.
- Use a self-referencing canonical on the clean URL.
- If you ever use query parameters, canonicalize them to the base URL.

**OG images.** Generate them at build time for 800 pages (Satori or resvg in the build step), not at request time. Use a consistent template: tool name, suite, and an example result.

**IndexNow / Crawler Hints.** Cloudflare Crawler Hints sends IndexNow pings (to Bing and others) when it sees cache misses ([docs](https://developers.cloudflare.com/cache/advanced-configuration/crawler-hints/)). **Google does not use IndexNow.** *Uncertain:* whether Crawler Hints fires for Workers static-asset responses. It is simpler to send changed URLs to IndexNow directly from CI after each deploy.

## 3. Mobile and field UX

**Numeric inputs:**
- Use `type="text" inputmode="decimal"`, not `type="number"`. GOV.UK dropped `type=number` after user testing because of spinner, scroll-wheel, zoom and autofill problems ([GOV.UK](https://design-system.service.gov.uk/components/text-input/)). `type=number` also accepts `e`, drops locale decimal commas, and changes the value when you scroll the page over a focused field.
- Some on-screen keypads have **no minus key** (GOV.UK notes this). Signed values such as longitude or temperature need a ± toggle button or plain `type=text`.
- Parse both `,` and `.` as the decimal separator.
- Add `enterkeyhint="next"` on middle fields and `"done"` on the last one.
- Add `autocomplete="off"`, `autocorrect="off"`, `spellcheck="false"`.

**iOS zoom.** Inputs smaller than 16px trigger auto-zoom on focus. Set inputs to ≥ 16px. Don't use `maximum-scale=1`, which blocks zoom and hurts accessibility ([CSS-Tricks](https://css-tricks.com/16px-or-larger-text-prevents-ios-form-zoom/)).

**Tap targets.** WCAG 2.2 SC 2.5.8 (AA) requires at least 24×24 px. The comfortable size is 44 pt (Apple) or 48 dp (Material), which is also SC 2.5.5 AAA ([W3C](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)). For glove use, aim for 48–56 px with ≥ 8 px gaps. Avoid tiny steppers, and offer ±1 and ±10 buttons.

**Layout:**
- Put a sticky result bar at the bottom in the thumb zone, and keep it visible above the keyboard (use the `visualViewport` API).
- Keep the primary actions in the lower half of the screen.
- On landscape tablets (iPad used as an EFB in the cockpit), put inputs and results side by side so nothing scrolls. Test split-screen widths of about 320–500 px, since pilots often run the tool beside ForeFlight.

**Sunlight readability.** Offer a high-contrast theme: pure black on white, heavier font weights, no light-gray text, results at 24 px or larger. Go well beyond WCAG's 4.5:1 minimum for the result. *I found no authoritative sunlight-specific standard; this is best practice.*

**Night / cockpit mode.** The FAA says dim red light preserves dark adaptation (moderate adaptation within about 20 minutes). It also warns that red "severely distorts colors, especially on aeronautical charts," and that a bright light cancels dark adaptation within seconds ([AIM 8-1-6](https://faraim.org/faa/aim/chapter-8/section-8-1-6.html), [Airplane Flying Handbook ch. 11](https://www.faa.gov/sites/faa.gov/files/regulations_policies/handbooks_manuals/aviation/airplane_handbook/12_afh_ch11.pdf)). ForeFlight's answer is a dark theme, inverted charts, and an extra in-app dimming slider rather than pure red ([ForeFlight support](https://support.foreflight.com/hc/en-us/articles/204460425-How-do-I-prepare-ForeFlight-Mobile-for-night-operations)). So offer three themes: Dark, Night-red (red or amber on black, very low luminance), and an extra dim overlay. Never flash white during a transition. **In Night-red mode, color can't carry meaning**, so safety states need text or icons.

**Offline.** A Service Worker should precache the shell, the Wasm module and each suite's data. Show an "Available offline" status, and let users "save this suite for offline" because surveyors work without signal.

## 4. Glanceable, human-readable calculator UI

- **Pre-filled realistic example.** Load each page with an example that already computes, so the page teaches itself and the example is also the worked example.
- **Progressive disclosure.** Show 3–5 core inputs. Put advanced options (dew point, altimeter setting source, ellipsoid choice) behind "More options," with clear defaults like "ISA, WGS84."
- **Plain-language result sentence** under the number: "Density altitude is **7,930 ft**, about 2,900 ft above field elevation. Expect a longer takeoff roll and a weaker climb." Also give a rule-of-thumb comparison: "Rule of thumb (120 ft per °C above ISA) gives 7,800 ft." The rule of thumb is PA + 120 × (OAT − ISA) ([Pilot Institute](https://pilotinstitute.com/how-to-calculate-density-altitude/)).
- **Show your work.** Display the formula, then the same formula with the user's numbers substituted, then intermediate values. Omni does this with the formula, a worked example, and author and reviewer bylines, though its source citations are thin and it shows no visible "last updated" date ([Omni](https://www.omnicalculator.com/physics/density-altitude)). Wolfram's step-by-step is the benchmark for showing intermediate steps.
- **Say where the numbers come from.** Pix4D's GSD documentation cites its formulas explicitly ([Pix4D](https://support.pix4d.com/hc/en-us/articles/202559809)).
- **Unit toggles.** Handle units per field and remember the choice per viewer in localStorage (a convenience only, not tracking). Show both units in the result sentence where it helps.
- **Careful traffic-light framing.** Use "within / near / beyond typical limits," always with text and an icon (WCAG 1.4.1: don't rely on color alone), and cite the threshold's source. Add the caveat "planning aid; not a substitute for the POH/AFM." Never show a green "SAFE."
- **Accessibility.** Put the result region in `aria-live="polite"`, debounced so screen readers don't read every keystroke. Desmos is the reference for accessible math: screen-reader speech for expressions, audio trace of graphs, and braille support ([Desmos](https://help.desmos.com/hc/en-us/articles/4404860698253-Introduction-to-Accessibility-Features)).

## 5. Trust and proof patterns

- **Cite the standard with edition and section on every tool.** For example: "ICAO Doc 7488/3 (Manual of the ICAO Standard Atmosphere, 3rd ed.)" or "Karney 2013, J. Geodesy 87(1)."
- **Worked examples you can check against official sources:** FAA handbook examples, NGS NCAT/VERTCON outputs, and GeographicLib's **GeodTest** set of 500,000 WGS84 geodesics in 9 hard categories (antipodal, near-pole, etc.). The full file is about 39 MB; the 1/50 subset is about 690 KB ([GeographicLib](https://geographiclib.sourceforge.io/C++/doc/geodesic.html)). Publish your pass rate and maximum error against it.
- **"Last verified" date** per tool, the date its test vectors last passed against the reference, plus the library and model version (e.g., "WMM2025").
- **Public test vectors** as JSON in the repo, runnable by anyone, and the same vectors the MCP server uses.
- **Changelog** per tool and per release. Label any change that alters numeric results as a *result change*.
- **Public "Known issues" page** generated from triaged reports: the tool, what's wrong, the workaround, and the fixing version. Showing mistakes openly builds more trust than hiding them.
- **Open source plus reproducible builds.** Link each page to its source file and its test file.

## 6. MCP server distributed by GitHub clone

**Protocol status.** The current MCP spec revision is **2026-07-28**. It requires OAuth 2.1 for HTTP transports, which doesn't apply to a local stdio server ([security best practices](https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/security_best_practices)).

**README install snippets, verified:**

| Client | How to install |
|---|---|
| Claude Code | `claude mcp add --transport stdio geoprims -- node /abs/path/dist/index.js`. Scopes: `local` (default), `project` (writes `.mcp.json`), `user` ([docs](https://code.claude.com/docs/en/mcp)) |
| Claude Desktop | `mcpServers` block in `claude_desktop_config.json`, or a one-click **.mcpb** bundle: a zip with `manifest.json` that replaces `.dxt` ([mcpb repo](https://github.com/modelcontextprotocol/mcpb)) |
| VS Code | `.vscode/mcp.json` with top-level key **`servers`** (not `mcpServers`), or `code --add-mcp '{...}'`. Shows a trust prompt on first start ([VS Code docs](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)) |
| Cursor | `.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global), key `mcpServers` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json`, global only; uses `serverUrl` instead of `url` for remote servers ([mcpfind](https://mcpfind.org/blog/how-to-use-mcp-with-windsurf)) |

**`npx github:owner/repo#v1.2.3`.** npm accepts GitHub shorthand with a `#ref` pin ([package-spec](https://docs.npmjs.com/cli/v11/using-npm/package-spec)). npx picks the command from the package's single `bin` entry ([npx docs](https://docs.npmjs.com/cli/v11/commands/npx)). Caveat: a git install has to build the package on the user's machine, which is slow and runs install scripts. *The npx and package-spec pages don't document that build behavior. I'm fairly confident, but verify before relying on it.* Committing a prebuilt `dist/` on release tags, or shipping release tarballs, avoids the build.

**Prebuilt release artifacts.** Ship `.mcpb` and `.tgz` files with SHA-256 checksums and **GitHub artifact attestations** (`actions/attest-build-provenance`). Users verify with `gh attestation verify <file> -R owner/repo`. This is free for public repos ([GitHub docs](https://docs.github.com/en/actions/concepts/security/artifact-attestations)).

**Security section for the README:**
- Pin to a tag **and** commit SHA; don't track `main`.
- Verify checksums and attestations.
- Explain that a stdio server runs with the user's own privileges.
- State plainly: **no network access, no filesystem writes, no telemetry.** Enforce it and back it with a test.
- Keep tool descriptions short and static. This guards against "tool poisoning" (hidden instructions in tool descriptions) and "rug pulls" (behavior changing after an update) ([OWASP MCP cheat sheet](https://cheatsheetseries.owasp.org/cheatsheets/MCP_Security_Cheat_Sheet.html)).
- *Unverified:* a reported June 2026 NSA/DoD advisory on MCP and a "34% compliance" figure come from secondary sources. Don't cite them.

**Testing.** MCP Inspector ships as one package with web, `--cli` and `--tui` modes and needs Node ≥ 22.19. For example: `npx @modelcontextprotocol/inspector --cli node dist/index.js --method tools/list` ([docs](https://modelcontextprotocol.io/docs/tools/inspector)). Use the CLI mode in CI to snapshot the tool list and schemas, so any change to them is caught.

## Recommendations for geoprims

1. **Backend:** one Worker with static assets and `run_worker_first: ["/api/*"]`, D1 on the Free plan, migrations applied from CI. Turn invocation logs off.
2. **Reports:** explicit send button, exact payload preview, Turnstile loaded only in the dialog (disclosed, Pre-Clearance off), WAF rate rule plus a daily cap. **No IPs, not even hashed.** Auto-delete 180 days after resolution.
3. **GitHub mirroring:** only after maintainer approval, only structured fields, fine-grained token. Generate the public Known Issues page from D1.
4. **Scope:** merge near-duplicate variants. Every page needs a unique worked example, a cited standard (edition and section), limitations, test vectors, and a "last verified" date. Aim for fewer, deeper pages over 800 thin ones.
5. **Structured data:** `WebApplication` with `price: 0` and no ratings, plus `BreadcrumbList`. Skip FAQ, HowTo and MathSolver. Use `Dataset` only for real data files.
6. **URLs:** inputs in `#fragment`, clean canonical URLs, sitemaps split by suite, OG images built at deploy, IndexNow pinged from CI. Add llms.txt for developers only.
7. **Performance:** prerendered example results so the page paints before Wasm loads, Wasm compiled in the background, computation in a Web Worker. Target INP under 100 ms.
8. **Inputs:** `type=text inputmode=decimal`, fonts ≥ 16 px, 48 px targets, ± toggles, both decimal separators accepted.
9. **Themes:** Light, High-contrast (sunlight), Dark, and Night-red with dimming. Meaning never carried by color alone.
10. **Offline:** offline precache per suite with a visible status.
11. **Results:** a plain-language result sentence, formula with substituted values, and a rule-of-thumb comparison. Careful three-state framing with a POH/AFM disclaimer.
12. **MCP server:** tagged releases with a committed `dist/`, a `.mcpb` bundle, checksums and attestations. README snippets for all five clients, with a pinned `npx github:geoprims/mcp#vX.Y.Z` option. Inspector CLI in CI. It should share its test vectors with the website.
