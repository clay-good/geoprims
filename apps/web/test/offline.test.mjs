// The offline status chip (ux/mobile-and-field). The service worker caches the
// whole release at once, so a page is either served by a worker that holds it
// or it is not; the chip says which, and never claims offline use before the
// release is actually cached.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { offlineStatus, showStatus } from '../src/lib/offline.mjs';

const web = new URL('..', import.meta.url).pathname;

test('a page a worker serves works offline', () => {
  assert.deepEqual(offlineStatus({ controlled: true }), { state: 'ready', text: 'Works offline' });
});

test('a release still downloading does not claim offline use', () => {
  assert.deepEqual(offlineStatus({ installing: true }), { state: 'saving', text: 'Saving for offline' });
  assert.deepEqual(offlineStatus({}), { state: 'pending', text: 'Needs the network' });
});

test('a browser without service workers is told plainly', () => {
  assert.deepEqual(offlineStatus({ supported: false, controlled: true }), { state: 'unavailable', text: 'Needs the network' });
});

test('the chip stays hidden until there is something to say', () => {
  const chip = { hidden: true, dataset: {}, textContent: '' };
  showStatus(null, offlineStatus({ controlled: true }));
  assert.equal(chip.hidden, true);
  showStatus(chip, offlineStatus({ controlled: true }));
  assert.deepEqual([chip.hidden, chip.textContent, chip.dataset.state], [false, 'Works offline', 'ready']);
});


test('the update prompt is wired to the registration, not to a guess', () => {
  const base = readFileSync(join(web, 'src/layouts/Base.astro'), 'utf8');
  assert.match(base, /registerOffline\(document\.querySelector\('\.update'\), null\)/);
  // The first install takes control without a reload.
  const pwa = readFileSync(join(web, 'src/lib/pwa.js'), 'utf8');
  assert.match(pwa, /controllerchange[\s\S]*?say\(\{ controlled: true \}\)/);
});
