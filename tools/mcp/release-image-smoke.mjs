#!/usr/bin/env node
// Runs inside the Node-only release image with networking disabled.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { verifyRelease } from '../../mcp/verify.mjs';

assert.equal(verifyRelease(), 12);
const child = spawn(process.execPath, ['mcp/server.mjs'], { stdio: ['pipe', 'pipe', 'inherit'] });
const replies = [];
child.stdout.setEncoding('utf8');
child.stdout.on('data', (chunk) => {
  for (const line of chunk.trim().split('\n').filter(Boolean)) replies.push(JSON.parse(line));
});
child.stdin.end([
  { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {} } },
  { jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} },
].map(JSON.stringify).join('\n') + '\n');
const code = await new Promise((resolve) => child.on('exit', resolve));
assert.equal(code, 0);
assert.equal(replies.find((reply) => reply.id === 1).result.serverInfo.name, 'geoprims');
assert.equal(replies.find((reply) => reply.id === 2).result.tools.length, 6);
console.log('Node-only clone smoke: verified payload and listed 6 meta-tools');
