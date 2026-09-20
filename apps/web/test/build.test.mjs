// Checks the built site (run `npm run build` in apps/web first).
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const page = (route) => readFileSync(join(dist, route, 'index.html'), 'utf8');
const route = (id) => '/' + id.split('.').join('/') + '/';

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

test('one page per catalog endpoint', () => {
  for (const t of catalog.tools) assert.ok(existsSync(join(dist, route(t.id), 'index.html')), route(t.id));
  const toolPages = htmlFiles(dist).filter((p) => p.slice(dist.length).split('/').length === 5);
  assert.equal(toolPages.length, catalog.counts.all.endpoints);
});

test('every tool page carries the worked example answer in its HTML', () => {
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    assert.match(html, /class="value[^"]*">[^<]+</, `${t.id}: no answer value`);
    assert.match(html, /class="sentence[^"]*">[^<]+</, `${t.id}: no sentence`);
  }
});

test('generated endpoints canonicalize to their parent; experimental pages are noindex', () => {
  const pair = page('/units/speed/kt-to-mph/');
  assert.match(pair, /<link rel="canonical" href="https:\/\/geoprims\.com\/units\/speed\/convert\/">/);
  for (const t of catalog.tools.filter((x) => x.stability === 'experimental')) {
    assert.match(page(route(t.id)), /<meta name="robots" content="noindex">/, t.id);
  }
});

test('no third-party scripts, styles, or fonts', () => {
  for (const f of htmlFiles(dist)) {
    const html = readFileSync(f, 'utf8');
    for (const m of html.matchAll(/<(script|link)[^>]+(src|href)="(https?:)?\/\/([^/"]+)/g)) {
      const tag = m[0];
      if (/rel="canonical"/.test(tag)) continue;
      assert.fail(`${f.slice(dist.length)} loads ${m[4]}`);
    }
  }
});

test('ships the Wasm modules and the catalog at their contract routes', () => {
  assert.ok(existsSync(join(dist, 'catalog/v1.json')));
  for (const m of ['base', 'link']) assert.ok(existsSync(join(dist, 'wasm', `${m}.wasm`)), m);
});

test('every tool page has one report button and no bot-check script in its HTML', () => {
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    assert.equal((html.match(/>Report a problem</g) ?? []).length, 1, t.id);
    assert.ok(!/src="https:\/\/challenges\.cloudflare\.com/.test(html), `${t.id} loads the bot check before a click`);
  }
});

test('the known-issues page is published', () => {
  assert.match(page('/known-issues/'), /<h1>Known issues<\/h1>/);
});

const ALLOWED_LD = new Set(['WebApplication', 'BreadcrumbList', 'CollectionPage', 'Article', 'Dataset']);

test('structured data: only allowlisted JSON-LD types, valid, with < escaped', () => {
  for (const f of htmlFiles(dist)) {
    const html = readFileSync(f, 'utf8');
    for (const m of html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)) {
      assert.ok(!m[1].includes('<'), `${f.slice(dist.length)}: unescaped < in JSON-LD`);
      const o = JSON.parse(m[1]);
      assert.ok(ALLOWED_LD.has(o['@type']), `${f.slice(dist.length)}: ${o['@type']} is not allowlisted`);
      if (o['@type'] === 'WebApplication') {
        assert.equal(o.isAccessibleForFree, true);
        assert.equal(o.offers.price, 0);
        assert.ok(!('aggregateRating' in o) && !('review' in o));
      }
    }
  }
  assert.match(page('/aviation/altimetry/density-altitude/'), /"@type":"WebApplication"/);
  assert.match(page('/aviation/'), /"@type":"CollectionPage"/);
  assert.match(page('/aviation/altimetry/'), /"@type":"CollectionPage"/);
});

const unescape = (s) => s.replaceAll('&quot;', '"').replaceAll('&#39;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&');

test('every tool page has a developer block whose call reproduces the example', async () => {
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const host = nodeHost(join(web, '../../dist/wasm'));
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const m = /<pre class="agent-call[^"]*">([\s\S]*?)<\/pre>/.exec(html);
    assert.ok(m, `${t.id}: no agent call`);
    const call = JSON.parse(unescape(m[1]));
    assert.equal(call.arguments.id, t.id);
    const r = JSON.parse(await host.invoke(t.id, JSON.stringify(call.arguments.args)));
    const sentence = unescape(/class="sentence[^"]*">([^<]+)</.exec(html)[1]);
    assert.equal(r.summary, sentence, `${t.id}: the agent call does not reproduce the page`);
  }
});

test('discovery files come from the build', () => {
  const llms = readFileSync(join(dist, 'llms.txt'), 'utf8');
  assert.ok(llms.includes(`${catalog.counts.all.operations} operations and ${catalog.counts.all.endpoints} tool ids`), 'llms.txt counts');
  const surface = JSON.parse(readFileSync(join(web, '../../mcp/surface.json'), 'utf8'));
  const mcp = JSON.parse(readFileSync(join(dist, '.well-known/mcp.json'), 'utf8'));
  assert.deepEqual(mcp.tools, surface.tools.map((t) => t.name), 'mcp.json matches the golden surface');
  assert.equal(mcp.transport, 'stdio');
  const agents = readFileSync(join(dist, 'AGENTS.md'), 'utf8');
  assert.ok(agents.includes('node geoprims/mcp/server.mjs'));
  assert.ok(agents.includes('Relay the caveats'));
  assert.ok(readFileSync(join(dist, 'robots.txt'), 'utf8').includes('Sitemap: https://geoprims.com/sitemap-index.xml'));
});

test('sitemaps list only indexable, self-canonical pages', () => {
  const index = readFileSync(join(dist, 'sitemap-index.xml'), 'utf8');
  const maps = [...index.matchAll(/<loc>https:\/\/geoprims\.com(\/sitemaps\/[a-z-]+\.xml)<\/loc>/g)].map((m) => m[1]);
  assert.ok(maps.length > 0);
  let urls = 0;
  for (const m of maps) {
    for (const [, path] of readFileSync(join(dist, m), 'utf8').matchAll(/<loc>https:\/\/geoprims\.com([^<]*)<\/loc>/g)) {
      urls++;
      assert.doesNotMatch(page(path), /<meta name="robots" content="noindex">/, `${path} is noindex but in a sitemap`);
    }
  }
  assert.ok(urls > 0);
});

test('sources and methodology pages, and per-tool vector downloads', () => {
  const src = page('/sources/');
  assert.match(src, /Manual of the ICAO Standard Atmosphere/);
  assert.match(src, /Density altitude/, 'lists the tools citing ICAO Doc 7488');
  assert.match(page('/methodology/'), /experimental/);
  for (const t of catalog.tools) {
    assert.ok(page(route(t.id)).includes(`/vectors/${t.id}.jsonl`), t.id);
    assert.ok(existsSync(join(dist, 'vectors', `${t.id}.jsonl`)), `${t.id} vectors not shipped`);
  }
  assert.match(readFileSync(join(dist, 'llms.txt'), 'utf8'), /\/methodology\//);
});

test('every aviation, drone, and navigation tool shows the safety notice in its header', () => {
  const notice = '<strong>Planning and education aid. Not for primary navigation.</strong> <a href="/disclaimer/">Full disclaimer</a>';
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const operational = ['aviation', 'drone', 'navigation'].includes(t.domain);
    assert.equal(html.includes(notice), operational, t.id);
    // In the header: before the calculator, not in a dialog.
    if (operational) assert.ok(html.indexOf(notice) < html.indexOf('<astro-island'), `${t.id}: notice after the calculator`);
  }
  assert.match(page('/disclaimer/'), /Not for primary navigation\./);
});

test('the home page explains the product, then searches, then browses', () => {
  // web/page-template "The home page explains the product": description, one
  // search field, then the categories. Nothing personal or historical above them.
  const html = page('');
  const at = (re) => html.search(re);
  const description = at(/<p class="purpose">/);
  const search = at(/class="hero-search"/);
  const categories = at(/<ul class="categories">/);
  assert.ok(description > 0 && search > description, 'search follows the description');
  assert.ok(categories > search, 'categories follow the search');
  assert.ok(at(/class="pinned"/) > categories && at(/class="recent"/) > categories, 'recent and pinned come last');
  // Every domain the description names is a domain the catalog has.
  const NOUNS = { drones: 'drone', surveying: 'survey', 'spatial indexing': 'indexing', terrain: 'raster' };
  const domains = new Set(catalog.tools.map((t) => t.domain));
  const named = /Exact, cited calculators for ([^.]+)\./.exec(html.replace(/\s+/g, ' '))?.[1];
  assert.ok(named, 'the description names its subjects');
  for (const word of named.split(/,\s*/).map((w) => w.replace(/^and\s+/, '').trim()).filter(Boolean)) {
    assert.ok(domains.has(NOUNS[word] ?? word), `the home page claims "${word}", which the catalog does not have`);
  }
});

test('the catalog page lists every operation once', () => {
  // web/app-shell "The catalog page".
  const html = page('tools');
  const listed = [...html.matchAll(/<li data-text="[^"]*"><a href="([^"]+)"/g)].map((m) => m[1]);
  const wanted = catalog.tools.filter((t) => t.composedOf.length === 0).map((t) => route(t.id));
  assert.deepEqual([...listed].sort(), [...wanted].sort());
  assert.equal(new Set(listed).size, listed.length, 'no tool listed twice');
  // The ItemList is real JSON-LD covering the same tools.
  const ld = [...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1].replaceAll('\\u003c', '<')));
  const items = ld.find((o) => o['@type'] === 'CollectionPage')?.mainEntity?.itemListElement ?? [];
  assert.equal(items.length, wanted.length);
  // It is in the sitemap and linked from the home page and every footer.
  assert.ok(readdirSync(join(dist, 'sitemaps')).some((f) => readFileSync(join(dist, 'sitemaps', f), 'utf8').includes('https://geoprims.com/tools/')), 'in a sitemap');
  assert.match(page(''), /href="\/tools\/"/);
});

test('the home page leads with the search field, then a working featured tool', () => {
  const html = page('/');
  // Order on the page: search first, then the featured tool, then browsing.
  const search = html.indexOf('class="hero-search"');
  const featured = html.indexOf('class="featured"');
  const browse = html.indexOf('class="browse-title"');
  assert.ok(search > -1 && featured > search && browse > featured, 'search, then featured tool, then browse');
  // The answer is server-rendered, so the tool is useful before any JavaScript runs.
  assert.match(html, /class="value[^"]*">[^<]+</, 'no featured answer value');
  assert.match(html, /class="sentence[^"]*">[^<]+</, 'no featured sentence');
  // Embedded mode: it links out to the tool's own page and keeps the page's own chrome off.
  assert.match(html, /class="open-tool" href="\/navigation\/geodesic\/inverse\/#/, 'no link to the full tool');
  assert.ok(!html.includes('Report a problem'), 'the embedded copy should not offer the report dialog');
});

test('a status output reads as a judgment with a mark and its source, not as another value', () => {
  const html = page('/aviation/wind/runway-components/');
  const line = /<p class="status">([\s\S]*?)<\/p>/.exec(html);
  assert.ok(line, 'the crosswind page shows a status line');
  const text = line[1].replace(/<!--.*?-->/g, '').replace(/<[^>]+>/g, '');
  assert.match(text, /^[✓!✕] (Within|Near|Beyond) your .+ limit Against the limit you entered\.$/, text);
  assert.match(line[1], /<span class="mark" aria-hidden="true">/, 'the mark is decorative; the words carry the meaning');
  // The phrase must not also appear as a row in the facts list.
  const facts = /<dl class="facts">([\s\S]*?)<\/dl>/.exec(html)?.[1] ?? '';
  assert.doesNotMatch(facts, /Within your|Near your|Beyond your/, 'a status is not repeated as a fact');
});

test('the answer card frames the answer against a rule of thumb where the tool has one', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  const line = /<p class="comparison">([\s\S]*?)<\/p>/.exec(html);
  assert.ok(line, 'density altitude shows a comparison line');
  const text = line[1].replace(/<!--.*?-->/g, '').replace(/<[^>]+>/g, '').trim();
  assert.equal(text, 'The 120 ft per °C rule of thumb gives 8,123 ft (191 ft high).');
  // It sits under the sentence, not among the values.
  assert.ok(html.indexOf('<p class="sentence">') < html.indexOf('<p class="comparison">'));
});

test('the site serves byte-identical Wasm modules, not a stale copy', () => {
  const { modules } = JSON.parse(readFileSync(join(web, '../../dist/wasm/modules.json'), 'utf8'));
  const problems = [];
  for (const m of modules) {
    const file = join(dist, 'wasm', `${m.module}.wasm`);
    if (!existsSync(file)) {
      problems.push(`the site is missing ${m.module}.wasm`);
      continue;
    }
    const sha = createHash('sha256').update(readFileSync(file)).digest('hex');
    if (sha !== m.sha256) problems.push(`the site's ${m.module}.wasm is ${sha}, the build made ${m.sha256}`);
  }
  assert.deepEqual(problems, []);
  assert.ok(modules.length >= 10, `only ${modules.length} modules`);
});
