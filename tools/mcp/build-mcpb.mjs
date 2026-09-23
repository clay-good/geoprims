// Builds the MCPB bundle (add-local-mcp-server 5.2): a zip of the server, its
// manifest, and the dist the server reads, for one-click desktop install.
//
// The zip is written here rather than shelled out to, so the bundle is
// byte-reproducible: entries in a fixed order, a fixed timestamp, and deflate
// from node:zlib. The digest it prints goes in the release notes, so a download
// can be checked against the source it was built from.
import { createHash } from 'node:crypto';
import { deflateRawSync } from 'node:zlib';
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const mcp = join(root, 'mcp');

/** The files a bundle carries: the manifest, the server's own modules, and dist. */
export function bundleFiles() {
  const files = ['manifest.json', 'README.md', 'server.mjs', 'meta.mjs', 'toolsets.mjs', 'prompts.mjs', 'clients.mjs', 'package.json'];
  const walk = (dir) => {
    for (const name of readdirSync(join(mcp, dir)).sort()) {
      const path = join(dir, name);
      if (statSync(join(mcp, path)).isDirectory()) walk(path);
      else files.push(path);
    }
  };
  walk('dist');
  return files.map((p) => p.split(sep).join('/'));
}

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};

/** A deterministic zip: stored order, no timestamps, deflate level default. */
export function zip(entries) {
  const chunks = [];
  const central = [];
  let offset = 0;
  for (const { name, data } of entries) {
    const nameBuf = Buffer.from(name, 'utf8');
    const deflated = deflateRawSync(data);
    const stored = deflated.length < data.length;
    const body = stored ? deflated : data;
    const head = Buffer.alloc(30);
    head.writeUInt32LE(0x04034b50, 0);
    head.writeUInt16LE(20, 4); // version needed
    head.writeUInt16LE(0, 6); // flags
    head.writeUInt16LE(stored ? 8 : 0, 8); // deflate or store
    head.writeUInt16LE(0, 10); // time: fixed, for reproducibility
    head.writeUInt16LE(0x0021, 12); // date: 1980-01-01
    head.writeUInt32LE(crc32(data), 14);
    head.writeUInt32LE(body.length, 18);
    head.writeUInt32LE(data.length, 22);
    head.writeUInt16LE(nameBuf.length, 26);
    head.writeUInt16LE(0, 28);
    chunks.push(head, nameBuf, body);
    const dir = Buffer.alloc(46);
    dir.writeUInt32LE(0x02014b50, 0);
    dir.writeUInt16LE(20, 4);
    dir.writeUInt16LE(20, 6);
    dir.writeUInt16LE(0, 8);
    dir.writeUInt16LE(stored ? 8 : 0, 10);
    dir.writeUInt16LE(0, 12);
    dir.writeUInt16LE(0x0021, 14);
    dir.writeUInt32LE(crc32(data), 16);
    dir.writeUInt32LE(body.length, 20);
    dir.writeUInt32LE(data.length, 24);
    dir.writeUInt16LE(nameBuf.length, 28);
    dir.writeUInt32LE(offset, 42);
    central.push(dir, nameBuf);
    offset += head.length + nameBuf.length + body.length;
  }
  const dirBuf = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(dirBuf.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...chunks, dirBuf, end]);
}

export function build(out = join(root, 'dist/mcpb')) {
  const manifest = JSON.parse(readFileSync(join(mcp, 'manifest.json'), 'utf8'));
  const entries = bundleFiles().map((name) => ({ name, data: readFileSync(join(mcp, name)) }));
  const bytes = zip(entries);
  mkdirSync(out, { recursive: true });
  const file = join(out, `geoprims-${manifest.version}.mcpb`);
  writeFileSync(file, bytes);
  const digest = createHash('sha256').update(bytes).digest('hex');
  return { file, digest, entries: entries.length, bytes: bytes.length, version: manifest.version };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const r = build();
  console.log(`${relative(root, r.file)}  ${r.entries} files  ${(r.bytes / 1e6).toFixed(1)} MB`);
  console.log(`sha256 ${r.digest}`);
}
