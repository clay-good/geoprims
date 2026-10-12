import assert from 'node:assert/strict';
import { test } from 'node:test';
import { licenseProblems } from './npm-licenses.mjs';

test('the npm license gate rejects missing and unreviewed licenses', () => {
  const lock = {
    packages: {
      '': { name: 'fixture' },
      'node_modules/good': { version: '1.0.0', license: 'MIT' },
      'node_modules/missing': { version: '2.0.0' },
      'node_modules/new': { version: '3.0.0', license: 'Unknown-1.0' },
    },
  };
  assert.deepEqual(licenseProblems(lock), [
    'missing@2.0.0: no license in package-lock.json',
    'new@3.0.0: unreviewed license Unknown-1.0',
  ]);
  assert.deepEqual(licenseProblems({ packages: { 'node_modules/good': { version: '1.0.0', license: 'MIT' } } }), []);
});
