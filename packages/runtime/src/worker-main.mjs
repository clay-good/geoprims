// The worker side of worker-host.mjs.
import { parentPort, workerData } from 'node:worker_threads';
import { nodeHost } from './node.mjs';

const host = nodeHost(workerData.wasmDir, { maxBytes: workerData.maxBytes });

const methods = {
  invoke: (id, input) => host.invoke(id, input),
  invokeBatch: (id, inputs) => host.invokeBatch(id, inputs),
  searchLoad: async (index) => (await host.module('search')).callString('gp_search_load', index),
  search: async (request) => (await host.module('search')).callString('gp_search', request),
  // Test hook: a call that never returns, to exercise the timeout.
  spin: () => {
    for (;;);
  },
};

// Messages are handled strictly in order, even though methods are async.
let chain = Promise.resolve();
parentPort.on('message', ({ id, method, args }) => {
  chain = chain.then(async () => {
    const out = await methods[method](...args);
    if (id >= 0) parentPort.postMessage({ id, out });
  });
});
