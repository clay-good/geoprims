#!/usr/bin/env node
// Fail before a release build if its tag, package metadata, or committed
// clone-and-run payload disagree.
import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));

export function validateRelease(tag, pkg, manifest, trackedFiles) {
  const match = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/.exec(tag);
  if (!match) throw new Error(`release tag must be v<semver>, got ${tag}`);
  const version = match[1];
  if (pkg.version !== version) throw new Error(`${tag} disagrees with mcp/package.json ${pkg.version}`);
  if (manifest.version !== version) throw new Error(`${tag} disagrees with mcp/manifest.json ${manifest.version}`);
  if (pkg.private !== true || pkg.publishConfig) throw new Error('the repository-only MCP package must be private with no registry target');
  if (Object.keys(pkg.dependencies ?? {}).length) throw new Error('the release MCP server must have zero runtime dependencies');
  for (const required of ['mcp/dist/wasm/modules.json', 'mcp/dist/catalog/v1.json', 'mcp/dist/assets/registry.json']) {
    if (!trackedFiles.includes(required)) throw new Error(`${tag} does not commit ${required}`);
  }
  return { version, prerelease: version.includes('-') };
}

export function inspectRelease(tag) {
  const read = (path) => JSON.parse(readFileSync(join(root, path), 'utf8'));
  const trackedFiles = execFileSync('git', ['ls-files', 'mcp/dist'], { cwd: root, encoding: 'utf8' }).trim().split('\n').filter(Boolean);
  return validateRelease(tag, read('mcp/package.json'), read('mcp/manifest.json'), trackedFiles);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const tag = process.argv[2];
  if (!tag) throw new Error('usage: node tools/release/contract.mjs v<semver>');
  const release = inspectRelease(tag);
  console.log(`${tag}: version ${release.version}, ${release.prerelease ? 'prerelease' : 'stable'}, committed mcp/dist`);
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, `version=${release.version}\nprerelease=${release.prerelease}\n`);
  }
}
