// Copies the built core into public/: Wasm modules and the catalog. Run after
// `npm run build` at the repository root (which builds dist/).
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
for (const need of ['dist/wasm/base.wasm', 'dist/catalog/v1.json']) {
  if (!existsSync(join(root, need))) throw new Error(`${need} is missing: run npm run build at the repository root first`);
}
rmSync(join(web, 'public/wasm'), { recursive: true, force: true });
mkdirSync(join(web, 'public/catalog'), { recursive: true });
cpSync(join(root, 'dist/wasm'), join(web, 'public/wasm'), { recursive: true, filter: (p) => !p.endsWith('.json') });
cpSync(join(root, 'dist/catalog/v1.json'), join(web, 'public/catalog/v1.json'));
// On-demand data assets, served same-origin at /assets/<id>/<version>/<file>.
// A file the registry marks `pack: false` is upstream source kept for
// provenance, not something the site serves: tz-boundaries ships the 49.5 MiB
// timezones.geojson.zip that boundaries.bin is derived from, and copying the
// whole of assets/data took it along. Cloudflare Workers refuses any asset over
// 25 MiB, so the deploy aborted on it and the site stopped being deployable.
const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
const unpacked = new Set(
  registry.assets.flatMap((a) =>
    Object.entries(a.files ?? {})
      .filter(([, f]) => f.pack === false)
      .map(([name]) => join(root, 'assets/data', a.id, a.version, name)),
  ),
);
rmSync(join(web, 'public/assets'), { recursive: true, force: true });
cpSync(join(root, 'assets/data'), join(web, 'public/assets'), {
  recursive: true,
  filter: (p) => !unpacked.has(p),
});
for (const p of unpacked) {
  if (!existsSync(p)) throw new Error(`registry marks ${p} pack: false but it is not in assets/data`);
}
// A file the registry splits into `parts` is served as those parts and not as
// itself: Cloudflare Workers refuses any asset over 25 MiB, and the 26.9 MiB
// time-zone boundary index was stopping the whole site from deploying. The
// runtime's asset provider joins the parts back and checks each one against its
// pinned digest and the join against the file's, so the core still asks for the
// whole file and gets one verified buffer.
for (const asset of registry.assets) {
  for (const [name, file] of Object.entries(asset.files ?? {})) {
    if (!Array.isArray(file.parts)) continue;
    const dir = join(web, 'public/assets', asset.id, asset.version);
    const whole = readFileSync(join(root, 'assets/data', asset.id, asset.version, name));
    let at = 0;
    for (const part of file.parts) {
      const bytes = whole.subarray(at, at + part.bytes);
      at += part.bytes;
      const sha256 = createHash('sha256').update(bytes).digest('hex');
      if (bytes.length !== part.bytes || sha256 !== part.sha256) {
        throw new Error(`${asset.id}/${name}: ${part.file} does not match its pinned digest; re-pin the parts in assets/registry.json`);
      }
      writeFileSync(join(dir, part.file), bytes);
    }
    if (at !== whole.length) throw new Error(`${asset.id}/${name}: the parts cover ${at} of ${whole.length} bytes`);
    rmSync(join(dir, name), { force: true });
  }
}
cpSync(join(root, 'assets/registry.json'), join(web, 'public/assets/registry.json'));
// Golden vectors, downloadable per tool at /vectors/<tool id>.jsonl (proof-display).
rmSync(join(web, 'public/vectors'), { recursive: true, force: true });
cpSync(join(root, 'core/vectors'), join(web, 'public/vectors'), { recursive: true });
console.log('prepared public/wasm, public/catalog/v1.json, public/assets, and public/vectors');
