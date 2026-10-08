// Tool chaining (web/app-shell, "Tool chaining"). An answer is often the next
// tool's input. This checks what "send to" offers, the single link that
// carries the value, and the breadcrumb the next page shows.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cameFrom, chainHref, chainState, chainTargets, MAX_TARGETS, routeOf, windHandoff } from '../src/lib/chain.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const byId = (id) => catalog.tools.find((t) => t.id === id);
const host = nodeHost(join(root, 'dist/wasm'));
const linkModule = await host.module('link');
const link = async (state) => JSON.parse(await linkModule.callString('gp_link_encode', JSON.stringify({ state })));
const unlink = async (fragment) => JSON.parse(await linkModule.callString('gp_link_decode', fragment));

test('a geodesic course can be sent to the wind triangle', async () => {
  // The scenario: the initial course from a geodesic inverse goes into the
  // wind-triangle tool's course input, and the page opens with it populated
  // and a breadcrumb back.
  const from = byId('navigation.geodesic.inverse');
  const targets = chainTargets(catalog.tools, from, 'azimuth1');
  const wind = targets.find((t) => t.id === 'aviation.wind.heading-groundspeed');
  assert.ok(wind, `the wind triangle is not offered: ${targets.map((t) => t.id).join(', ')}`);
  assert.equal(wind.field, 'course');

  const state = chainState(wind, '64.5 deg', from.id);
  const enc = await link(state);
  assert.ok(enc.ok, JSON.stringify(enc).slice(0, 200));
  const href = chainHref(wind, enc.result.fragment);
  assert.match(href, /^\/aviation\/wind\/heading-groundspeed\/#v1:/);

  // The link opens the wind tool with the course in it, and says where it came from.
  const back = await unlink(enc.result.fragment);
  assert.ok(back.ok);
  assert.equal(back.result.state.i.course, '64.5 deg');
  assert.deepEqual(cameFrom(back.result.state, catalog.tools), {
    id: 'navigation.geodesic.inverse',
    title: from.title,
    href: '/navigation/geodesic/inverse/',
  });
});

test('only tools that take the same kind of value are offered', () => {
  const from = byId('aviation.altimetry.density-altitude');
  for (const target of chainTargets(catalog.tools, from, 'density_altitude')) {
    const schema = byId(target.id).inputs.properties[target.field];
    assert.equal(schema['x-quantity'], from.outputs.properties.density_altitude['x-quantity'], target.id);
  }
});

test('a field that means the same thing comes first', () => {
  const from = byId('navigation.geodesic.inverse');
  const first = chainTargets(catalog.tools, from, 'azimuth1')[0];
  assert.match(`${first.fieldTitle}`.toLowerCase(), /course|azimuth/);
});

test('a coordinate is not offered: a point is two fields and a different gesture', () => {
  const from = byId('geodesy.utm.inverse');
  for (const name of Object.keys(from.outputs.properties)) {
    for (const target of chainTargets(catalog.tools, from, name)) {
      assert.ok(!/(^|_)(lat|lon|latitude|longitude)\d*$/.test(target.field), `${name} → ${target.id}.${target.field}`);
    }
  }
});

test('the list stays readable, and a tool never sends to itself', () => {
  let offered = 0;
  for (const t of catalog.tools) {
    for (const name of Object.keys(t.outputs.properties)) {
      const targets = chainTargets(catalog.tools, t, name);
      assert.ok(targets.length <= MAX_TARGETS, `${t.id}.${name}: ${targets.length} targets`);
      assert.ok(!targets.some((x) => x.id === t.id), `${t.id} sends to itself`);
      offered += targets.length;
    }
  }
  assert.ok(offered > 500, `only ${offered} destinations across the catalog`);
});

test('a link with no breadcrumb shows none, and an unknown one is ignored', () => {
  assert.equal(cameFrom({ i: {} }, catalog.tools), null);
  assert.equal(cameFrom({ c: 'not.a.tool' }, catalog.tools), null);
});

test('the page offers the control and knows what to do with it', () => {
  const html = readFileSync(join(web, 'dist/navigation/geodesic/inverse/index.html'), 'utf8');
  assert.match(html, /class="send-open"/, 'no send control on the page');
  assert.match(html, /aria-label="Send [^"]+ to another tool"/);
  const app = readFileSync(join(web, 'src/components/ToolApp.svelte'), 'utf8');
  // The catalog is fetched only when a reader asks, not on every page load.
  assert.match(app, /tools \?\?= \(await \(await fetch\('\/catalog\/v1\.json'\)\)\.json\(\)\)\.tools/);
  assert.match(app, /class="came-from"/);
});

test('routeOf turns a tool id into its path', () => {
  assert.equal(routeOf('aviation.wind.heading-groundspeed'), '/aviation/wind/heading-groundspeed/');
});

test('a METAR wind goes to the runway tool marked true, and the runway tool asks for the variation', async () => {
  // aviation/weather-decoding "Hand-off to runway components".
  const invoke = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));
  const metar = await invoke('aviation.weather.metar-decode', { report: 'KDEN 181753Z 30015G25KT 10SM FEW080 30/08 A2980' });
  const h = windHandoff('aviation.weather.metar-decode', metar);
  assert.equal(h.id, 'aviation.wind.runway-components');
  assert.deepEqual(h.state.i, { wind_direction: '300 deg', wind_speed: '15 kt', gust: '25 kt', wind_reference: 'true' });
  // The link carries it intact.
  const enc = await link(h.state);
  assert.ok(enc.ok);
  assert.deepEqual((await unlink(enc.result.fragment)).result.state.i, h.state.i);
  // With a magnetic runway number the runway tool asks for the variation, then answers.
  const asks = await invoke(h.id, { ...h.state.i, runway: '26' });
  assert.equal(asks.ok, false);
  assert.match(asks.error.message, /variation/);
  const answers = await invoke(h.id, { ...h.state.i, runway: '26', variation: '8' });
  assert.ok(answers.ok, JSON.stringify(answers.error));
  // A variable wind has no direction to hand off; other tools hand off nothing.
  const vrb = await invoke('aviation.weather.metar-decode', { report: 'KDEN 181753Z VRB03KT 10SM FEW080 30/08 A2980' });
  assert.equal(windHandoff('aviation.weather.metar-decode', vrb), null);
  assert.equal(windHandoff('aviation.wind.runway-components', metar), null);
  // One winds-aloft level goes to the wind triangle, also marked true.
  const fb = await invoke('aviation.weather.fb-winds-decode', { report: '731960', level: '34000 ft' });
  const w = windHandoff('aviation.weather.fb-winds-decode', fb);
  assert.equal(w.id, 'aviation.wind.heading-groundspeed');
  assert.deepEqual(w.state.i, { wind_direction: '230 deg', wind_speed: '119 kt', wind_reference: 'true' });
  const triangle = await invoke(w.id, { ...w.state.i, course: '090 deg', tas: '450 kt' });
  assert.ok(triangle.ok, JSON.stringify(triangle.error));
});
