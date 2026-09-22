// Build gate (web/tool-docs 9.1): every worked example that is a golden vector
// must still agree with its tool, within the vector's tolerance, or the build
// fails naming the tool and the value.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';
import { disagreements, vectorFor } from '../src/lib/worked.mjs';

const root = fileURLToPath(new URL('../../..', import.meta.url));
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
let matched = 0;
const bad = [];
for (const t of catalog.tools) {
  const ex = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  const vectors = readFileSync(join(root, t.vectors), 'utf8').trim().split('\n').map((l) => JSON.parse(l));
  const v = vectorFor(ex, vectors);
  if (!v) continue;
  matched += 1;
  const r = JSON.parse(await host.invoke(t.id, JSON.stringify(v.input)));
  for (const d of disagreements(r, v)) bad.push(`${t.id} (${v.id}): ${d}`);
}
if (bad.length) {
  console.error(`examples: ${bad.length} worked examples disagree with their tools:\n  ${bad.join('\n  ')}`);
  process.exit(1);
}
console.log(`examples: ${matched} of ${catalog.tools.length} worked examples are golden vectors, and every one agrees with its tool`);
