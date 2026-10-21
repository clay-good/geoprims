// The practitioner-essentials catalog cut: promised search language stays on
// the intended tool, and the implemented operation count cannot drift silently.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const byId = new Map(catalog.tools.map((tool) => [tool.id, tool]));

test('practitioner essentials expose the promised aliases', () => {
  const expected = {
    'time.scale.utc-offset': 'Zulu time converter',
    'aviation.weather.metar-decode': 'METAR decoder',
    'time.sun.night-currency': 'night currency',
    'aviation.ifr.hold-entry': 'hold entry calculator',
    'survey.land.deed-plot': 'deed plotter',
    'survey.land.plss-parse': 'township range section',
  };
  for (const [id, alias] of Object.entries(expected)) {
    assert.ok(byId.get(id)?.aliases.includes(alias), `${id}: missing alias "${alias}"`);
  }
});

test('practitioner operation counts match the implemented catalog cut', () => {
  const groups = {
    time: new Set(['scale', 'sun']),
    aviation: new Set(['airspace', 'ifr', 'weather']),
    survey: new Set(['gnss', 'land']),
    drone: new Set(['links', 'sensors']),
  };
  const count = (domain) => catalog.tools.filter(
    (tool) => tool.domain === domain && groups[domain].has(tool.group) && tool.composedOf.length === 0,
  ).length;
  assert.deepEqual(
    Object.fromEntries(Object.keys(groups).map((domain) => [domain, count(domain)])),
    // time 17: the 14 practitioner tools plus unix-time, iso-week, and sidereal
    // (add-celestial-and-moon), which share the scale group.
    // aviation 16: the 12 practitioner tools plus approach-category,
    // rvr-visibility, visibility-to-cmv, and circling-radius
    // (add-aviation-practical-additions), which share the ifr group.
    { time: 17, aviation: 16, survey: 11, drone: 5 },
  );
});
