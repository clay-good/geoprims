// Copies the built core into public/: Wasm modules and the catalog. Run after
// `npm run build` at the repository root (which builds dist/).
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
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
rmSync(join(web, 'public/assets'), { recursive: true, force: true });
cpSync(join(root, 'assets/data'), join(web, 'public/assets'), { recursive: true });
cpSync(join(root, 'assets/registry.json'), join(web, 'public/assets/registry.json'));
console.log('prepared public/wasm, public/catalog/v1.json, and public/assets');
