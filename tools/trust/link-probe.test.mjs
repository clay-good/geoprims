// The free-access link probe (trust/freshness "Broken link"), run against a
// local fixture server rather than the network: a moved PDF is reported with
// its citation, the tools that cite it, and the HTTP status.
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { collectLinks, probeAll, report } from './link-probe.mjs';
import { readLedger } from './ledger.mjs';

const root = new URL('../..', import.meta.url).pathname;

async function fixture(t) {
  const server = createServer((req, res) => {
    const routes = {
      '/ok.pdf': () => res.writeHead(200).end('ok'),
      '/moved.pdf': () => res.writeHead(404).end(),
      '/old.pdf': () => res.writeHead(301, { location: '/new/report.pdf' }).end(),
      '/dir': () => res.writeHead(301, { location: '/dir/' }).end(),
      '/blocked': () => res.writeHead(403).end(),
      // Answers HEAD wrongly but serves the page.
      '/no-head': () => res.writeHead(req.method === 'HEAD' ? 405 : 200).end(),
      '/slow': () => setTimeout(() => res.writeHead(200).end(), 500),
      '/walled': () => res.writeHead(302, { location: 'https://unblock.example.gov/' }).end(),
      // Drops the connection on HEAD, as some servers do, and serves GET.
      '/drops-head': () => (req.method === 'HEAD' ? req.socket.destroy() : res.writeHead(200).end()),
    };
    (routes[req.url] ?? (() => res.writeHead(500).end()))();
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  t.after(() => server.close());
  return `http://127.0.0.1:${server.address().port}`;
}

test('the probe names the citation, its tools, and the HTTP status of each bad link', async (t) => {
  const base = await fixture(t);
  const catalog = {
    tools: [
      { id: 'geodesy.geoid.geoid-height', references: [{ title: 'GEOID18 technical details', edition: '2019', url: `${base}/moved.pdf` }] },
      { id: 'geodesy.height.convert', references: [{ title: 'GEOID18 technical details', edition: '2019', url: `${base}/moved.pdf` }, { title: 'Working', url: `${base}/ok.pdf` }] },
      { id: 'navigation.geodesic.inverse', references: [{ title: 'Old report', url: `${base}/old.pdf` }, { title: 'A folder', url: `${base}/dir` }] },
      { id: 'time.sun.events', references: [{ title: 'Bot wall', url: `${base}/blocked` }, { title: 'No HEAD', url: `${base}/no-head` }] },
      { id: 'time.sun.position', references: [{ title: 'Unblock page', url: `${base}/walled` }, { title: 'Dropped HEAD', url: `${base}/drops-head` }] },
    ],
  };
  const links = collectLinks([], catalog);
  assert.equal(links.length, 8, 'one entry per URL');
  const results = await probeAll(links, { timeoutMs: 2000 });
  const state = Object.fromEntries(results.map((r) => [new URL(r.url).pathname, r.state]));
  assert.deepEqual(state, {
    '/moved.pdf': 'broken', '/ok.pdf': 'ok', '/old.pdf': 'redirected', '/dir': 'ok', '/blocked': 'refused', '/no-head': 'ok', '/walled': 'refused', '/drops-head': 'ok',
  });

  const body = report(results, { date: '2026-11-01' });
  const line = body.split('\n').find((l) => l.includes('/moved.pdf'));
  assert.match(line, /\| 404 \|/);
  assert.match(line, /GEOID18 technical details, 2019/);
  assert.match(line, /`geodesy\.geoid\.geoid-height`, `geodesy\.height\.convert`/);
  assert.ok(body.includes(`301 → ${base}/new/report.pdf`), 'the redirect names where it goes');
  assert.match(body, /## Not checked \(2\)[\s\S]*\/blocked \| 403/);
  assert.ok(!body.includes('/ok.pdf') && !body.includes('/no-head') && !body.includes(`${base}/dir |`), 'working links stay out of the issue');
});

test('a DOI resolving and an archive snapshot settling are not moves', async () => {
  const fake = (location) => async () => new Response(null, { status: 302, headers: { location } });
  const { probe } = await import('./link-probe.mjs');
  assert.equal((await probe('https://doi.org/10.1007/s001900050278', { fetchImpl: fake('http://link.springer.com/10.1007/s001900050278') })).state, 'ok');
  assert.equal((await probe('https://web.archive.org/web/2008/http://geohash.org/', { fetchImpl: fake('https://web.archive.org/web/20080304/http://geohash.org/') })).state, 'ok');
  assert.equal((await probe('https://www.rfc-editor.org/rfc/rfc7946', { fetchImpl: fake('https://www.rfc-editor.org/info/rfc7946/') })).state, 'redirected');
});

test('a link that never answers is unreachable, not broken, and all-good means no issue', async (t) => {
  const base = await fixture(t);
  // Nothing answered, so the document is not known to be gone: the host may be
  // refusing automated traffic or be down. Reported apart from a real 4xx.
  const [slow] = await probeAll([{ url: `${base}/slow`, citations: ['Slow'], tools: [] }], { timeoutMs: 100, retries: 0 });
  assert.deepEqual([slow.state, slow.status], ['unreachable', 'timeout']);
  const good = await probeAll([{ url: `${base}/ok.pdf`, citations: ['Fine'], tools: [] }]);
  assert.equal(report(good), '');
});

test('a connection that fails once and then answers is not reported', async () => {
  const { probe } = await import('./link-probe.mjs');
  // Three of the sixteen links reported broken on 2026-11-01 answered on the
  // next try, so one bad connection must not reach the issue.
  let calls = 0;
  const flaky = async () => {
    calls += 1;
    if (calls <= 2) throw Object.assign(new Error('timed out'), { name: 'TimeoutError' });
    return new Response(null, { status: 200 });
  };
  assert.equal((await probe('https://example.gov/doc.pdf', { fetchImpl: flaky, retryDelayMs: 0 })).state, 'ok');
  // And a host that is really down still reports, after the retry.
  const dead = async () => {
    throw Object.assign(new Error('timed out'), { name: 'TimeoutError' });
  };
  const out = await probe('https://example.gov/doc.pdf', { fetchImpl: dead, retryDelayMs: 0 });
  assert.deepEqual([out.state, out.status], ['unreachable', 'timeout']);
});

test('a redirect to the front page is not a move to cite', async () => {
  const { probe } = await import('./link-probe.mjs');
  // itrf.ign.fr sent every path to its own front page while in maintenance,
  // and the issue advised citing that address directly. It is not the document.
  const fake = (location) => async () => new Response(null, { status: 301, headers: { location } });
  const root = await probe('https://itrf.ign.fr/docs/solutions/itrf2020/Transfo-ITRF2020_TRFs.txt', {
    fetchImpl: fake('https://itrf.ign.fr/'),
  });
  assert.deepEqual([root.state, root.to], ['refused', 'https://itrf.ign.fr/']);
  // A move to a real path is still a move.
  const moved = await probe('https://example.gov/old.pdf', { fetchImpl: fake('https://example.gov/new/old.pdf') });
  assert.equal(moved.state, 'redirected');
  // A front page asking for a front page is nothing unusual.
  const home = await probe('https://example.gov/', { fetchImpl: fake('https://example.gov/') });
  assert.equal(home.state, 'ok');
});

test('the report tells a reader not to repoint a citation the probe could not reach', async () => {
  const body = report([
    { url: 'https://nga.example.mil/doc.pdf', state: 'unreachable', status: 'timeout', citations: ['A standard'], tools: ['geodesy.utm.forward'] },
    { url: 'https://example.gov/gone.pdf', state: 'broken', status: 404, citations: ['A gone thing'], tools: [] },
  ], { date: '2026-11-07' });
  assert.match(body, /## No answer \(1\)/);
  assert.match(body, /check in a browser first/);
  assert.match(body, /## Broken \(1\)/);
  // The broken note no longer claims a link that did not answer is gone.
  assert.doesNotMatch(body, /These did not answer, or answered with an error/);
});

test('the real catalog and ledger give every link a citation', () => {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const links = collectLinks(readLedger(root), catalog);
  assert.ok(links.length > 150, `${links.length} links`);
  for (const l of links) {
    assert.match(l.url, /^https?:\/\//, l.url);
    assert.ok(l.citations.length > 0, `${l.url} has no citation`);
  }
  // A cited link carries the tools that cite it.
  assert.ok(links.filter((l) => l.tools.length).length > 140);
});
