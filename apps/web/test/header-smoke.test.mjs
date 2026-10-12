import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { smoke } from '../scripts/header-smoke.mjs';
import { matches, parseHeaders } from '../scripts/serve.mjs';

const web = new URL('..', import.meta.url).pathname;
const rules = parseHeaders(readFileSync(join(web, 'dist/_headers'), 'utf8'));

function deployment(mode) {
  return async (url) => {
    const path = new URL(url).pathname;
    const expected = Object.assign({}, ...rules.filter(([pattern]) => matches(pattern, path)).map(([, headers]) => headers));
    const headers = new Headers(expected);
    if (mode === 'preview') headers.set('X-Robots-Tag', 'noindex, nofollow, noarchive');
    const html = path === '/' || path.endsWith('/') || path === '/not-a-real-route';
    const meta = expected['Content-Security-Policy']?.split('; ').filter((directive) => !directive.startsWith('frame-ancestors')).join('; ');
    const body = html ? `<meta http-equiv="Content-Security-Policy" content="${meta}">` : 'asset';
    return new Response(body, { status: path === '/not-a-real-route' ? 404 : 200, headers });
  };
}

test('the deployed-header smoke distinguishes staging from production', async () => {
  assert.equal((await smoke('https://preview.example', deployment('preview'), 'preview')).length, 6);
  assert.equal((await smoke('https://www.example', deployment('production'), 'production')).length, 6);
  await assert.rejects(smoke('https://www.example', deployment('preview'), 'production'), /must not send X-Robots-Tag/);
});
