#!/usr/bin/env node
// Verify the prebuilt release payload before the MCP server loads any Wasm.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

export function verifyRelease(root = here) {
  const wasm = join(root, 'dist/wasm');
  const manifest = JSON.parse(readFileSync(join(wasm, 'modules.json'), 'utf8'));
  for (const module of manifest.modules) {
    const bytes = readFileSync(join(wasm, `${module.module}.wasm`));
    const actual = createHash('sha256').update(bytes).digest('hex');
    if (actual !== module.sha256) throw new Error(`${module.module}.wasm: expected ${module.sha256}, got ${actual}`);
  }
  JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  return manifest.modules.length;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    console.log(`verified ${verifyRelease()} Wasm modules`);
  } catch (error) {
    console.error(`release verification failed: ${error.message}`);
    process.exitCode = 1;
  }
}
