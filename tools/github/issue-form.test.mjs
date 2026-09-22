// feedback/triage-and-corrections "Wrong-answer issue form": the form is
// labeled correctness and GitHub's validation requires the tool or URL, the
// inputs, the answer received, the answer expected, and the published source.
// A zero-dependency reader of the form's block structure, enough to check the
// fields GitHub validates.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const root = new URL('../..', import.meta.url);
const read = (p) => readFileSync(new URL(p, root), 'utf8');

/** Each `- type:` block of the form body: its id, label, and whether it is required. */
export function fields(yaml) {
  const body = yaml.slice(yaml.indexOf('\nbody:'));
  return body
    .split(/\n  - type: /)
    .slice(1)
    .map((b) => ({
      type: b.split('\n')[0].trim(),
      id: /\n    id: (\S+)/.exec(b)?.[1] ?? null,
      label: /\n      label: (.+)/.exec(b)?.[1].trim() ?? null,
      required: /\n    validations:\n      required: true/.test(b),
    }));
}

const REQUIRED = {
  tool: 'Tool or URL',
  inputs: 'Inputs',
  received: 'Answer received',
  expected: 'Answer expected',
  source: 'The published source that settles it',
};

test('the wrong-answer form requires every field the spec names', () => {
  const form = read('.github/ISSUE_TEMPLATE/wrong-answer.yml');
  assert.match(form, /^labels: \["correctness"\]$/m);
  const byId = new Map(fields(form).map((f) => [f.id, f]));
  for (const [id, label] of Object.entries(REQUIRED)) {
    const f = byId.get(id);
    assert.ok(f, `missing field ${id}`);
    assert.equal(f.label, label);
    assert.equal(f.required, true, `${id} must be required`);
  }
});

test('the reader catches a field that is not required', () => {
  const form = read('.github/ISSUE_TEMPLATE/wrong-answer.yml').replace(
    /(id: source[\s\S]*?validations:\n      required: )true/,
    '$1false',
  );
  assert.equal(fields(form).find((f) => f.id === 'source').required, false);
});

test('every link to the form names the file that exists, and config.yml points to private security reports', () => {
  const config = read('.github/ISSUE_TEMPLATE/config.yml');
  assert.match(config, /security\/advisories\/new/);
  for (const p of ['apps/web/src/lib/report.js', 'apps/web/src/layouts/Base.astro', 'mcp/meta.mjs']) {
    for (const m of read(p).matchAll(/issues\/new\?template=([\w.-]+)/g)) {
      assert.doesNotThrow(() => read(`.github/ISSUE_TEMPLATE/${m[1]}`), `${p} links to a missing form ${m[1]}`);
    }
  }
});
