#!/usr/bin/env node
// Manually mirror one confirmed D1 report to GitHub. Nothing invokes this
// script automatically, and --approve is mandatory on every run.
//
//   GEOPRIMS_GITHUB_TOKEN=github_pat_... node worker/scripts/mirror-github.mjs \
//     REPORT_ID --remote --summary "Maintainer-written explanation" --approve
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildIssue, createIssue, MIRROR_QUERY } from '../src/github-mirror.mjs';
import { inline } from './triage.mjs';

const root = new URL('../..', import.meta.url).pathname;
export const DEFAULT_REPOSITORY = 'clay-good/geoprims';

export function parseMirror(argv) {
  const opts = { positional: [] };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--approve') opts.approve = true;
    else if (arg === '--local' || arg === '--remote') opts.target = arg;
    else if (arg.startsWith('--')) opts[arg.slice(2)] = argv[++i];
    else opts.positional.push(arg);
  }
  return opts;
}

export function checkMirror(opts, env = process.env) {
  if (!opts.approve) return new Error('refusing to mirror without explicit --approve');
  if (!opts.summary) return new Error('a maintainer-written --summary is required; reporter notes are never copied');
  if (!env.GEOPRIMS_GITHUB_TOKEN) return new Error('GEOPRIMS_GITHUB_TOKEN is required');
  if (!(opts.file || (opts.positional[0] && opts.target))) {
    return new Error('give REPORT_ID with --remote|--local, or --file report.json');
  }
  return null;
}

export function readReport(opts, execFile = execFileSync) {
  if (opts.file) return JSON.parse(readFileSync(opts.file, 'utf8'));
  const output = execFile(
    'npx',
    ['wrangler', 'd1', 'execute', 'geoprims-reports', opts.target, '--json', '--command', inline(MIRROR_QUERY, [opts.positional[0]])],
    { cwd: join(root, 'worker'), encoding: 'utf8' },
  );
  return JSON.parse(output)[0]?.results?.[0] ?? null;
}

export async function mirror(opts, { env = process.env, execFile = execFileSync, fetchImpl = fetch } = {}) {
  const invalid = checkMirror(opts, env);
  if (invalid) throw invalid;
  const row = readReport(opts, execFile);
  if (!row) throw new Error('no such report');
  const issue = buildIssue(row, opts.summary);
  return createIssue({
    repository: opts.repository ?? env.GEOPRIMS_GITHUB_REPOSITORY ?? DEFAULT_REPOSITORY,
    token: env.GEOPRIMS_GITHUB_TOKEN,
    issue,
    fetchImpl,
    apiOrigin: opts['api-origin'],
  });
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const created = await mirror(parseMirror(process.argv.slice(2)));
    console.log(`Created ${created.url}`);
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
