#!/usr/bin/env node
// geoprims local MCP server: zero dependencies, stdio only, no network.
//
//   node mcp/server.mjs [--timeout=<ms>] [--debug]
//
// It runs the same Wasm modules as geoprims.com through packages/runtime.
// Logs go to stderr only and never include argument values unless --debug.
import { existsSync, readFileSync, realpathSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TOOLS, metaHandlers, schemaCheck } from './meta.mjs';

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
    else if (k === '--toolsets' || k === '--no-meta') {
      throw new Error(`${k} is not available in this build: no stable toolsets exist yet.`);
    } else {
      throw new Error(`Unknown option ${a}. Options: --timeout=<ms>, --debug, --allow-asset-download.`);
    }
  }
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
  const handlers = metaHandlers({ host, catalog });
  const byName = new Map(TOOLS.map((t) => [t.name, t]));
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));

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
    'tools/list': () => ({ tools: TOOLS, ...LIST_CACHE }),
    'tools/call': async (p) => {
      const tool = byName.get(p?.name);
      if (!tool) throw rpcError(-32602, `Unknown tool ${p?.name}. Tools: ${TOOLS.map((t) => t.name).join(', ')}`);
      const args = p.arguments ?? {};
      const bad = schemaCheck(tool, args);
      if (bad) return toolResult({ ok: false, error: { code: 'INVALID_INPUT', message: `${tool.name}: ${bad}.` } });
      return toolResult(await handlers[tool.name](args));
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
    'prompts/list': () => ({ prompts: [], ...LIST_CACHE }),
  };

  async function handle(msg) {
    if (msg === null || typeof msg !== 'object' || Array.isArray(msg) || msg.jsonrpc !== '2.0' || typeof msg.method !== 'string') {
      return { jsonrpc: '2.0', id: msg?.id ?? null, error: { code: -32600, message: 'Invalid request' } };
    }
    const isNotification = !('id' in msg);
    const fn = methods[msg.method];
    if (isNotification) return null;
    if (!fn) return { jsonrpc: '2.0', id: msg.id, error: { code: -32601, message: `Method not found: ${msg.method}` } };
    try {
      return { jsonrpc: '2.0', id: msg.id, result: await fn(msg.params) };
    } catch (e) {
      if (e.rpc) return { jsonrpc: '2.0', id: msg.id, error: e.rpc };
      log(opts, `internal error in ${msg.method}: ${e.message}`);
      return { jsonrpc: '2.0', id: msg.id, error: { code: -32603, message: 'Internal error' } };
    }
  }

  return { handle, close: () => host.close() };
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
  const server = await createServer(opts);
  if (!server) {
    process.stderr.write('Built files missing: check out a release tag (git checkout vX.Y.Z) or run npm run build (requires Rust)\n');
    process.exit(1);
  }
  let buf = '';
  let chain = Promise.resolve();
  const send = (obj) => obj && process.stdout.write(JSON.stringify(obj) + '\n');
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk) => {
    buf += chunk;
    let nl;
    while ((nl = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      if (!line) continue;
      chain = chain.then(async () => {
        if (Buffer.byteLength(line) > MAX_MESSAGE_BYTES) {
          return send({ jsonrpc: '2.0', id: null, error: { code: -32600, message: `Message over ${MAX_MESSAGE_BYTES} bytes` } });
        }
        let msg;
        try {
          msg = JSON.parse(line);
        } catch {
          return send({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } });
        }
        if (opts.debug) log(opts, `<- ${line}`);
        else log(opts, `<- ${msg.method ?? 'response'}${msg.params?.name ? ` ${msg.params.name}` : ''}`);
        send(await server.handle(msg));
      });
    }
    if (Buffer.byteLength(buf) > MAX_MESSAGE_BYTES) {
      buf = '';
      send({ jsonrpc: '2.0', id: null, error: { code: -32600, message: `Message over ${MAX_MESSAGE_BYTES} bytes` } });
    }
  });
  process.stdin.on('end', () => chain.then(() => server.close()).then(() => process.exit(0)));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === realpathSync(process.argv[1])) main();
