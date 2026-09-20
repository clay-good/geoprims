#!/usr/bin/env node
// Byte-level manifest for two independent CI builds of the same commit.
// Include every shipped core, MCP, and website file, not just Wasm modules.
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;

export function artifactDigests(base = root, dirs = ['dist', 'mcp/dist', 'apps/web/dist']) {
  const files = [];
  function visit(rel) {
    for (const entry of readdirSync(join(base, rel), { withFileTypes: true })) {
      const path = `${rel}/${entry.name}`;
      if (entry.isDirectory()) visit(path);
      else if (entry.isFile()) files.push(path);
      else throw new Error(`unsupported artifact: ${path}`);
    }
  }
  for (const dir of dirs) visit(dir);
  return files.sort().map((path) => `${createHash('sha256').update(readFileSync(join(base, path))).digest('hex')}  ${path}`).join('\n') + '\n';
}

if (import.meta.url === `file://${process.argv[1]}`) process.stdout.write(artifactDigests());
