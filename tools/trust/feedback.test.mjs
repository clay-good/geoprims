// The feedback-loop gate on the real build, and on one bad fixture per check.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';
import { checkCsp, checkLazyImport, checkLimits, checkPages, checkServiceWorker } from './feedback.mjs';
import { sql as generated } from '../codegen/d1-migration.mjs';
import { policy } from '../../apps/web/scripts/headers.mjs';

const root = new URL('../..', import.meta.url).pathname;
const web = join(root, 'apps/web');
const dist = join(web, 'dist');
const read = (p) => readFileSync(join(root, p), 'utf8');
const built = existsSync(join(dist, 'sw.js')) ? false : 'needs the web build';

const components = Object.fromEntries(
  readdirSync(join(web, 'src/components'))
    .filter((f) => f.endsWith('.svelte'))
    .map((f) => [`apps/web/src/components/${f}`, readFileSync(join(web, 'src/components', f), 'utf8')]),
);
const limitsInput = () => ({
  clientSrc: read('apps/web/src/lib/report.js'),
  workerSrc: read('worker/src/report.mjs'),
  migration: read('worker/migrations/0001_problem_reports.sql'),
  generated,
});
const swInput = () => {
  const text = readFileSync(join(dist, 'sw.js'), 'utf8');
  return { source: text, precache: vm.runInNewContext(`${text.split('\n\n')[0]}; PRECACHE`) };
};

test('the real build passes every check', { skip: built }, () => {
  const catalog = JSON.parse(read('dist/catalog/v1.json'));
  const pages = catalog.tools.map((t) => ({ id: t.id, html: readFileSync(join(dist, ...t.id.split('.'), 'index.html'), 'utf8') }));
  assert.deepEqual(
    [...checkPages(pages), ...checkLazyImport(components), ...checkLimits(limitsInput()), ...checkServiceWorker(swInput()), ...checkCsp(policy({}))],
    [],
  );
});

test('each bad fixture fails its check', { skip: built }, () => {
  const page = '<button>Report a problem</button>';
  assert.match(checkPages([{ id: 'x', html: page + page }])[0], /2 report buttons/);
  assert.match(checkPages([{ id: 'x', html: '<p>no button</p>' }])[0], /0 report buttons/);
  assert.match(checkPages([{ id: 'x', html: `${page}<script src="https://challenges.cloudflare.com/turnstile/v0/api.js"></script>` }])[0], /before a click/);

  const eager = { ...components, 'x.svelte': "import ReportDialog from './ReportDialog.svelte';" };
  assert.match(checkLazyImport(eager).join(), /statically/);
  assert.match(checkLazyImport({ 'x.svelte': '' }).join(), /no dynamic import/);

  const L = limitsInput();
  assert.match(checkLimits({ ...L, migration: L.migration.replace('<= 280', '<= 500') }).join(), /migration differs/);
  assert.match(checkLimits({ ...L, clientSrc: L.clientSrc.replace("from '../../../../data/report-limits.json'", "from './limits.json'") }).join(), /client does not read/);

  const sw = swInput();
  assert.match(checkServiceWorker({ ...sw, source: sw.source.replace("pathname.startsWith('/api/')) return", "pathname.startsWith('/nope/')) return") }).join(), /does not return before/);
  assert.match(checkServiceWorker({ ...sw, precache: [...sw.precache, '/api/reports/config'] }).join(), /precaches/);

  const csp = policy({});
  const without = (name, value) => csp.map((d) => (d[0] === name ? d.filter((x) => x !== value) : d));
  assert.match(checkCsp(without('frame-src', 'https://challenges.cloudflare.com')).join(), /frame-src/);
  assert.match(checkCsp(without('script-src', 'https://challenges.cloudflare.com')).join(), /script-src/);
  assert.match(checkCsp(csp.map((d) => (d[0] === 'connect-src' ? [...d, '*'] : d))).join(), /somewhere other/);
});
