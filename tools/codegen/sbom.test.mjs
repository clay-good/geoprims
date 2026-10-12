import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { buildSbom, generateSbom } from './sbom.mjs';

const root = fileURLToPath(new URL('../..', import.meta.url));

test('the committed CycloneDX SBOM matches both lockfiles', () => {
  const committed = JSON.parse(readFileSync(`${root}/data/sbom.cdx.json`, 'utf8'));
  assert.deepEqual(committed, generateSbom());
  assert.equal(committed.bomFormat, 'CycloneDX');
  assert.equal(committed.specVersion, '1.6');
  assert.ok(committed.components.length > 300);
  assert.equal(new Set(committed.components.map((entry) => entry['bom-ref'])).size, committed.components.length);
  for (const entry of committed.components) {
    assert.ok(entry.licenses[0].expression, `${entry['bom-ref']} has no license`);
  }
});

test('SBOM generation fails closed on incomplete and duplicate packages', () => {
  assert.throws(() => buildSbom({
    npmLock: { packages: { 'node_modules/unknown': { version: '1.0.0' } } },
    cargoMetadata: { packages: [] },
  }), /requires version and license/);

  assert.throws(() => buildSbom({
    npmLock: { packages: {
      'node_modules/one': { name: 'same', version: '1.0.0', license: 'MIT' },
      'node_modules/two': { name: 'same', version: '1.0.0', license: 'MIT' },
    } },
    cargoMetadata: { packages: [] },
  }), /duplicate package URL/);
});
