import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { sources, validateCof, validateTestValues } from './magnetic-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('the committed magnetic files are the pinned issuer files', () => {
  for (const [name, source] of Object.entries(sources)) {
    for (const part of source.coefficient ? [source.coefficient, source.tests] : [source]) {
      const bytes = readFileSync(join(root, part.output));
      assert.equal(bytes.length, part.bytes, `${name} ${part.output} bytes`);
      assert.equal(sha256(bytes), part.sha256, `${name} ${part.output} sha256`);
    }
  }
});

test('both WMM coefficient files and published test vectors have the expected shape', () => {
  for (const [name, source] of Object.entries({ wmm2025: sources.wmm2025, wmmhr2025: sources.wmmhr2025 })) {
    validateCof(readFileSync(join(root, source.coefficient.output)), source.coefficient.degree, name);
    validateTestValues(readFileSync(join(root, source.tests.output)), source.tests.rows, name);
  }
});

test('the coefficient-shape gate rejects a missing harmonic row', () => {
  const source = sources.wmm2025.coefficient;
  const lines = readFileSync(join(root, source.output), 'utf8').split(/\r?\n/);
  lines.splice(1, 1);
  assert.throws(() => validateCof(Buffer.from(lines.join('\n')), source.degree, 'bad WMM'), /89 coefficients, expected 90/);
});
