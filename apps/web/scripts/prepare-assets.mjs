// Copies the built core into public/: Wasm modules and the catalog. Run after
// `npm run build` at the repository root (which builds dist/).
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
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
cpSync(join(root, 'assets/registry.json'), join(web, 'public/assets/registry.json'));
// Golden vectors, downloadable per tool at /vectors/<tool id>.jsonl (proof-display).
rmSync(join(web, 'public/vectors'), { recursive: true, force: true });
cpSync(join(root, 'core/vectors'), join(web, 'public/vectors'), { recursive: true });
console.log('prepared public/wasm, public/catalog/v1.json, public/assets, and public/vectors');
