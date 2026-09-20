// The verification report and changelog pages (trust/proof-display,
// platform/verification "Verification report published").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const changelog = JSON.parse(readFileSync(join(root, 'data/changelog.json'), 'utf8'));
const page = (p) => readFileSync(join(dist, p, 'index.html'), 'utf8');
const v = catalog.coreVersion;

test('the release verification report lists every tool with counts, sources, error, and tolerance', () => {
  const report = JSON.parse(readFileSync(join(dist, 'verification', `${v}.json`), 'utf8'));
  assert.equal(report.version, v);
  assert.equal(report.counts.failures, 0);
  assert.equal(report.tools.length, catalog.tools.length);
  const html = page(`verification/${v}`);
  for (const t of catalog.tools) {
    const row = report.tools.find((r) => r.id === t.id);
    assert.equal(row.vectors, t.vectorCount, t.id);
    if (row.vectors) assert.ok(row.sources.length > 0, `${t.id} sources`);
    if (row.checks.numeric) assert.ok(row.worst.used <= 1, `${t.id} is outside its tolerance`);
    assert.ok(html.includes(`<tr id="${t.id}">`), `${t.id} row`);
  }
  assert.match(page('verification'), new RegExp(`href="/verification/${v.replaceAll('.', '\\.')}/"`));
});

test('the changelog shows every entry, labels result changes, and tool pages link their entries', () => {
  const html = page('changelog');
  // The page escapes ampersands, angle brackets, and both kinds of quote, so compare escaped text.
  const esc = (s) =>
    s
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#39;');
  for (const e of changelog.entries) assert.ok(html.includes(esc(e.summary.slice(0, 60))), e.summary.slice(0, 40));
  const results = changelog.entries.filter((e) => e.kind === 'result-change').length;
  assert.equal((html.match(/<span class="badge"[^>]*>Result change<\/span>/g) ?? []).length, results);
  const sun = page('time/sun/position');
  assert.match(sun, /NREL Solar Position Algorithm/);
  assert.ok(sun.includes(`/verification/${v}/#time.sun.position`));
  const footer = page('');
  assert.ok(footer.includes('href="/changelog/"') && footer.includes(`href="/verification/${v}/"`));
});

test('the "Use with agents" page shows every client snippet and the toolsets', async () => {
  const { CLIENTS, snippet } = await import('../../../mcp/clients.mjs');
  const { TOOLSETS } = await import('../../../mcp/toolsets.mjs');
  const html = page('agents');
  const esc = (s) => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
  for (const c of CLIENTS) {
    for (const how of ['clone', 'npx']) {
      const text = snippet(c, how);
      assert.ok(html.includes(text) || html.includes(esc(text)), `${c.name} ${how}`);
    }
  }
  assert.match(html, /&quot;servers&quot;|"servers"/);
  for (const name of Object.keys(TOOLSETS)) assert.ok(html.includes(`<code>${name}</code>`), name);
  assert.ok(page('').includes('href="/agents/"'));
});

test('the privacy page says what is kept, what is sent, and what never is', () => {
  const html = page('/privacy/');
  for (const promise of ['No accounts', 'No advertising', 'No analytics', 'No cookies', 'No third-party requests']) {
    assert.ok(html.includes(promise), `the privacy page does not promise: ${promise}`);
  }
  // The one exception, named, and the one thing that is ever sent.
  assert.match(html, /Turnstile/, 'names the bot check');
  assert.match(html, /only while the report dialog is open/);
  assert.match(html, /does not store your IP address/);
  assert.match(html, /gp-/, 'names the local-storage keys');
  // The note cap comes from the shared limits file, not a number typed twice.
  const limits = JSON.parse(readFileSync(join(root, 'data/report-limits.json'), 'utf8'));
  assert.ok(html.includes(`up to ${limits.noteChars} characters`), 'the note cap is the shared one');
});

test('the licenses page lists every registered dataset with its attribution', () => {
  // Astro escapes the apostrophes in dataset titles; compare the text a reader sees.
  const html = page('/licenses/').replaceAll('&#39;', "'").replaceAll('&amp;', '&').replaceAll('&quot;', '"');
  const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
  for (const a of registry.assets) {
    assert.ok(html.includes(a.title), `${a.id} is not listed`);
    assert.ok(html.includes(a.license), `${a.id} has no license`);
  }
  assert.match(html, /Open Font License/, 'the fonts are attributed');
  assert.match(html, /Made with Natural Earth/, 'the base map carries its required attribution');
  for (const left of ['what3words', 'Enhanced Magnetic Model', 'Natural Earth']) assert.ok(html.includes(left), left);
});

test('every page links privacy and licenses in its footer', () => {
  for (const route of ['/', '/tools/', '/aviation/', '/aviation/altimetry/density-altitude/', '/settings/']) {
    const html = page(route);
    assert.match(html, /<a href="\/privacy\/">Privacy<\/a>/, `${route} does not link privacy`);
    assert.match(html, /<a href="\/licenses\/">Licenses<\/a>/, `${route} does not link licenses`);
  }
});
