// Group hubs (web/tool-docs, "Domain and group index pages"): every group's
// tools are listed under the tasks they serve, each exactly once, and the
// airspeed hub carries the IAS → CAS → EAS → TAS → Mach guide.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { catalog, hubFor } from '../src/lib/catalog.mjs';

const web = new URL('..', import.meta.url).pathname;
const hubs = JSON.parse(readFileSync(join(web, '../../data/hubs.json'), 'utf8')).hubs;
const groups = [...new Set(catalog.tools.map((t) => `${t.domain}.${t.group}`))];

test('every group lists each of its tools under exactly one task', () => {
  for (const key of groups) {
    const [domain, group] = key.split('.');
    if (domain !== 'units') assert.ok(hubs[key], `${key}: no hub entry in data/hubs.json`);
    const hub = hubFor(domain, group);
    const listed = hub.tasks.flatMap((t) => t.tools.map((x) => x.id));
    const own = catalog.tools.filter((t) => t.domain === domain && t.group === group).map((t) => t.id);
    assert.deepEqual([...listed].sort(), [...own].sort(), `${key}: tasks must list every tool once`);
    for (const t of hub.tasks) assert.match(t.want, /^[a-z]/, `${key}: "${t.want}" reads after "I want to"`);
  }
  for (const key of Object.keys(hubs)) assert.ok(groups.includes(key), `${key}: no such group`);
});

test('every tool a hub names exists, including guide steps elsewhere', () => {
  for (const [key, hub] of Object.entries(hubs)) {
    const [domain, group] = key.split('.');
    const refs = [...hub.tasks.flatMap((t) => t.tools), ...(hub.guide?.steps ?? []).map((s) => s.tool).filter(Boolean)];
    for (const ref of refs) {
      const id = ref.includes('.') ? ref : `${domain}.${group}.${ref}`;
      assert.ok(catalog.tools.some((t) => t.id === id), `${key}: ${ref} is not a tool`);
    }
  }
});

test('group index: /aviation/airspeed lists every airspeed tool with its summary and the IAS → CAS → EAS → TAS → Mach guide', () => {
  const file = join(web, 'dist/aviation/airspeed/index.html');
  if (!existsSync(file)) return; // the build test covers a fresh build
  const html = readFileSync(file, 'utf8');
  const esc = (s) => s.replaceAll('&', '&amp;');
  for (const t of catalog.tools.filter((x) => x.domain === 'aviation' && x.group === 'airspeed')) {
    assert.ok(html.includes(`href="/aviation/airspeed/${t.id.split('.')[2]}/"`), `${t.id}: not linked`);
    assert.ok(html.includes(esc(t.summary)), `${t.id}: summary missing`);
  }
  assert.match(html, /IAS → CAS → EAS → TAS → Mach/);
  const guide = html.slice(html.indexOf('id="guide-title"'));
  const at = ['calibrated airspeed (CAS)', 'equivalent airspeed (EAS)', 'true airspeed (TAS)', 'Mach number'].map((w) => guide.indexOf(w));
  assert.ok(at.every((i) => i > 0) && at.every((i, k) => k === 0 || i > at[k - 1]), `guide order: ${at}`);
  assert.equal((html.match(/I want to<\/span>/g) ?? []).length, 3);
});
