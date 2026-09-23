// Settings, recent and pinned tools (web/app-shell), against a stand-in
// localStorage. The browser check covers the pages that show them.
import { test } from 'node:test';
import assert from 'node:assert/strict';

const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => store.set(k, String(v)),
  removeItem: (k) => store.delete(k),
};
const events = [];
globalThis.dispatchEvent = (e) => events.push(e.type);
const prefs = await import('../src/lib/prefs.js');
const tool = (n) => ({ id: `units.test.t${n}`, title: `Tool ${n}`, route: `/units/test/t${n}/` });

test('recent tools: most recent first, no duplicates, at most 50', () => {
  for (const n of [1, 2, 3]) prefs.recordUse(tool(n));
  assert.deepEqual(prefs.recents().map((r) => r.title), ['Tool 3', 'Tool 2', 'Tool 1']);
  prefs.recordUse(tool(1));
  assert.deepEqual(prefs.recents().map((r) => r.title), ['Tool 1', 'Tool 3', 'Tool 2']);
  for (let n = 10; n < 80; n++) prefs.recordUse(tool(n));
  assert.equal(prefs.recents().length, prefs.MAX_RECENT);
  assert.equal(prefs.recents()[0].title, 'Tool 79');
  assert.ok(events.includes('gp-lists') && !events.includes('gp-prefs'), 'list changes do not re-run tools');
});

test('pins toggle', () => {
  prefs.togglePin(tool(5));
  assert.ok(prefs.isPinned('units.test.t5'));
  prefs.togglePin(tool(5));
  assert.ok(!prefs.isPinned('units.test.t5'));
});

test('the unit profile and number format become tool options', () => {
  assert.equal(prefs.toolOptions(), undefined, 'defaults send nothing');
  events.length = 0;
  prefs.setProfile('aviation');
  assert.deepEqual(prefs.toolOptions(), { profile: 'aviation' });
  prefs.setNumberFormat('decimal-comma');
  assert.deepEqual(prefs.toolOptions(), { profile: 'aviation', numberFormat: 'decimal-comma' });
  assert.deepEqual(events, ['gp-prefs', 'gp-prefs']);
  assert.equal(prefs.nextProfile('aviation'), 'aviation-hpa');
  assert.equal(prefs.nextProfile(prefs.PROFILES.at(-1)[0]), '', 'cycles back to each tool’s own units');
});

test('every profile id is one the core accepts', async () => {
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const host = nodeHost(new URL('../../../dist/wasm', import.meta.url).pathname);
  for (const [id] of prefs.PROFILES.filter(([p]) => p)) {
    const r = JSON.parse(await host.invoke('navigation.geodesic.inverse', JSON.stringify({ lat1: 40, lon1: -74, lat2: 51, lon2: 0, options: { profile: id } })));
    assert.ok(r.ok, `${id}: ${r.error?.message}`);
  }
  const r = JSON.parse(await host.invoke('navigation.geodesic.inverse', JSON.stringify({ lat1: 40, lon1: -74, lat2: 51, lon2: 0, options: { profile: 'aviation' } })));
  assert.equal(r.result.distance.unit, 'NM', 'aviation shows distance in NM');
});

test('the coordinate format is saved, defaults to decimal degrees, and never reaches the core', () => {
  assert.equal(prefs.coordFormat(), 'dd');
  events.length = 0;
  prefs.setCoordFormat('mgrs');
  assert.equal(prefs.coordFormat(), 'mgrs');
  assert.deepEqual(events, ['gp-prefs'], 'an open map rereads it');
  assert.ok(!('coordFormat' in (prefs.toolOptions() ?? {})), 'a display setting, not a tool option');
  store.set('gp-coord-format', JSON.stringify('bogus'));
  assert.equal(prefs.coordFormat(), 'dd', 'an unknown saved value reads as the default');
  prefs.setCoordFormat('dd');
});

test('motion and the default map view are saved, and reduce motion wins over the device', () => {
  globalThis.matchMedia = () => ({ matches: false });
  assert.equal(prefs.motion(), 'system');
  assert.equal(prefs.reducedMotion(), false);
  prefs.setMotion('reduce');
  assert.equal(prefs.reducedMotion(), true, 'the setting reduces motion on a device that does not');
  prefs.setMotion('system');
  globalThis.matchMedia = () => ({ matches: true });
  assert.equal(prefs.reducedMotion(), true, 'and the device still can');
  assert.equal(prefs.canvasDefault(), 'auto');
  prefs.setCanvasDefault('globe');
  assert.equal(prefs.canvasDefault(), 'globe');
  store.set('gp-canvas', JSON.stringify('cube'));
  assert.equal(prefs.canvasDefault(), 'auto', 'an unknown value reads as the default');
  prefs.setCanvasDefault('auto');
});
