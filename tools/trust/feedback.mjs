// The feedback-loop gate (add-problem-reporting task 3.1): one place that
// checks every link of the report loop agrees. Each check is pure and returns
// a list of problems, so the test runs it on the real build and on a bad
// fixture for each way it can break.
//
// 1. Pages: every tool page has exactly one "Report a problem" button in the
//    tool and one in the footer (which opens the same report), and no
//    bot-check script before the click.
// 2. Lazy import: the dialog is only reached through a dynamic import().
// 3. Limits: the client and the Worker read the one limits file, and the D1
//    migration is the one generated from it.
// 4. Service worker: the report API is never handled, cached, or precached.
// 5. CSP: the bot check may load its script and frame, and reports post only
//    to the site's own origin.

export const TURNSTILE = 'https://challenges.cloudflare.com';

export function checkPages(pages) {
  const out = [];
  for (const { id, html } of pages) {
    const at = html.indexOf('<footer class="site">');
    const count = (s) => (s.match(/>Report a problem</g) ?? []).length;
    const n = count(at < 0 ? html : html.slice(0, at));
    if (n !== 1) out.push(`${id}: ${n} report buttons, want 1`);
    if (at >= 0 && count(html.slice(at)) !== 1) out.push(`${id}: the footer has no "Report a problem" button`);
    if (html.includes(`src="${TURNSTILE}`)) out.push(`${id}: loads the bot check before a click`);
  }
  return out;
}

export function checkLazyImport(sources) {
  const out = [];
  for (const [path, src] of Object.entries(sources)) {
    if (/^\s*import\s+[\w{}\s,]+\s+from\s+['"][^'"]*ReportDialog\.svelte['"]/m.test(src)) out.push(`${path}: imports the report dialog statically`);
  }
  if (!Object.values(sources).some((s) => /import\(\s*['"][^'"]*ReportDialog\.svelte['"]\s*\)/.test(s))) out.push('no dynamic import() of the report dialog');
  return out;
}

export function checkLimits({ clientSrc, workerSrc, migration, generated }) {
  const out = [];
  const reads = /import\s+LIMITS\s+from\s+['"][./]+data\/report-limits\.json['"]/;
  if (!reads.test(clientSrc)) out.push('the client does not read data/report-limits.json');
  if (!reads.test(workerSrc)) out.push('the Worker does not read data/report-limits.json');
  if (migration !== generated) out.push('the D1 migration differs from the one generated from data/report-limits.json');
  return out;
}

export function checkServiceWorker({ source, precache }) {
  const out = [];
  const fetchAt = source.indexOf("addEventListener('fetch'");
  const guard = source.indexOf("pathname.startsWith('/api/')) return", fetchAt);
  const respond = source.indexOf('respondWith', fetchAt);
  if (fetchAt < 0 || guard < 0 || (respond >= 0 && guard > respond)) out.push('the service worker does not return before handling /api/');
  if (precache.some((u) => u.startsWith('/api/'))) out.push('the service worker precaches the report API');
  return out;
}

export function checkCsp(directives) {
  const out = [];
  const get = (name) => directives.find((d) => d[0] === name) ?? [];
  for (const name of ['script-src', 'frame-src']) {
    if (!get(name).includes(TURNSTILE)) out.push(`${name} does not allow the bot check`);
  }
  const connect = get('connect-src');
  if (!connect.includes("'self'")) out.push("connect-src does not allow posting reports to 'self'");
  if (connect.includes(TURNSTILE) || connect.includes('*')) out.push('connect-src lets reports go somewhere other than the site');
  return out;
}
