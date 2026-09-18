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
console.log('prepared public/wasm and public/catalog/v1.json');
