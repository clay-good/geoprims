// Example parity, the web half (contracts/manifest-extensions, "The primary
// worked example"). Everywhere an example appears on the site it must be the
// same example the MCP server runs by default: the answer the page ships in
// its HTML, and the readouts on the home page's instrument panel. The MCP half
// lives in mcp/server.test.mjs and tools/mcp/parity.test.mjs.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const route = (id) => '/' + id.split('.').join('/') + '/';
const page = (r) => readFileSync(join(dist, r, 'index.html'), 'utf8');

const primaryOf = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];

/** The answer card's value as a reader sees it ("7,932 ft"). */
export function pageAnswer(html) {
  const raw = /<div class="value"[^>]*>([\s\S]*?)<\/div>/.exec(html)?.[1];
  if (raw === undefined) return undefined;
  return raw
    .replace(/<!--.*?-->/g, '')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, ' ')
    .trim();
}

/** The value and unit the core produces for a result's first present output. */
async function coreAnswer(t, input) {
  const r = JSON.parse(await host.invoke(t.id, JSON.stringify(input)));
  if (!r.ok) return undefined;
  const first = Object.keys(t.outputs.properties).find((k) => r.display?.[k] !== undefined);
  return first === undefined ? undefined : r.display[first];
}

test('every tool page ships the answer to its own primary example', async () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const shown = pageAnswer(html);
    const expected = await coreAnswer(t, primaryOf(t).input);
    if (shown !== expected) problems.push(`${t.id}: the page shows "${shown}", the primary example gives "${expected}"`);
  }
  assert.deepEqual(problems, []);
});

test("the home page's readouts are the answers each tool's page shows", () => {
  const home = page('/');
  const gauges = [...home.matchAll(/<li class="gauge"><a href="([^"#]+)#example">[\s\S]*?<span class="gauge-value">([^<]+)</g)];
  assert.ok(gauges.length >= 6, 'the home page has no instrument panel');
  for (const [, href, value] of gauges) {
    assert.ok(catalog.tools.some((t) => route(t.id) === href), `${href} is not a tool route`);
    assert.ok(page(href).includes(value.trim()), `${href}: the home page shows ${value}, its page does not`);
  }
});

test('a page whose answer drifted from its example is caught', async () => {
  const t = catalog.tools.find((x) => x.id === 'aviation.altimetry.density-altitude');
  const html = page(route(t.id));
  assert.equal(pageAnswer(html), '7,937 ft');
  const drifted = html.replaceAll('7,937', '7,900');
  assert.notEqual(pageAnswer(drifted), await coreAnswer(t, primaryOf(t).input));
});


/** An explainer's live tool: its `example`, or else the first of its `tools`. */
function liveTool(markdown) {
  const front = /^---\n([\s\S]*?)\n---/.exec(markdown)?.[1] ?? '';
  const example = /^example:\s*(\S+)/m.exec(front)?.[1];
  const first = /^tools:\s*\n\s*-\s*(\S+)/m.exec(front)?.[1];
  return example ?? first;
}

test("every explainer's live example shows the answer its tool's page shows", async () => {
  const { readdirSync } = await import('node:fs');
  const dir = join(web, 'src/content/learn');
  const problems = [];
  let checked = 0;
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.md'))) {
    const slug = f.replace(/\.md$/, '');
    const id = liveTool(readFileSync(join(dir, f), 'utf8'));
    if (!catalog.tools.some((t) => t.id === id)) {
      problems.push(`${slug}: live tool ${id} is not in the catalog`);
      continue;
    }
    const shown = pageAnswer(page(`/learn/${slug}/`));
    const onPage = pageAnswer(page(route(id)));
    checked++;
    if (shown === undefined) problems.push(`${slug}: the explainer shows no answer`);
    else if (shown !== onPage) problems.push(`${slug}: the explainer shows "${shown}", ${id}'s page shows "${onPage}"`);
  }
  assert.ok(checked >= 25, `only ${checked} explainers checked`);
  assert.deepEqual(problems, []);
});

test("every tool's share card carries the answer its page shows", () => {
  // Recorded by scripts/og.mjs as it draws the cards, since the attributes a
  // card is drawn from are stripped from the pages that ship.
  const drawn = JSON.parse(readFileSync(join(web, 'node_modules/.cache/og/drawn.json'), 'utf8'));
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const card = /<meta property="og:image" content="https:\/\/[^/]+(\/og\/[^"]+)"/.exec(html)?.[1];
    if (!card) {
      problems.push(`${t.id}: no share card`);
      continue;
    }
    // A preset shares its canonical page's card. A card carries the answer
    // only when it reads at a glance, 24 characters or fewer, and otherwise
    // none rather than a different one.
    const canonical = /<link rel="canonical" href="https:\/\/[^/]+([^"]+)"/.exec(html)?.[1];
    const drawnFrom = canonical && canonical !== route(t.id) ? page(canonical) : html;
    const shown = pageAnswer(drawnFrom) ?? '';
    const expected = shown.length <= 24 ? shown : '';
    if (!(card in drawn)) problems.push(`${t.id}: its card ${card} was not drawn`);
    else if ((drawn[card] ?? '') !== expected) problems.push(`${t.id}: the card says "${drawn[card]}", the page "${shown}"`);
  }
  assert.deepEqual(problems, []);
});
