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

// The units files are built by four generators in order: gen_units.py writes
// the hand-picked cases and a sweep, then gen_units_gaps.py, gen_units_pairs.py
// and gen_units_special.py append what it did not reach. Reproducibility is the
// whole chain, run in that order, not the first script alone.
//
// A superseded vector is the one thing the generators cannot reproduce, and
// that is the point of superseding: the published line stays as it was
// published, and the corrected vector goes in at a new id. So each generated
// vector must match the committed one with its id, unless that one has been
// superseded -- in which case it must match the replacement the supersede
// pointed at, which is the same vector under a different name.
const GENERATORS = ['gen_units.py', 'gen_units_gaps.py', 'gen_units_pairs.py', 'gen_units_special.py'];

test('units vectors are reproducible from tools/vectors/gen_units.py', () => {
  const out = mkdtempSync(join(tmpdir(), 'gp-vectors-'));
  for (const gen of GENERATORS) execFileSync('python3', [join(root, 'tools/vectors', gen), out]);
  const parse = (p) => new Map(readFileSync(p, 'utf8').split('\n').filter(Boolean).map((l) => [JSON.parse(l).id, JSON.parse(l)]));
  for (const f of readdirSync(out)) {
    const made = parse(join(out, f));
    const have = parse(join(dir, f));
    assert.equal(have.size >= made.size, true, `${f} has fewer vectors than its generator makes`);
    for (const [id, want] of made) {
      let got = have.get(id);
      assert.ok(got, `${f} ${id} is missing`);
      if (got.supersededBy) {
        got = have.get(got.supersededBy);
        assert.ok(got, `${f} ${id} is superseded by a vector that is not there`);
      }
      assert.deepEqual({ ...got, id }, { ...want, id }, `${f} ${id} differs from its generator`);
    }
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
