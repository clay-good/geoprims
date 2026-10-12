#!/usr/bin/env node
// Fail closed when an npm build dependency adds an undeclared or unreviewed
// license. The website is private, but its build chain is still part of the
// release supply chain.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
export const ALLOWED = new Set([
  '0BSD',
  'Apache-2.0',
  'Apache-2.0 AND LGPL-3.0-or-later',
  'Apache-2.0 AND LGPL-3.0-or-later AND MIT',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BlueOak-1.0.0',
  'CC0-1.0',
  'ISC',
  'LGPL-3.0-or-later',
  'MIT',
  'MIT OR Apache-2.0',
  'MPL-2.0',
  'Python-2.0',
]);

export function licenseProblems(lock, allowed = ALLOWED) {
  const problems = [];
  for (const [path, entry] of Object.entries(lock.packages ?? {})) {
    if (!path) continue;
    const name = path.replace(/^.*node_modules\//, '');
    if (!entry.license) problems.push(`${name}@${entry.version}: no license in package-lock.json`);
    else if (!allowed.has(entry.license)) problems.push(`${name}@${entry.version}: unreviewed license ${entry.license}`);
  }
  return problems;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const lock = JSON.parse(readFileSync(resolve(root, 'apps/web/package-lock.json'), 'utf8'));
  const problems = licenseProblems(lock);
  if (problems.length) {
    console.error(problems.join('\n'));
    process.exitCode = 1;
  } else {
    const dependencies = Object.keys(lock.packages).length - 1;
    const licenses = new Set(Object.values(lock.packages).map((entry) => entry.license).filter(Boolean));
    console.log(`npm licenses: ${dependencies} locked dependencies use ${licenses.size} reviewed license expressions`);
  }
}
