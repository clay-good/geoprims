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
  // The page escapes quotes and ampersands, so compare escaped text.
  const esc = (s) => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
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
