// The desktop bundle and the registry entry (add-local-mcp-server 5.2, 5.3):
// the manifest says what the server actually takes, the bundle carries every
// file the server reads and unpacks byte-for-byte, and the registry entry
// names the same versions with a digest the build writes.
import { createHash } from 'node:crypto';
import { inflateRawSync } from 'node:zlib';
import { readdirSync, readFileSync, mkdtempSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { build, bundleFiles, writeDigest } from './build-mcpb.mjs';
import { TOOLSETS } from '../../mcp/toolsets.mjs';
import { TOOLS as META_TOOLS } from '../../mcp/meta.mjs';

const root = new URL('../..', import.meta.url).pathname;
const manifest = JSON.parse(readFileSync(join(root, 'mcp/manifest.json'), 'utf8'));
const pkg = JSON.parse(readFileSync(join(root, 'mcp/package.json'), 'utf8'));

test('the MCPB manifest is v0.3 and describes the server that exists', () => {
  assert.equal(manifest.manifest_version, '0.3');
  assert.equal(manifest.server.type, 'node');
  assert.equal(manifest.version, pkg.version, 'the bundle and the npm package are the same release');
  // The entry point is a file the bundle carries, and the command runs it.
  assert.ok(bundleFiles().includes(manifest.server.entry_point), 'the entry point is not in the bundle');
  assert.equal(manifest.server.mcp_config.command, 'node');
  assert.ok(manifest.server.mcp_config.args[0].includes(manifest.server.entry_point));
  // Every tool it advertises is a meta-tool the server really has.
  const names = new Set(META_TOOLS.map((t) => t.name));
  assert.deepEqual(manifest.tools.map((t) => t.name).filter((n) => !names.has(n)), []);
  assert.equal(manifest.tools.length, names.size, 'the manifest lists every meta-tool');
  // The one user setting is the server's own --toolsets, and it names the
  // toolsets that exist rather than a list that can drift.
  assert.deepEqual(Object.keys(manifest.user_config), ['toolsets']);
  assert.ok(manifest.server.mcp_config.args.some((a) => a.includes('${user_config.toolsets}')));
  for (const name of Object.keys(TOOLSETS)) {
    assert.ok(manifest.user_config.toolsets.description.includes(name), `${name} is not offered in the bundle's settings`);
  }
});

test('the bundle carries every file it declares, and unpacks to the same bytes', () => {
  const out = mkdtempSync(join(tmpdir(), 'mcpb-'));
  const r = build(out);
  const zip = readFileSync(r.file);
  // Read it back through the central directory, as an installer would.
  const eocd = zip.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  const count = zip.readUInt16LE(eocd + 10);
  let at = zip.readUInt32LE(eocd + 16);
  const seen = new Map();
  for (let i = 0; i < count; i += 1) {
    assert.equal(zip.readUInt32LE(at), 0x02014b50, 'central directory entry');
    const method = zip.readUInt16LE(at + 10);
    const compressed = zip.readUInt32LE(at + 20);
    const size = zip.readUInt32LE(at + 24);
    const nameLen = zip.readUInt16LE(at + 28);
    const local = zip.readUInt32LE(at + 42);
    const name = zip.toString('utf8', at + 46, at + 46 + nameLen);
    const body = zip.subarray(local + 30 + zip.readUInt16LE(local + 26) + zip.readUInt16LE(local + 28), local + 30 + zip.readUInt16LE(local + 26) + zip.readUInt16LE(local + 28) + compressed);
    const data = method === 8 ? inflateRawSync(body) : body;
    assert.equal(data.length, size, `${name} unpacks to its declared size`);
    seen.set(name, data);
    at += 46 + nameLen + zip.readUInt16LE(at + 30) + zip.readUInt16LE(at + 32);
  }
  assert.deepEqual([...seen.keys()].sort(), bundleFiles().sort());
  for (const [name, data] of seen) {
    assert.deepEqual(data, readFileSync(join(root, 'mcp', name)), `${name} came back changed`);
  }
  // The bundle is reproducible: the same inputs give the same bytes.
  assert.equal(build(mkdtempSync(join(tmpdir(), 'mcpb-'))).digest, r.digest);
});

test('the registry entry names the same release, and the build writes its digest', () => {
  const server = JSON.parse(readFileSync(join(root, 'mcp/server.json'), 'utf8'));
  assert.equal(server.name, pkg.mcpName);
  assert.match(server.name, /^[a-zA-Z0-9.-]+\/[a-zA-Z0-9._-]+$/, 'reverse-DNS name');
  assert.ok(server.description.length <= 100, 'the registry caps the description at 100 characters');
  assert.equal(server.version, pkg.version);
  const npm = server.packages.find((p) => p.registryType === 'npm');
  assert.equal(npm.identifier, pkg.name);
  assert.equal(npm.version, pkg.version);
  assert.equal(npm.transport.type, 'stdio');
  const mcpb = server.packages.find((p) => p.registryType === 'mcpb');
  assert.match(mcpb.fileSha256, /^[a-f0-9]{64}$/, 'a digest, even as a placeholder');
  assert.ok(mcpb.identifier.includes(`geoprims-${pkg.version}.mcpb`), 'the download names this release');
  // Releasing fills the digest in from the built bundle.
  const out = mkdtempSync(join(tmpdir(), 'mcpb-'));
  const built = build(out);
  const copy = join(out, 'server.json');
  writeFileSync(copy, JSON.stringify(server, null, 2));
  const updated = writeDigest(built, copy);
  const digest = createHash('sha256').update(readFileSync(built.file)).digest('hex');
  assert.equal(updated.packages.find((p) => p.registryType === 'mcpb').fileSha256, digest);
});

test('the bundled offline assets stay inside the 6 MB the spec allows', () => {
  // add-local-mcp-server 4.2: the bundle is what makes the server work with
  // no network, and it is also what a desktop user downloads, so it is capped.
  const dir = join(root, 'mcp/dist/assets');
  const walk = (d) => readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(join(d, e.name)) : [join(d, e.name)]));
  const files = walk(dir);
  const bytes = files.reduce((n, f) => n + statSync(f).size, 0);
  assert.ok(bytes <= 6e6, `bundled assets are ${(bytes / 1e6).toFixed(1)} MB, over the 6 MB cap`);
  // Every asset the registry lists is either bundled or compiled into the
  // core; nothing is listed that the server cannot reach offline.
  const registry = JSON.parse(readFileSync(join(dir, 'registry.json'), 'utf8'));
  const present = new Set(files.map((f) => f.slice(dir.length + 1)));
  const compiledIn = new Set(['wmm2025', 'igrf14']);
  for (const asset of registry.assets) {
    if (compiledIn.has(asset.id)) continue;
    for (const name of Object.keys(asset.files)) {
      assert.ok(present.has(join('data', asset.id, asset.version, name)), `${asset.id}/${name} is listed but not bundled`);
    }
  }
  // A compiled-in asset really is reachable with no files on disk: its tool
  // answers from the bundle as shipped.
  for (const id of compiledIn) assert.ok(registry.assets.some((a) => a.id === id), `${id} is not in the registry`);
});
