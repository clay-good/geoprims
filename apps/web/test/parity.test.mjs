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
  assert.equal(pageAnswer(html), '7,932 ft');
  const drifted = html.replaceAll('7,932', '7,900');
  assert.notEqual(pageAnswer(drifted), await coreAnswer(t, primaryOf(t).input));
});

