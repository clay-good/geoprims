#!/usr/bin/env node
// Rebuilds the magnetic coefficient assets from pinned issuer downloads.
// Usage:
//   node tools/data/magnetic-assets.mjs WMM2025COF.zip WMMHR2025COF.zip igrf14coeffs.txt
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const sources = {
  wmm2025: {
    url: 'https://www.ncei.noaa.gov/sites/default/files/2024-12/WMM2025COF.zip',
    archiveSha256: '2e76569370d081f2cd7919490218bd094ca9afde347b198eff5621e0af460d03',
    coefficient: {
      member: 'WMM2025COF/WMM2025.COF',
      sha256: 'dfa8597825af4e0b87ff4198a5b4fb661b3c49f4cd090cd0164e0259b075582f',
      bytes: 4554,
      output: 'core/crates/gp-geo/data/WMM2025.COF',
      degree: 12,
    },
    tests: {
      member: 'WMM2025COF/WMM2025_TestValues.txt',
      sha256: 'e6975b093dddeb6153e0b23cc418425c438167e7c5b1dd795da379cb654f5819',
      bytes: 21774,
      output: 'core/crates/gp-geodesy/tests/data/WMM2025_TestValues.txt',
      rows: 100,
    },
  },
  wmmhr2025: {
    url: 'https://www.ncei.noaa.gov/sites/default/files/2025-01/WMMHR2025COF.zip',
    archiveSha256: '29a1112fc5594b859a00c258a2a639f5f19f8743e39efc94fd9ce4d85ec4b597',
    coefficient: {
      member: 'WMMHR2025COF/WMMHR.COF',
      sha256: '8851d40e57a1d948cb56d49b837612844890a941f93a73846a122b6c1182d504',
      bytes: 533743,
      output: 'assets/data/wmmhr2025/2025.0/WMMHR.COF',
      degree: 133,
    },
    tests: {
      member: 'WMMHR2025COF/WMMHR2025_TEST_VALUES.txt',
      sha256: 'e2cce79cc444ecf1655166c18c1252a841162b7fa4ea240fe94ee6fb8bcdc9de',
      bytes: 2652,
      output: 'core/crates/gp-geodesy/tests/data/WMMHR2025_TEST_VALUES.txt',
      rows: 12,
    },
  },
  igrf14: {
    url: 'https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf14coeffs.txt',
    sha256: '8f8d88403028fc4ee92c4f38d97b46e0a87e2cfc496045b43c9e26c1d6b0903c',
    bytes: 42411,
    output: 'core/crates/gp-geo/data/igrf14coeffs.txt',
  },
};

const check = (bytes, expected, label) => {
  if (bytes.length !== expected.bytes) throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  const got = sha256(bytes);
  if (got !== expected.sha256) throw new Error(`${label}: sha256 ${got}, expected ${expected.sha256}`);
};

export function validateCof(bytes, degree, label) {
  const lines = bytes.toString('utf8').split(/\r?\n/);
  if (!/^\s*2025\.0\s+WMM(?:HR)?-?2025/i.test(lines[0])) throw new Error(`${label}: bad header`);
  const rows = lines.slice(1).filter((line) => line.trim() && !line.startsWith('9999'));
  const expected = degree * (degree + 3) / 2;
  if (rows.length !== expected) throw new Error(`${label}: ${rows.length} coefficients, expected ${expected}`);
  const last = rows.at(-1).trim().split(/\s+/).slice(0, 2).map(Number);
  if (last[0] !== degree || last[1] !== degree) throw new Error(`${label}: last coefficient is ${last.join(',')}`);
  if (lines.filter((line) => line.startsWith('9999')).length !== 2) throw new Error(`${label}: needs two terminators`);
}

export function validateTestValues(bytes, rows, label) {
  const got = bytes.toString('utf8').split(/\r?\n/).filter((line) => line.trim() && !line.startsWith('#'));
  if (got.length !== rows) throw new Error(`${label}: ${got.length} test rows, expected ${rows}`);
  for (const [i, line] of got.entries()) {
    if (line.trim().split(/\s+/).length < 18) throw new Error(`${label}: row ${i + 1} is incomplete`);
  }
}

function extract(archive, member) {
  return execFileSync('unzip', ['-p', archive, member], { maxBuffer: 2_000_000 });
}

function write(output, bytes) {
  const path = join(root, output);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, bytes);
}

export function build(wmmArchive, wmmhrArchive, igrfFile) {
  for (const [name, archive, source] of [
    ['WMM2025', wmmArchive, sources.wmm2025],
    ['WMMHR2025', wmmhrArchive, sources.wmmhr2025],
  ]) {
    const archiveBytes = readFileSync(archive);
    const digest = sha256(archiveBytes);
    if (digest !== source.archiveSha256) throw new Error(`${name} archive: sha256 ${digest}, expected ${source.archiveSha256}`);
    const coefficient = extract(archive, source.coefficient.member);
    const tests = extract(archive, source.tests.member);
    check(coefficient, source.coefficient, `${name} coefficients`);
    check(tests, source.tests, `${name} test values`);
    validateCof(coefficient, source.coefficient.degree, `${name} coefficients`);
    validateTestValues(tests, source.tests.rows, `${name} test values`);
    write(source.coefficient.output, coefficient);
    write(source.tests.output, tests);
  }
  const igrf = readFileSync(igrfFile);
  check(igrf, sources.igrf14, 'IGRF-14 coefficients');
  if (!igrf.toString('utf8').startsWith('# 14th Generation International Geomagnetic Reference Field')) {
    throw new Error('IGRF-14 coefficients: bad header');
  }
  write(sources.igrf14.output, igrf);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 5) throw new Error('usage: magnetic-assets.mjs WMM2025COF.zip WMMHR2025COF.zip igrf14coeffs.txt');
  build(...process.argv.slice(2));
  console.log('magnetic assets: WMM2025, WMMHR2025, and IGRF-14 verified and written');
}
