#!/usr/bin/env node
// geoprims local MCP server: zero dependencies, stdio only, no network.
//
//   node mcp/server.mjs [--toolsets=<name,...>] [--no-meta] [--timeout=<ms>] [--debug]
//
// It runs the same Wasm modules as geoprims.com through packages/runtime.
// Logs go to stderr only and never include argument values unless --debug.
import { existsSync, readFileSync, realpathSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ANNOTATIONS, TOOLS, metaHandlers, schemaCheck } from './meta.mjs';
import { getPrompt, promptList } from './prompts.mjs';
import { directTools, parseToolsets } from './toolsets.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const SERVER_VERSION = JSON.parse(readFileSync(join(here, 'package.json'), 'utf8')).version;
export const PROTOCOL_VERSIONS = ['2026-07-28', '2025-11-25', '2025-06-18'];
const MAX_MESSAGE_BYTES = 10_000_000;
const INSTRUCTIONS =
  'geoprims runs exact, cited geospatial, aviation, drone, survey, and unit calculations locally. Search for a tool, describe it, then run it. Results are planning aids, not certified for navigation; relay warnings to the user.';

function parseArgs(argv) {
  const opts = { timeoutMs: 10_000, debug: false };
  for (const a of argv) {
    const [k, v] = a.split('=');
    if (k === '--timeout' && /^\d+$/.test(v ?? '')) opts.timeoutMs = Number(v);
    else if (k === '--debug') opts.debug = true;
    else if (k === '--allow-asset-download') opts.allowAssetDownload = true; // no downloadable assets exist yet
    else if (k === '--toolsets') opts.toolsets = parseToolsets(v);
    else if (k === '--no-meta') opts.noMeta = true;
    else {
      throw new Error(`Unknown option ${a}. Options: --toolsets=<name,...>, --no-meta, --timeout=<ms>, --debug, --allow-asset-download.`);
    }
  }
  if (opts.noMeta && !opts.toolsets) throw new Error('--no-meta needs --toolsets, or no tools would be listed.');
  return opts;
}

/** Finds built files: mcp/dist (release tags and npm) or the repo's dist/ (a local build). */
export function findDist() {
  for (const dir of [join(here, 'dist'), join(here, '..', 'dist')]) {
    if (existsSync(join(dir, 'wasm', 'base.wasm')) && existsSync(join(dir, 'catalog', 'v1.json'))) return dir;
  }
  return null;
}

async function loadRuntime() {
  const bundled = join(here, 'dist', 'runtime', 'worker-host.mjs');
  const source = join(here, '..', 'packages', 'runtime', 'src', 'worker-host.mjs');
  return import(existsSync(bundled) ? bundled : source);
}

export async function createServer(opts = {}) {
  const dist = findDist();
  if (!dist) return null;
  const catalog = JSON.parse(readFileSync(join(dist, 'catalog', 'v1.json'), 'utf8'));
  const { workerHost } = await loadRuntime();
  const host = workerHost(join(dist, 'wasm'), { timeoutMs: opts.timeoutMs ?? 10_000, maxBytes: MAX_MESSAGE_BYTES });
  await host.searchLoad(JSON.stringify(catalog.tools));
  const modules = JSON.parse(readFileSync(join(dist, 'wasm', 'modules.json'), 'utf8')).modules;
  const limits = JSON.parse(readFileSync(findData(dist, 'report-limits.json'), 'utf8'));
  const handlers = metaHandlers({ host, catalog, modules, limits });
  const direct = opts.toolsets ? directTools(catalog, opts.toolsets, ANNOTATIONS) : [];
  if (opts.toolsets && !direct.length) log(opts, `toolsets ${opts.toolsets.join(', ')} have no stable tools yet`);
  const listed = [...(opts.noMeta ? [] : TOOLS), ...direct.map(({ id, ...t }) => t)];
  const byName = new Map(opts.noMeta ? [] : TOOLS.map((t) => [t.name, t]));
  const directByName = new Map(direct.map((t) => [t.name, t.id]));
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  const active = new Map();
  const queued = new Set();
  const canceledQueued = new Set();

  const capabilities = { tools: { listChanged: false }, resources: { listChanged: false }, prompts: { listChanged: false } };
  const serverInfo = { name: 'geoprims', title: 'geoprims', version: SERVER_VERSION };
  const LIST_CACHE = { ttlMs: 3_600_000, cacheScope: 'public' };

  const toolResult = (body) => ({
    content: [{ type: 'text', text: JSON.stringify(body) }],
    structuredContent: body,
    ...(body.ok === false ? { isError: true } : {}),
  });

  const resourceFor = (id) => {
    const m = byId.get(id);
    return m && { ...m, vectors: vectorsFor(dist, m) };
  };

  const methods = {
    initialize: (p) => ({
      protocolVersion: PROTOCOL_VERSIONS.includes(p?.protocolVersion) ? p.protocolVersion : PROTOCOL_VERSIONS[1],
      capabilities,
      serverInfo,
      instructions: INSTRUCTIONS,
    }),
    'server/discover': () => ({ supportedVersions: PROTOCOL_VERSIONS, capabilities, serverInfo, instructions: INSTRUCTIONS }),
    ping: () => ({}),
    'tools/list': () => ({ tools: listed, ...LIST_CACHE }),
    'tools/call': async (p, context) => {
      // A direct tool is geoprims_run with its id; the core validates the arguments.
      const directId = directByName.get(p?.name);
      if (directId) {
        const body = await handlers.geoprims_run({ id: directId, args: p.arguments ?? {} }, context);
        return body === null ? null : toolResult(body);
      }
      const tool = byName.get(p?.name);
      if (!tool) throw rpcError(-32602, `Unknown tool ${p?.name}. Tools: ${listed.map((t) => t.name).join(', ')}`);
      const args = p.arguments ?? {};
      const bad = schemaCheck(tool, args);
      if (bad) return toolResult({ ok: false, error: { code: 'INVALID_INPUT', message: `${tool.name}: ${bad}.` } });
      const body = await handlers[tool.name](args, context);
      return body === null ? null : toolResult(body);
    },
    'resources/list': () => ({
      resources: [
        { uri: 'geoprims://catalog', name: 'catalog', title: 'geoprims catalog', description: 'Every tool id with title, summary, domain, and stability.', mimeType: 'application/json' },
      ],
      ...LIST_CACHE,
    }),
    'resources/templates/list': () => ({
      resourceTemplates: [
        { uriTemplate: 'geoprims://tool/{id}', name: 'tool', title: 'geoprims tool manifest', description: 'Full manifest with accuracy, references, worked example, and golden vectors.', mimeType: 'application/json' },
      ],
      ...LIST_CACHE,
    }),
    'resources/read': (p) => {
      const uri = p?.uri ?? '';
      let body;
      if (uri === 'geoprims://catalog') {
        body = {
          coreVersion: catalog.coreVersion,
          counts: catalog.counts,
          tools: catalog.tools.map(({ id, title, summary, domain, stability }) => ({ id, title, summary, domain, stability })),
        };
      } else if (uri.startsWith('geoprims://tool/')) {
        body = resourceFor(uri.slice('geoprims://tool/'.length));
      }
      if (!body) throw rpcError(-32002, `Resource not found: ${uri}`);
      return { contents: [{ uri, mimeType: 'application/json', text: JSON.stringify(body) }] };
    },
    'prompts/list': () => ({ prompts: promptList(), ...LIST_CACHE }),
    'prompts/get': (p) => {
      const out = getPrompt(p?.name, p?.arguments ?? {});
      if (out.error) throw rpcError(-32602, out.error);
      return { description: out.description, messages: out.messages };
    },
  };

  async function handle(msg) {
    if (msg === null || typeof msg !== 'object' || Array.isArray(msg) || msg.jsonrpc !== '2.0' || typeof msg.method !== 'string') {
      return { jsonrpc: '2.0', id: msg?.id ?? null, error: { code: -32600, message: 'Invalid request' } };
    }
    const isNotification = !('id' in msg);
    const fn = methods[msg.method];
    if (isNotification) {
      if (msg.method === 'notifications/cancelled') {
        const id = msg.params?.requestId;
        if (active.has(id)) active.get(id).abort();
        else if (queued.has(id)) canceledQueued.add(id);
      }
      return null;
    }
    queued.delete(msg.id);
    if (canceledQueued.delete(msg.id)) return null;
    if (!fn) return { jsonrpc: '2.0', id: msg.id, error: { code: -32601, message: `Method not found: ${msg.method}` } };
    const controller = msg.method === 'tools/call' ? new AbortController() : null;
    if (controller) active.set(msg.id, controller);
    const token = msg.params?._meta?.progressToken;
    const validToken = typeof token === 'string' || (typeof token === 'number' && Number.isFinite(token));
    const onProgress = controller && opts.emit && validToken
      ? (elapsed) => {
        if (!controller.signal.aborted) opts.emit({ jsonrpc: '2.0', method: 'notifications/progress', params: { progressToken: token, progress: Math.round(elapsed), message: 'Calculating' } });
      }
      : undefined;
    try {
      const result = await fn(msg.params, { signal: controller?.signal, onProgress });
      return controller?.signal.aborted || result === null ? null : { jsonrpc: '2.0', id: msg.id, result };
    } catch (e) {
      if (controller?.signal.aborted) return null;
      if (e.rpc) return { jsonrpc: '2.0', id: msg.id, error: e.rpc };
      log(opts, `internal error in ${msg.method}: ${e.message}`);
      return { jsonrpc: '2.0', id: msg.id, error: { code: -32603, message: 'Internal error' } };
    } finally {
      if (active.get(msg.id) === controller) active.delete(msg.id);
    }
  }

  return { handle, reserve: (id) => queued.add(id), close: () => host.close() };
}

/** Shared data files: bundled in mcp/dist/data, or read from the repo's data/. */
function findData(dist, name) {
  const bundled = join(dist, 'data', name);
  return existsSync(bundled) ? bundled : join(here, '..', 'data', name);
}

function vectorsFor(dist, m) {
  for (const f of [join(dist, 'vectors', basename(m.vectors)), join(dist, '..', m.vectors)]) {
    if (existsSync(f)) return readFileSync(f, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
  }
  return [];
}

function rpcError(code, message) {
  const e = new Error(message);
  e.rpc = { code, message };
  return e;
}

function log(opts, text) {
  process.stderr.write(`geoprims-mcp: ${text}\n`);
}

/** Newline-delimited JSON-RPC over stdio. */
async function main() {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (e) {
    process.stderr.write(`geoprims-mcp: ${e.message}\n`);
    process.exit(2);
  }
  const send = (obj) => obj && process.stdout.write(JSON.stringify(obj) + '\n');
  const server = await createServer({ ...opts, emit: send });
  if (!server) {
    process.stderr.write('Built files missing: check out a release tag (git checkout vX.Y.Z) or run npm run build (requires Rust)\n');
    process.exit(1);
  }
  let buf = '';
  let chain = Promise.resolve();
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk) => {
    buf += chunk;
    let nl;
    while ((nl = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      if (!line) continue;
      if (Buffer.byteLength(line) > MAX_MESSAGE_BYTES) {
        send({ jsonrpc: '2.0', id: null, error: { code: -32600, message: `Message over ${MAX_MESSAGE_BYTES} bytes` } });
        continue;
      }
      let msg;
      try {
        msg = JSON.parse(line);
      } catch {
        send({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } });
        continue;
      }
      if (opts.debug) log(opts, `<- ${line}`);
      else log(opts, `<- ${msg.method ?? 'response'}${msg.params?.name ? ` ${msg.params.name}` : ''}`);
      // A cancellation notification must get past a call awaiting its worker.
      if (msg.method === 'notifications/cancelled' && !('id' in msg)) server.handle(msg);
      else {
        if (msg.method === 'tools/call' && 'id' in msg) server.reserve(msg.id);
        chain = chain.then(async () => send(await server.handle(msg)));
      }
    }
    if (Buffer.byteLength(buf) > MAX_MESSAGE_BYTES) {
      buf = '';
      send({ jsonrpc: '2.0', id: null, error: { code: -32600, message: `Message over ${MAX_MESSAGE_BYTES} bytes` } });
    }
  });
  process.stdin.on('end', () => chain.then(() => server.close()).then(() => process.exit(0)));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === realpathSync(process.argv[1])) main();
