// Vector integrity (verification "Regression history"):
// 1. Generated vector files are reproducible from their generator.
// 2. No published vector is silently edited: compared with the base revision
//    (VECTOR_BASE, default origin/main), a vector whose id still exists must keep
//    its input and expectations unless it is marked superseded with a reason.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const root = new URL('../..', import.meta.url).pathname;
const dir = join(root, 'core/vectors');

test('units vectors are reproducible from tools/vectors/gen_units.py', () => {
  const out = mkdtempSync(join(tmpdir(), 'gp-vectors-'));
  execFileSync('python3', [join(root, 'tools/vectors/gen_units.py'), out]);
  for (const f of readdirSync(out)) {
    assert.equal(readFileSync(join(out, f), 'utf8'), readFileSync(join(dir, f), 'utf8'), `${f} differs from its generator`);
  }
});

test('published vectors are never silently edited', () => {
  const base = process.env.VECTOR_BASE ?? 'origin/main';
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' });
  try {
    git('rev-parse', '--verify', base);
  } catch {
    return; // no base revision available (fresh clone without remotes)
  }
  for (const f of readdirSync(dir).filter((x) => x.endsWith('.jsonl'))) {
    let old;
    try {
      old = git('show', `${base}:core/vectors/${f}`);
    } catch {
      continue; // new file
    }
    const now = new Map(readFileSync(join(dir, f), 'utf8').split('\n').filter(Boolean).map((l) => [JSON.parse(l).id, JSON.parse(l)]));
    for (const line of old.split('\n').filter(Boolean)) {
      const was = JSON.parse(line);
      const is = now.get(was.id);
      assert.ok(is, `${f} ${was.id} was deleted; supersede it instead`);
      if (is.supersededBy) {
        assert.ok(is.reason, `${f} ${was.id} is superseded without a reason`);
        continue;
      }
      assert.deepEqual(
        { input: is.input, expect: is.expect },
        { input: was.input, expect: was.expect },
        `${f} ${was.id} changed without a supersession record`,
      );
    }
  }
});
