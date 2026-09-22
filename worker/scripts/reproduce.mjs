#!/usr/bin/env node
// Reproduction from the report alone (feedback/triage-and-corrections): takes
// a report and prints what it said, the recomputation on the reported build,
// and the recomputation on the current build, marking every difference.
//
//   node worker/scripts/reproduce.mjs REPORT_ID --remote
//   node worker/scripts/reproduce.mjs --file report.json [--reported-wasm DIR]
//
// The permalink in page_path restores the inputs through the core's own
// decoder. The reported build is the current one when the module hash
// matches; otherwise pass the dist/wasm directory of that release.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost, moduleFor } from '../../packages/runtime/src/node.mjs';
import { REPORT_QUERY } from '../src/triage.mjs';
import { inline, parse } from './triage.mjs';

const root = new URL('../..', import.meta.url).pathname;

/** The inputs a report's permalink restores, or null when it was sent without them. */
export async function argsOf(host, pagePath) {
  const frag = pagePath.split('#')[1];
  if (!frag || !frag.startsWith('v1:')) return null;
  const link = await host.module('link');
  const out = JSON.parse(await link.callString('gp_link_decode', frag));
  if (!out.ok) throw new Error(`the permalink does not decode: ${out.error.message}`);
  const { i = {}, u } = out.result.state;
  return u ? { ...i, options: { outputUnits: u } } : i;
}

/** Output rows { field, value, unit } from a result envelope, as a report stores them. */
export function rowsOf(envelope) {
  if (!envelope.ok) return [{ field: 'error', value: envelope.error.code, unit: '' }];
  return Object.entries(envelope.result).map(([field, v]) =>
    v && typeof v === 'object' && 'value' in v
      ? { field, value: String(v.value), unit: v.unit }
      : { field, value: typeof v === 'string' ? v : JSON.stringify(v), unit: '' },
  );
}

const same = (a, b) => {
  if (!a || !b) return false;
  if (a.unit !== b.unit) return false;
  const x = Number(a.value);
  const y = Number(b.value);
  if (Number.isFinite(x) && Number.isFinite(y)) return x === y || Math.abs(x - y) <= 1e-9 * Math.max(Math.abs(x), Math.abs(y));
  return a.value === b.value;
};

/**
 * Recomputes `row` (a report as REPORT_QUERY returns it) on `current` and,
 * when given, `reported` hosts. Returns the three sets of rows and the
 * fields that differ between each pair.
 */
export async function reproduce(row, { current, reported = null }) {
  const args = await argsOf(current, row.page_path);
  if (!args) return { reproducible: false, why: 'The report was sent without its inputs (the include toggle was off).' };
  const said = JSON.parse(row.outputs_json).map(({ field, value, unit }) => ({ field, value, unit }));
  const run = async (host) => (host ? rowsOf(JSON.parse(await host.invoke(row.tool_id, JSON.stringify(args)))) : null);
  const now = await run(current);
  const then = await run(reported);
  const fields = [...new Set([...said, ...(then ?? []), ...now].map((r) => r.field))];
  const pick = (rows, f) => rows?.find((r) => r.field === f) ?? null;
  // Against the report, only the fields it carried (it may be cut at the row limit).
  const saidFields = said.map((r) => r.field);
  const differ = (a, b, over = fields) => (a && b ? over.filter((f) => !same(pick(a, f), pick(b, f))) : null);
  return {
    reproducible: true,
    args,
    fields,
    said,
    then,
    now,
    saidVsThen: differ(said, then, saidFields),
    thenVsNow: differ(then, now),
    saidVsNow: differ(said, now, saidFields),
  };
}

function print(row, r, reportedNote) {
  console.log(`Report ${row.id}: ${row.tool_id} ${row.tool_version} (build ${row.build_hash}), ${row.kind}, ${row.status ?? ''}`);
  if (!r.reproducible) return console.log(r.why);
  console.log(`Inputs: ${JSON.stringify(r.args)}`);
  if (reportedNote) console.log(reportedNote);
  const cell = (x) => (x ? `${x.value} ${x.unit}`.trim() : '—');
  const pick = (rows, f) => rows?.find((x) => x.field === f);
  console.log(['field', 'reported', 'reported build', 'current build'].join('\t'));
  for (const f of r.fields) {
    const mark = (r.saidVsNow ?? []).includes(f) || (r.thenVsNow ?? []).includes(f) ? '  ≠' : '';
    console.log([f, cell(pick(r.said, f)), r.then ? cell(pick(r.then, f)) : 'n/a', cell(pick(r.now, f))].join('\t') + mark);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const { opts } = parse(['reproduce', ...process.argv.slice(2)]);
  let row;
  if (opts.file) {
    row = JSON.parse(readFileSync(opts.file, 'utf8'));
  } else if (opts.positional[0] && opts.target) {
    const out = execFileSync('npx', ['wrangler', 'd1', 'execute', 'geoprims-reports', opts.target, '--json', '--command', inline(REPORT_QUERY, [opts.positional[0]])], {
      cwd: join(root, 'worker'),
      encoding: 'utf8',
    });
    row = JSON.parse(out)[0]?.results?.[0];
  } else {
    console.error('usage: reproduce.mjs REPORT_ID --remote|--local, or --file report.json [--reported-wasm DIR]');
    process.exit(1);
  }
  if (!row) {
    console.error('No such report.');
    process.exit(1);
  }
  const wasm = join(root, 'dist/wasm');
  const current = nodeHost(wasm);
  const modules = JSON.parse(readFileSync(join(wasm, 'modules.json'), 'utf8')).modules;
  const hash = (modules.find((m) => m.module === moduleFor(row.tool_id))?.sha256 ?? '').slice(0, 16);
  let reported = null;
  let note = null;
  if (opts['reported-wasm']) reported = nodeHost(opts['reported-wasm']);
  else if (hash && hash === row.build_hash) {
    reported = current;
    note = 'The reported build is the current build.';
  } else note = `The reported build (${row.build_hash}) is not this one (${hash}); pass --reported-wasm with that release's dist/wasm to compare.`;
  print(row, await reproduce(row, { current, reported }), note);
}
