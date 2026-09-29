#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const VERSION = '2026-08-27';
const NGS_VERSION = '2026-06-01';
const EPSG_VERSION = 'v13.102';
const NGS_INPUTS = {
  definitions: { bytes: 632927, sha256: 'f222dac669503c8e25eb41d477bbb129b813b894b43e7d012effb9dc00bbc06a' },
  bounds: { bytes: 654390, sha256: '040f9d5a6e4af2587cb8306d05829a0efefd17a482b37f55678e4ea861f48b66' },
  coordinates: { bytes: 810931, sha256: '90ed9bff3d2395f146e6533d7805e234e736193d09906372852df40abaf457a3' },
};

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const number = (value) => Number(String(value).replaceAll(',', ''));

export function angle(value) {
  const match = String(value).match(/^(\d+)°(\d+)'(?:(\d+(?:\.\d+)?)")?([NSEW])$/);
  if (!match) throw new Error(`invalid NGS angle: ${value}`);
  const result = Number(match[1]) + Number(match[2]) / 60 + Number(match[3] ?? 0) / 3600;
  return 'SW'.includes(match[4]) ? -result : result;
}

function signedAngle(value) {
  const match = String(value).trim().match(/^([+-]?)(\d+)°(\d+)'(\d+(?:\.\d+)?)"$/);
  if (!match) throw new Error(`invalid signed NGS angle: ${value}`);
  const result = Number(match[2]) + Number(match[3]) / 60 + Number(match[4]) / 3600;
  return match[1] === '-' ? -result : result;
}

export function normalizeSpcs2022(definitions, bounds) {
  const boundsByCode = new Map(bounds.map((row) => [row['Zone code'], row]));
  if (boundsByCode.size !== bounds.length) throw new Error('SPCS2022 bounds repeat a zone code');
  const zones = definitions.map((row) => {
    const code = row['Zone code'];
    const area = boundsByCode.get(code);
    if (!area) throw new Error(`SPCS2022 zone ${code} has no bounds`);
    boundsByCode.delete(code);
    const definition = {
      projection: row['Proj type'],
      originLatitude: angle(row['Origin latitude']),
      originLongitude: angle(row['Origin longitude west']),
      originScale: number(row['Projection origin scale']),
      falseEastingMeters: number(row['False easting (m)']),
      falseNorthingMeters: number(row['False northing (m)']),
    };
    if (row['Skew azimuth (deg)']) definition.skewAzimuthDegrees = number(row['Skew azimuth (deg)']);
    return {
      id: `NGS:SPCS2022:${code}`,
      name: `${row['Zone name']} (${row['Zone abrv']})`,
      type: 'ProjectedCRS',
      status: 'beta',
      publishedAt: NGS_VERSION,
      referenceFrame: row['Reference frame'],
      zoneType: row['Zone type'],
      designedBy: row['Design by'],
      bounds: [number(area['Min lon west (deg)']), number(area['Min lat (deg)']), number(area['Max lon west (deg)']), number(area['Max lat (deg)'])],
      definition,
    };
  }).sort((a, b) => a.id.localeCompare(b.id));
  if (boundsByCode.size) throw new Error(`${boundsByCode.size} SPCS2022 bounds have no definition`);
  if (new Set(zones.map((zone) => zone.id)).size !== zones.length) throw new Error('SPCS2022 repeats a zone code');
  return zones;
}

function verifyInput(path, expected) {
  const bytes = readFileSync(path);
  if (bytes.length !== expected.bytes || sha256(bytes) !== expected.sha256) {
    throw new Error(`${path} does not match the pinned NGS source`);
  }
  return JSON.parse(bytes);
}

function spcs83Codes() {
  const source = readFileSync(join(ROOT, 'core/crates/gp-geo/src/spcs83_zones.rs'), 'utf8');
  const codes = [...source.matchAll(/epsg: (\d+)/g)].map((match) => Number(match[1]));
  if (codes.length !== 124) throw new Error(`expected 124 SPCS83 zones, got ${codes.length}`);
  return codes;
}

export function requiredEpsgCodes() {
  const direct = [3857, 4087, 4269, 4326, 6319, 7912, 9988];
  const utm = Array.from({ length: 60 }, (_, index) => 32601 + index)
    .concat(Array.from({ length: 60 }, (_, index) => 32701 + index));
  return [...new Set([...direct, ...utm, 32661, 32761, ...spcs83Codes()])].sort((a, b) => a - b);
}

function epsgMetadata() {
  const searchPaths = execFileSync('projinfo', ['--searchpaths'], { encoding: 'utf8' }).trim().split('\n');
  const database = join(searchPaths.at(-1), 'proj.db');
  const rows = execFileSync('sqlite3', [database, 'select key||"="||value from metadata;'], { encoding: 'utf8' });
  const metadata = Object.fromEntries(rows.trim().split('\n').map((row) => row.split('=')));
  if (metadata['EPSG.VERSION'] !== EPSG_VERSION || metadata['EPSG.DATE'] !== VERSION) {
    throw new Error(`need EPSG ${EPSG_VERSION} (${VERSION}); found ${metadata['EPSG.VERSION']} (${metadata['EPSG.DATE']})`);
  }
  return { version: metadata['EPSG.VERSION'], publishedAt: metadata['EPSG.DATE'], projVersion: metadata['PROJ.VERSION'] };
}

function epsgEntry(code) {
  const output = execFileSync('projinfo', ['-q', `EPSG:${code}`, '-o', 'PROJJSON', '--single-line'], { encoding: 'utf8' }).trim();
  const definition = JSON.parse(output);
  const box = definition.bbox;
  if (!box) throw new Error(`EPSG:${code} has no bounds`);
  return {
    id: `EPSG:${code}`,
    name: definition.name,
    type: definition.type,
    status: 'official',
    bounds: [box.west_longitude, box.south_latitude, box.east_longitude, box.north_latitude],
    definition,
  };
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value)}\n`);
  const bytes = readFileSync(path);
  console.error(`wrote ${path} (${bytes.length} bytes, ${sha256(bytes)})`);
}

function writeChecks(path, rows) {
  if (rows.length !== 953 || new Set(rows.map((row) => row['Zone code'])).size !== 953) {
    throw new Error(`expected 953 unique SPCS2022 checks, got ${rows.length}`);
  }
  const lines = ['code,lat,lon,easting,northing,scale,convergence'];
  for (const row of rows.sort((a, b) => a['Zone code'].localeCompare(b['Zone code']))) {
    lines.push([
      row['Zone code'], number(row['Latitude (deg)']), number(row['Longitude west (deg)']),
      number(row['Easting (m)']), number(row['Northing (m)']), number(row['Point scale factor']),
      signedAngle(row['Convergence angle']),
    ].join(','));
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${lines.join('\n')}\n`);
  const bytes = readFileSync(path);
  console.error(`wrote ${path} (${bytes.length} bytes, ${sha256(bytes)})`);
}

function wrapLongitude(value) {
  return ((value + 180) % 360 + 360) % 360 - 180;
}

function writeVectors(path, rows, inverse) {
  const picked = Array.from({ length: 20 }, (_, index) => rows[Math.round(index * (rows.length - 1) / 19)]);
  const vectors = picked.map((row, index) => {
    const code = row['Zone code'];
    const lat = number(row['Latitude (deg)']);
    const lon = wrapLongitude(number(row['Longitude west (deg)']));
    const easting = number(row['Easting (m)']);
    const northing = number(row['Northing (m)']);
    return JSON.stringify({
      id: `v${String(index + 1).padStart(3, '0')}`,
      input: inverse ? { zone: code, easting, northing } : { lat, lon, zone: code },
      expect: inverse ? {
        ok: true,
        'result.lat.value': lat,
        'result.lon.value': lon,
        'result.zone_code': code,
        'result.status': 'beta',
        'meta.warnings.*.code': 'NON_OFFICIAL_DATUM',
      } : {
        ok: true,
        'result.easting.value': easting,
        'result.northing.value': northing,
        'result.scale_factor': number(row['Point scale factor']),
        'result.convergence.value': signedAngle(row['Convergence angle']),
        'result.zone_code': code,
        'result.status': 'beta',
        'meta.warnings.*.code': 'NON_OFFICIAL_DATUM',
      },
      source: 'NGS SPCS2022 Example Coordinates and Distortion Values',
      sourceVersion: 'beta, 2026-06-01',
      tolerance: inverse ? {
        'result.lat.value': { abs: 1e-8 },
        'result.lon.value': { abs: 1e-8 },
      } : {
        'result.easting.value': { abs: 0.001 },
        'result.northing.value': { abs: 0.001 },
        'result.scale_factor': { abs: 2.1e-9 },
        'result.convergence.value': { abs: 2.78e-6 },
      },
    });
  });
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${vectors.join('\n')}\n`);
  const bytes = readFileSync(path);
  console.error(`wrote ${path} (${bytes.length} bytes, ${sha256(bytes)})`);
}

function main() {
  const [definitionsPath, boundsPath, coordinatesPath] = process.argv.slice(2);
  if (!definitionsPath || !boundsPath || !coordinatesPath) {
    throw new Error('usage: node tools/data/crs-registry.mjs zoneDefinitions.json zoneBounds.json coordinates.json');
  }
  const zones = normalizeSpcs2022(
    verifyInput(definitionsPath, NGS_INPUTS.definitions),
    verifyInput(boundsPath, NGS_INPUTS.bounds),
  );
  if (zones.length !== 953) throw new Error(`expected 953 SPCS2022 zones, got ${zones.length}`);
  const epsg = epsgMetadata();
  const stable = requiredEpsgCodes().map(epsgEntry);
  const spcsAsset = {
    schemaVersion: 1,
    source: { authority: 'NGS', status: 'beta', publishedAt: NGS_VERSION },
    zones,
  };
  const registryAsset = {
    schemaVersion: 1,
    sources: { EPSG: epsg, SPCS2022: spcsAsset.source },
    crs: [...stable, ...zones],
  };
  writeJson(join(ROOT, 'assets/data/spcs2022-beta', NGS_VERSION, 'spcs2022-beta.json'), spcsAsset);
  writeJson(join(ROOT, 'assets/data/crs-registry', VERSION, 'crs-registry.json'), registryAsset);
  const coordinates = verifyInput(coordinatesPath, NGS_INPUTS.coordinates);
  writeChecks(
    join(ROOT, 'core/crates/gp-geodesy/tests/data/spcs2022_checks.csv'),
    coordinates,
  );
  writeVectors(join(ROOT, 'core/vectors/geodesy.spcs.spcs2022-forward.jsonl'), coordinates, false);
  writeVectors(join(ROOT, 'core/vectors/geodesy.spcs.spcs2022-inverse.jsonl'), coordinates, true);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
