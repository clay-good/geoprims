import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildIssue, createIssue, MIRROR_QUERY } from '../src/github-mirror.mjs';
import { checkMirror, mirror, parseMirror } from '../scripts/mirror-github.mjs';

const REPORTER_NOTE = 'Reporter secret phrase that must never leave D1';
const row = {
  id: 'r-123',
  tool_id: 'aviation.altimetry.density-altitude',
  tool_version: '1.2.3',
  core_version: '0.4.0',
  build_hash: 'abc123',
  asset_versions_json: '{"wmm":"2025.0"}',
  kind: 'wrong-result',
  page_path: '/aviation/altimetry/density-altitude/#v1:example',
  inputs_json: '[{"field":"temperature","label":"Temperature","value":"30","unit":"°C"}]',
  outputs_json: '[{"field":"density_altitude","label":"Density altitude","value":"7200","unit":"ft"}]',
  status: 'confirmed',
  note: REPORTER_NOTE,
};

test('the structured mirror contains the specified fields and no reporter prose', () => {
  const issue = buildIssue(row, 'Published example gives a lower density altitude.');
  assert.equal(issue.title, 'Confirmed report: aviation.altimetry.density-altitude');
  for (const expected of ['Published example', 'v1:example', '1.2.3', '0.4.0', 'abc123', 'temperature', 'density_altitude']) {
    assert.match(issue.body, new RegExp(expected));
  }
  assert.doesNotMatch(issue.body, new RegExp(REPORTER_NOTE));
  assert.doesNotMatch(MIRROR_QUERY, /\bnote\b/i);
  assert.deepEqual(issue.labels, ['correctness']);
});

test('only confirmed reports and a maintainer-written summary build an issue', () => {
  assert.throws(() => buildIssue({ ...row, status: 'triaged' }, 'Looks wrong.'), /only a confirmed report/);
  assert.throws(() => buildIssue(row, '  '), /maintainer summary/);
  assert.throws(() => buildIssue({ ...row, page_path: '//example.com/report' }, 'Looks wrong.'), /same-site/);
});

test('the CLI requires approval, summary, token, and an explicit report source', () => {
  const complete = parseMirror(['r-123', '--remote', '--summary', 'Checked against the standard.', '--approve']);
  assert.deepEqual(complete, {
    positional: ['r-123'],
    target: '--remote',
    summary: 'Checked against the standard.',
    approve: true,
  });
  assert.equal(checkMirror(complete, { GEOPRIMS_GITHUB_TOKEN: 'token' }), null);
  assert.match(checkMirror({ ...complete, approve: false }, { GEOPRIMS_GITHUB_TOKEN: 'token' }).message, /--approve/);
  assert.match(checkMirror({ ...complete, summary: '' }, { GEOPRIMS_GITHUB_TOKEN: 'token' }).message, /--summary/);
  assert.match(checkMirror(complete, {}).message, /TOKEN/);
  assert.match(checkMirror({ positional: [], approve: true, summary: 'x' }, { GEOPRIMS_GITHUB_TOKEN: 'token' }).message, /REPORT_ID/);
});

test('the GitHub client posts exactly one structured issue to the selected repository', async () => {
  const calls = [];
  const issue = buildIssue(row, 'Checked independently.');
  const created = await createIssue({
    repository: 'geoprims-test/reports',
    token: 'fine-grained-test-token',
    issue,
    apiOrigin: 'https://github.test/api',
    fetchImpl: async (url, init) => {
      calls.push({ url, init });
      return new Response(JSON.stringify({ number: 17, html_url: 'https://github.test/geoprims-test/reports/issues/17' }), {
        status: 201,
        headers: { 'content-type': 'application/json' },
      });
    },
  });
  assert.deepEqual(created, { number: 17, url: 'https://github.test/geoprims-test/reports/issues/17' });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, 'https://github.test/api/repos/geoprims-test/reports/issues');
  assert.equal(calls[0].init.method, 'POST');
  assert.equal(calls[0].init.headers.Authorization, 'Bearer fine-grained-test-token');
  assert.deepEqual(JSON.parse(calls[0].init.body), issue);
  assert.doesNotMatch(calls[0].init.body, new RegExp(REPORTER_NOTE));
});

test('the guarded CLI path reads the safe D1 projection before posting', async () => {
  let d1Args;
  let posted;
  const result = await mirror(
    parseMirror(['r-123', '--remote', '--summary', 'Verified from the published source.', '--approve', '--repository', 'geoprims-test/reports']),
    {
      env: { GEOPRIMS_GITHUB_TOKEN: 'token' },
      execFile(command, args) {
        assert.equal(command, 'npx');
        d1Args = args;
        return JSON.stringify([{ results: [row] }]);
      },
      async fetchImpl(url, init) {
        posted = { url, init };
        return new Response(JSON.stringify({ number: 18, html_url: 'https://github.test/issues/18' }), { status: 201 });
      },
    },
  );
  assert.equal(result.number, 18);
  assert.ok(d1Args.includes('--remote'));
  assert.doesNotMatch(d1Args.at(-1), /\bnote\b/i);
  assert.equal(posted.url, 'https://api.github.com/repos/geoprims-test/reports/issues');
  assert.doesNotMatch(posted.init.body, new RegExp(REPORTER_NOTE));
});

test('GitHub failures do not look like successful mirrors', async () => {
  await assert.rejects(
    createIssue({
      repository: 'geoprims-test/reports',
      token: 'token',
      issue: buildIssue(row, 'Checked.'),
      fetchImpl: async () => new Response('{"message":"forbidden"}', { status: 403 }),
    }),
    /failed \(403\).*forbidden/,
  );
  await assert.rejects(createIssue({ repository: 'not-a-repository', token: 'token', issue: {} }), /owner\/name/);
});
