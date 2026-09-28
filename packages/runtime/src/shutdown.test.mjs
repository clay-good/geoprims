import { test } from 'node:test';
import assert from 'node:assert/strict';
import { workerHost } from './worker-host.mjs';

test('closing the host cancels running and queued calls and stops progress', async () => {
  const host = workerHost('/unused', { timeoutMs: 150 });
  const controller = new AbortController();
  let updates = 0;
  const running = host._callWithOptions('spin', [], {
    signal: controller.signal,
    onProgress: () => updates++,
  });
  const queued = host._call('spin');
  const closing = host.close();
  try {
    const outcome = await Promise.race([
      Promise.all([running, queued]),
      new Promise((resolve) => setTimeout(() => resolve('unsettled'), 100)),
    ]);
    assert.deepEqual(outcome, [null, null]);
    await closing;
    controller.abort();
    await new Promise((resolve) => setTimeout(resolve, 300));
    assert.equal(updates, 0);
    assert.equal(await host.invoke('units.speed.kt-to-mph', '{"value":1}'), null);
    assert.equal(await host.search('{}'), null);
    assert.equal(host.close(), closing, 'repeated close shares the termination promise');
  } finally {
    await host.close();
  }
});

test('closing an idle host also refuses subsequent calls', async () => {
  const host = workerHost('/unused');
  await host.close();
  assert.equal(await host.invokeBatch('units.speed.kt-to-mph', '[]'), null);
  assert.equal(await host.searchLoad('[]'), null);
});
