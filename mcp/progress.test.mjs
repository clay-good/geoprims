// Stdio progress and cancellation must work while a long tools/call is still
// waiting for the same worker. A client that cancels receives no late answer.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { test } from 'node:test';

test('a progress-aware MCP call can be canceled and the server keeps serving', { timeout: 10_000 }, async () => {
  const server = new URL('./server.mjs', import.meta.url).pathname;
  const proc = spawn(process.execPath, [server], { stdio: ['pipe', 'pipe', 'pipe'] });
  const messages = [];
  const waiters = [];
  let text = '';
  let stderr = '';
  proc.stderr.on('data', (part) => { stderr += part; });
  proc.stdout.setEncoding('utf8');
  proc.stdout.on('data', (part) => {
    text += part;
    for (let end; (end = text.indexOf('\n')) >= 0; text = text.slice(end + 1)) {
      const msg = JSON.parse(text.slice(0, end));
      messages.push(msg);
      for (const waiter of [...waiters]) {
        if (!waiter.matches(msg)) continue;
        clearTimeout(waiter.timer);
        waiters.splice(waiters.indexOf(waiter), 1);
        waiter.resolve(msg);
      }
    }
  });
  const send = (msg) => proc.stdin.write(JSON.stringify({ jsonrpc: '2.0', ...msg }) + '\n');
  const until = (matches) => {
    const found = messages.find(matches);
    if (found) return Promise.resolve(found);
    return new Promise((resolve, reject) => {
      const waiter = { matches, resolve, timer: setTimeout(() => reject(new Error(`No matching MCP message: ${stderr}`)), 5_000) };
      waiters.push(waiter);
    });
  };
  try {
    const points = [{ lat: 40.0, lon: -80.35 }, { lat: 40.0, lon: -79.55 }, { lat: 40.5, lon: -79.55 }, { lat: 40.5, lon: -80.35 }];
    send({ id: 1, method: 'tools/call', params: {
      name: 'geoprims_run',
      arguments: { id: 'indexing.h3.polygon-to-cells', args: { points, resolution: 10 } },
      _meta: { progressToken: 'polyfill-1' },
    } });
    const progress = await until((m) => m.method === 'notifications/progress');
    assert.equal(progress.params.progressToken, 'polyfill-1');
    assert.ok(progress.params.progress >= 200);
    const started = performance.now();
    send({ method: 'notifications/cancelled', params: { requestId: 1, reason: 'new inputs' } });
    send({ id: 2, method: 'tools/call', params: { name: 'geoprims_run', arguments: { id: 'units.speed.kt-to-mph', args: { value: 100 } } } });
    const next = await until((m) => m.id === 2);
    assert.equal(next.result.structuredContent.result.converted.value, 115.07794480235425);
    assert.ok(performance.now() - started < 1_000, 'the canceled call blocked the next request');
    await new Promise((resolve) => setTimeout(resolve, 300));
    assert.equal(messages.some((m) => m.id === 1), false, 'the canceled call sent a late response');

    // Both lines may arrive in one stdin chunk, before the queued call starts.
    const queued = { jsonrpc: '2.0', id: 3, method: 'tools/call', params: {
      name: 'geoprims_run', arguments: { id: 'indexing.h3.polygon-to-cells', args: { points, resolution: 10 } },
    } };
    const cancel = { jsonrpc: '2.0', method: 'notifications/cancelled', params: { requestId: 3 } };
    proc.stdin.write(JSON.stringify(queued) + '\n' + JSON.stringify(cancel) + '\n');
    send({ id: 4, method: 'tools/call', params: { name: 'geoprims_run', arguments: { id: 'units.speed.kt-to-mph', args: { value: 1 } } } });
    assert.equal((await until((m) => m.id === 4)).result.structuredContent.ok, true);
    assert.equal(messages.some((m) => m.id === 3), false, 'a queued cancellation sent a response');
  } finally {
    for (const waiter of waiters) clearTimeout(waiter.timer);
    proc.kill();
  }
});
