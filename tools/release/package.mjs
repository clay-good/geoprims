#!/usr/bin/env node
// Assemble the small, user-facing assets attached to a GitHub release.
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build as buildMcpb } from '../mcp/build-mcpb.mjs';
import { artifactDigests } from '../wasm/artifact-digests.mjs';

const root = fileURLToPath(new URL('../..', import.meta.url));
const sha256 = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');

export function packageRelease(out = join(root, 'dist/release')) {
  const pkg = JSON.parse(readFileSync(join(root, 'mcp/package.json'), 'utf8'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const evalReport = JSON.parse(readFileSync(join(root, 'data/mcp-eval.json'), 'utf8'));
  const version = pkg.version;
  if (catalog.coreVersion !== version) throw new Error(`release ${version} disagrees with core ${catalog.coreVersion}`);

  mkdirSync(out, { recursive: true });
  const bundle = buildMcpb(out).file;
  const copies = [
    ['data/sbom.cdx.json', `geoprims-${version}-sbom.cdx.json`],
    ['data/mcp-eval.json', `geoprims-${version}-mcp-eval.json`],
    [`apps/web/dist/verification/${version}.json`, `geoprims-${version}-verification.json`],
  ];
  const assets = [bundle];
  for (const [source, name] of copies) {
    const target = join(out, name);
    copyFileSync(join(root, source), target);
    assets.push(target);
  }
  const digestFile = join(out, 'artifact-digests.txt');
  writeFileSync(digestFile, artifactDigests(root, ['dist/catalog', 'dist/wasm', 'mcp/dist', 'apps/web/dist']));
  assets.push(digestFile);
  assets.sort((a, b) => basename(a).localeCompare(basename(b)));
  const sums = assets.map((file) => `${sha256(file)}  ${basename(file)}`).join('\n') + '\n';
  writeFileSync(join(out, 'SHA256SUMS'), sums);

  const notes = [
    `## Verified artifacts`,
    '',
    `${catalog.counts.all.operations} operations and ${catalog.counts.all.endpoints} endpoints; ${catalog.counts.stable.operations} operations are stable.`,
    `The MCP evaluation covers ${evalReport.tasks} tasks (${evalReport.selection1}% top-1 selection, ${evalReport.answered}% answered end to end).`,
    '',
    'The tag contains the prebuilt `mcp/dist`, so a clone runs with Node 22 and no install step. The MCPB bundle, verification report, evaluation report, SBOM, and complete shipped-file digest manifest are attached.',
    '',
    'Verify a downloaded asset:',
    '',
    '```bash',
    `sha256sum -c SHA256SUMS --ignore-missing`,
    `gh attestation verify geoprims-${version}.mcpb -R clay-good/geoprims`,
    '```',
    '',
    'Planning and education aid. Not certified for navigation, and not a legal survey.',
    '',
  ].join('\n');
  writeFileSync(join(out, 'RELEASE_NOTES.md'), notes);
  return { version, assets: assets.map((file) => basename(file)), sums, notes };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const release = packageRelease();
  console.log(`release ${release.version}: ${release.assets.length} hashed assets plus SHA256SUMS`);
}
