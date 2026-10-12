import assert from 'node:assert/strict';
import { test } from 'node:test';
import { validateRelease } from './contract.mjs';

const pkg = { version: '1.2.3-rc.1', private: true, dependencies: {} };
const manifest = { version: '1.2.3-rc.1' };
const files = ['mcp/dist/wasm/modules.json', 'mcp/dist/catalog/v1.json', 'mcp/dist/assets/registry.json'];

test('a release candidate binds its tag to a private clone-and-run payload', () => {
  assert.deepEqual(validateRelease('v1.2.3-rc.1', pkg, manifest, files), { version: '1.2.3-rc.1', prerelease: true });
  assert.deepEqual(validateRelease('v1.2.3', { ...pkg, version: '1.2.3' }, { version: '1.2.3' }, files), { version: '1.2.3', prerelease: false });
});

test('the release contract fails closed on drift and registry publication', () => {
  assert.throws(() => validateRelease('release-1', pkg, manifest, files), /must be v<semver>/);
  assert.throws(() => validateRelease('v1.2.4', pkg, manifest, files), /disagrees/);
  assert.throws(() => validateRelease('v1.2.3-rc.1', { ...pkg, private: false }, manifest, files), /must be private/);
  assert.throws(() => validateRelease('v1.2.3-rc.1', pkg, manifest, files.slice(1)), /does not commit/);
});
