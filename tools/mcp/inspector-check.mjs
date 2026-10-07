#!/usr/bin/env node
// Exercises the server through the MCP Inspector CLI (agent/mcp-server
// "Golden surface file"): what a real client lists must be the committed
// mcp/surface.json, and a call must answer. Any difference fails with the
// lines that differ, so surface drift shows in CI as a diff.
//
// Usage: node tools/mcp/inspector-check.mjs   (after build:mcp; needs network
// once, for npx to fetch the Inspector version pinned in tools/toolchain.json)
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const version = JSON.parse(readFileSync(join(root, 'tools/toolchain.json'), 'utf8')).mcpInspector;
const server = join(root, 'mcp/server.mjs');

/** One Inspector CLI request against the server, parsed. */
export function inspect(method, extra = []) {
  const out = execFileSync('npx', ['-y', `@modelcontextprotocol/inspector@${version}`, '--cli', 'node', server, '--method', method, ...extra], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
    maxBuffer: 64 * 1024 * 1024,
  });
  return JSON.parse(out);
}

/** The value with every object's keys in order: a client may reorder keys, which changes nothing. */
export const canonical = (v) =>
  Array.isArray(v) ? v.map(canonical) : v && typeof v === 'object' ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canonical(v[k])])) : v;

/** The lines of `b` that differ from `a` (keys in order), as an LCS line diff with a few lines of context. */
export function lineDiff(a, b, context = 3) {
  const [x, y] = [JSON.stringify(canonical(a), null, 2).split('\n'), JSON.stringify(canonical(b), null, 2).split('\n')];
  // Longest common subsequence table, from the end, so the walk below goes forward.
  const L = Array.from({ length: x.length + 1 }, () => new Uint32Array(y.length + 1));
  for (let i = x.length - 1; i >= 0; i--) {
    for (let j = y.length - 1; j >= 0; j--) L[i][j] = x[i] === y[j] ? L[i + 1][j + 1] + 1 : Math.max(L[i + 1][j], L[i][j + 1]);
  }
  const ops = [];
  let [i, j] = [0, 0];
  while (i < x.length || j < y.length) {
    if (i < x.length && j < y.length && x[i] === y[j]) ops.push([' ', x[i++], j++]);
    else if (j < y.length && (i === x.length || L[i][j + 1] > L[i + 1][j])) ops.push(['+', y[j], j++]);
    else ops.push(['-', x[i++], j]);
  }
  const changed = ops.map((o, k) => (o[0] !== ' ' ? k : -1)).filter((k) => k >= 0);
  if (!changed.length) return '';
  const keep = new Set(changed.flatMap((k) => Array.from({ length: 2 * context + 1 }, (_, d) => k - context + d)));
  const out = [];
  let last = -2;
  ops.forEach((o, k) => {
    if (!keep.has(k)) return;
    if (k !== last + 1) out.push(`@@ line ${o[2] + 1}`);
    out.push(`${o[0]} ${o[1]}`);
    last = k;
  });
  return out.join('\n');
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const golden = JSON.parse(readFileSync(join(root, 'mcp/surface.json'), 'utf8'));
  const seen = {
    tools: inspect('tools/list').tools,
    resources: inspect('resources/list').resources,
    resourceTemplates: inspect('resources/templates/list').resourceTemplates,
    prompts: inspect('prompts/list').prompts,
  };
  let failed = false;
  for (const key of Object.keys(golden)) {
    const diff = lineDiff(golden[key], seen[key]);
    if (diff) {
      failed = true;
      console.error(`mcp/surface.json ${key} differs from what the Inspector lists (- golden, + server):\n${diff}\n`);
    }
  }
  // A call through a real client answers with the core's result.
  const call = inspect('tools/call', ['--tool-name', 'geoprims_run', '--tool-arg', 'id=units.speed.kt-to-mph', '--tool-arg', 'args={"value":100}']);
  const summary = call.structuredContent?.summary;
  if (summary !== '100 kt is 115.07794 mph.') {
    failed = true;
    console.error(`geoprims_run through the Inspector answered ${JSON.stringify(call).slice(0, 400)}`);
  }
  console.log(`MCP Inspector ${version}: ${seen.tools.length} tools, ${seen.resources.length} resources, ${seen.resourceTemplates.length} templates, ${seen.prompts.length} prompts; ${failed ? 'DIFFERS from' : 'matches'} mcp/surface.json`);
  process.exit(failed ? 1 : 0);
}
