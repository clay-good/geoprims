// The report payload the dialog builds passes the Worker's own validator, holds
// only the contract's fields, and drops x-private inputs (feedback/problem-reports).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildPayload, buildSitePayload, openState, reportText, sendState, tokenIsFresh, TOKEN_MAX_AGE_MS } from '../src/lib/report.js';
import { KEYS, validate } from '../../../worker/src/report.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const versions = new Map(catalog.tools.map((t) => [t.id, t.version]));
const host = nodeHost(join(root, 'dist/wasm'));
const display = { theme: 'light', unitProfile: 'default', viewportClass: 'phone' };

async function payloadFor(t, over = {}) {
  const ex = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  const result = JSON.parse(await host.invoke(t.id, JSON.stringify(ex)));
  const tool = { ...t, coreVersion: catalog.coreVersion, buildHash: '0123456789abcdef' };
  return buildPayload({ tool, args: ex, result, includeInputs: true, note: 'Expected a different value', kind: 'wrong-result', display, pagePath: `/${t.id.split('.').join('/')}/#v1:x`, token: 't', ...over });
}

test('every tool: the dialog payload passes the Worker validator', async () => {
  for (const t of catalog.tools) {
    const p = await payloadFor(t);
    assert.deepEqual(Object.keys(p).sort(), [...KEYS].sort(), t.id);
    assert.ok(validate(p, versions), `${t.id} payload rejected by the Worker`);
  }
});

test('no identifying fields in the payload schema', async () => {
  const p = await payloadFor(catalog.tools[0]);
  const text = JSON.stringify(p);
  for (const banned of ['userAgent', 'screen', 'language', 'timeZone', 'referrer', 'ip']) assert.ok(!Object.hasOwn(p, banned), banned);
  assert.ok(!/Mozilla|Chrome|Safari/.test(text));
});

test('excluding inputs drops rows and the permalink fragment', async () => {
  const t = catalog.tools.find((x) => x.id === 'aviation.altimetry.density-altitude');
  const p = await payloadFor(t, { includeInputs: false });
  assert.deepEqual(p.inputs, []);
  assert.deepEqual(p.outputs, []);
  assert.equal(p.pagePath, '/aviation/altimetry/density-altitude/');
});

test('x-private inputs are never sent and outputs are withheld', async () => {
  const t = structuredClone(catalog.tools.find((x) => x.id === 'aviation.altimetry.density-altitude'));
  t.inputs.properties.elevation['x-private'] = true;
  const p = await payloadFor(t);
  assert.ok(!p.inputs.some((r) => r.field === 'elevation'));
  assert.ok(p.outputs.every((r) => r.value === '(withheld)'));
});

test('copy-report text carries no token', async () => {
  const p = await payloadFor(catalog.tools[0], { token: 'secret-token' });
  assert.ok(!reportText(p).includes('secret-token'));
});

test('the dialog opens in the state the connection and the switch put it in', () => {
  const on = { enabled: true, sitekey: 'x' };
  assert.equal(openState({ online: true, config: on }), 'ready');
  assert.equal(openState({ online: false, config: on }), 'offline', 'offline before anything is asked');
  assert.equal(openState({ online: true, config: { enabled: false } }), 'paused', 'the kill switch');
  assert.equal(openState({ online: true, config: null }), 'paused', 'config unreachable');
  assert.equal(openState({ online: false, config: null }), 'offline');
});

test('only the uniform 202 counts as sent', () => {
  assert.equal(sendState(202), 'sent');
  for (const code of [200, 400, 405, 429, 500, 503]) assert.equal(sendState(code), 'failed', String(code));
});

test('the slow note: a token older than the window is asked for again', () => {
  const now = 1_800_000_000_000;
  assert.equal(tokenIsFresh('t', now, now), true);
  assert.equal(tokenIsFresh('t', now - TOKEN_MAX_AGE_MS + 1, now), true, 'just inside the window');
  assert.equal(tokenIsFresh('t', now - TOKEN_MAX_AGE_MS, now), false, 'exactly at the window');
  assert.equal(tokenIsFresh('t', now - 6 * 60_000, now), false, 'six minutes writing a note');
  assert.equal(tokenIsFresh('', now, now), false, 'no token yet');
});

test('a page report from any page is one the Worker accepts, and names no GitHub issue', () => {
  const display = { theme: 'paper', unitProfile: 'default', viewportClass: 'desktop' };
  const site = { coreVersion: '0.1.0', buildHash: '0123456789abcdef' };
  for (const pagePath of ['/', '/privacy/', '/aviation/altimetry/', '/journeys/vfr-preflight/', '/nope/404-path']) {
    const p = buildSitePayload({ site, note: 'The filter did nothing', kind: 'broken', display, pagePath: `${pagePath}#stale`, token: 't' });
    assert.deepEqual(Object.keys(p), KEYS);
    assert.equal(p.pagePath, pagePath, 'the fragment is dropped');
    assert.ok(validate(p, new Map()), `${pagePath} rejected by the Worker`);
  }
  assert.doesNotMatch(reportText(buildSitePayload({ site, note: '', kind: 'other', display, pagePath: '/' })), /github/i);
});

