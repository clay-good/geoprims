// MCP "Network audit": the server runs every tool's worked example inside an
// OS sandbox with no network, every one succeeds, and nothing tries to
// connect. macOS uses sandbox-exec; Linux uses an empty network namespace.
import { spawn, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const here = new URL('.', import.meta.url).pathname;
const root = join(here, '..');
const server = join(here, 'server.mjs');
const hook = join(here, 'test', 'no-network.mjs');
const node = [process.execPath, '--import', hook, server];

function sandbox() {
  if (process.platform === 'darwin' && !spawnSync('sandbox-exec', ['-p', '(version 1)(allow default)', 'true']).error) {
    return ['sandbox-exec', ['-p', '(version 1)(allow default)(deny network*)', ...node]];
  }
  if (process.platform === 'linux' && spawnSync('unshare', ['-rn', 'true']).status === 0) return ['unshare', ['-rn', ...node]];
  return null;
}
const wrap = sandbox();

test('every tool runs its worked example with no network and no connection attempts', { skip: wrap ? false : 'no network sandbox on this machine', timeout: 300_000 }, async () => {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const proc = spawn(wrap[0], wrap[1], { stdio: ['pipe', 'pipe', 'pipe'] });
  let stderr = '';
  proc.stderr.on('data', (d) => (stderr += d));
  const waiting = new Map();
  let buf = '';
  proc.stdout.setEncoding('utf8');
  proc.stdout.on('data', (d) => {
    buf += d;
    for (let nl; (nl = buf.indexOf('\n')) >= 0; buf = buf.slice(nl + 1)) {
      const msg = JSON.parse(buf.slice(0, nl));
      waiting.get(msg.id)?.(msg);
    }
  });
  let seq = 0;
  const request = (method, params) =>
    new Promise((resolve) => {
      const id = ++seq;
      waiting.set(id, resolve);
      proc.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    });
  try {
    await request('initialize', { protocolVersion: '2025-11-25', capabilities: {} });
    const failed = [];
    for (const t of catalog.tools) {
      const r = await request('tools/call', { name: 'geoprims_run', arguments: { id: t.id } });
      const body = r.result?.structuredContent;
      if (!body?.ok) failed.push(`${t.id}: ${JSON.stringify(body?.error ?? r.error)}`);
    }
    assert.deepEqual(failed, []);
    assert.equal(catalog.tools.length > 150, true);
  } finally {
    proc.stdin.end();
  }
  await new Promise((resolve) => proc.on('close', resolve));
  assert.ok(!stderr.includes('NETWORK ATTEMPT'), stderr.split('\n').filter((l) => l.includes('NETWORK')).join('\n'));
});

test('the sandbox really blocks the network', { skip: wrap ? false : 'no network sandbox on this machine' }, () => {
  const probe = "require('net').connect(443, '1.1.1.1').on('connect', () => { console.log('CONNECTED'); process.exit(0); }).on('error', (e) => { console.log('blocked', e.code); })";
  const args = [...wrap[1].slice(0, wrap[1].indexOf(process.execPath)), process.execPath, '-e', probe];
  const r = spawnSync(wrap[0], args, { encoding: 'utf8', timeout: 20_000 });
  assert.match(r.stdout, /^blocked/);
});
