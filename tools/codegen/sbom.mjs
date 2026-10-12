#!/usr/bin/env node
// Generate the release dependency inventory from the two authoritative lockfiles.
// CycloneDX metadata intentionally omits a timestamp so identical inputs produce
// identical bytes.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, 'data/sbom.cdx.json');

function npmName(path, entry) {
  return entry.name ?? path.replace(/^.*node_modules\//, '');
}

function npmPurl(name, version) {
  const parts = name.split('/').map(encodeURIComponent);
  return `pkg:npm/${parts.join('/')}@${encodeURIComponent(version)}`;
}

function cargoPurl(name, version) {
  return `pkg:cargo/${encodeURIComponent(name)}@${encodeURIComponent(version)}`;
}

function component({ ecosystem, name, version, license, scope }) {
  const purl = ecosystem === 'npm' ? npmPurl(name, version) : cargoPurl(name, version);
  return {
    type: 'library',
    'bom-ref': purl,
    name,
    version,
    licenses: [{ expression: license }],
    purl,
    scope,
    properties: [{ name: 'geoprims:ecosystem', value: ecosystem }],
  };
}

export function buildSbom({ npmLock, cargoMetadata }) {
  const components = [];
  for (const [path, entry] of Object.entries(npmLock.packages ?? {})) {
    if (!path) continue;
    if (!entry.version || !entry.license) throw new Error(`${path}: SBOM requires version and license`);
    components.push(component({
      ecosystem: 'npm',
      name: npmName(path, entry),
      version: entry.version,
      license: entry.license,
      scope: entry.optional ? 'optional' : 'excluded',
    }));
  }

  for (const entry of cargoMetadata.packages.filter((entry) => entry.source)) {
    if (!entry.license) throw new Error(`${entry.name}@${entry.version}: SBOM requires a license`);
    components.push(component({
      ecosystem: 'cargo',
      name: entry.name,
      version: entry.version,
      license: entry.license === 'MIT/Apache-2.0' ? 'MIT OR Apache-2.0' : entry.license,
      scope: 'required',
    }));
  }

  components.sort((a, b) =>
    a.properties[0].value.localeCompare(b.properties[0].value)
      || a.name.localeCompare(b.name)
      || a.version.localeCompare(b.version));

  const refs = components.map((entry) => entry['bom-ref']);
  if (new Set(refs).size !== refs.length) throw new Error('SBOM contains a duplicate package URL');

  return {
    bomFormat: 'CycloneDX',
    specVersion: '1.6',
    version: 1,
    metadata: {
      component: {
        type: 'application',
        'bom-ref': 'pkg:github/clay-good/geoprims@main',
        name: 'geoprims',
        version: 'main',
        licenses: [{ license: { id: 'MIT' } }],
        purl: 'pkg:github/clay-good/geoprims@main',
      },
      properties: [
        { name: 'geoprims:source:npm', value: 'apps/web/package-lock.json' },
        { name: 'geoprims:source:cargo', value: 'core/Cargo.lock' },
      ],
    },
    components,
  };
}

export function generateSbom() {
  const npmLock = JSON.parse(readFileSync(resolve(root, 'apps/web/package-lock.json'), 'utf8'));
  const cargoMetadata = JSON.parse(execFileSync('cargo', [
    'metadata', '--manifest-path', resolve(root, 'core/Cargo.toml'), '--locked', '--format-version', '1',
  ], { encoding: 'utf8' }));
  return buildSbom({ npmLock, cargoMetadata });
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const text = `${JSON.stringify(generateSbom(), null, 2)}\n`;
  if (process.argv.includes('--check')) {
    if (readFileSync(output, 'utf8') !== text) {
      console.error('data/sbom.cdx.json is stale; run npm run build:sbom');
      process.exitCode = 1;
    } else {
      console.log('SBOM matches the npm and Cargo lockfiles');
    }
  } else {
    writeFileSync(output, text);
    console.log(`wrote ${output}`);
  }
}
