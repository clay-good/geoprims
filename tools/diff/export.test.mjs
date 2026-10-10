// The mission export, read back by an independent reader: GDAL's ogrinfo
// opens each KML and GeoJSON file the tool writes and must find the same
// waypoints, in order, longitude before latitude, with the heights the tool's
// rule for the height reference gives. Skipped where GDAL is not installed.
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { rng } from './runner.mjs';

const root = new URL('../..', import.meta.url).pathname;
const host = nodeHost(join(root, 'dist/wasm'));
const installed = !spawnSync('ogrinfo', ['--version'], { stdio: 'ignore' }).error;

/** The points GDAL reads from a file: [lon, lat, z] in file order. */
function read(path) {
  const out = spawnSync('ogrinfo', ['-al', '-q', path], { encoding: 'utf8' });
  assert.equal(out.status, 0, out.stderr);
  return [...out.stdout.matchAll(/^\s*POINT Z? ?\(([^)]+)\)/gm)].map((m) => m[1].split(' ').map(Number));
}

test('GDAL reads back every exported waypoint where the tool put it', { skip: !installed && 'ogrinfo (GDAL) is not installed' }, async () => {
  const dir = mkdtempSync(join(tmpdir(), 'gp-export-'));
  const r = rng(20261010);
  const round = (x, d) => Number(x.toFixed(d));
  let files = 0;
  try {
    for (let c = 0; c < 16; c++) {
      const n = 2 + Math.floor(r() * 6);
      // Both hemispheres and both sides of the prime meridian and antimeridian.
      const waypoints = Array.from({ length: n }, () => ({ lat: round(-85 + r() * 170, 6), lon: round(-179.9 + r() * 359.8, 6), height: `${round(5 + r() * 395, 1)} m` }));
      const [reference, extra, offset] = [
        ['agl', {}, 0],
        ['msl', {}, 0],
        ['takeoff', { takeoff_elevation: '312.5 m' }, 312.5],
        ['hae', { geoid_height: '-33.25 m' }, 33.25],
      ][c % 4];
      for (const format of ['kml', 'geojson']) {
        const env = JSON.parse(await host.invoke('drone.mission.export', JSON.stringify({ waypoints, height_reference: reference, format, ...extra })));
        assert.equal(env.ok, true, JSON.stringify(env.error));
        const path = join(dir, `case${c}.${format}`);
        writeFileSync(path, env.result.file);
        files++;
        const got = read(path);
        assert.equal(got.length, n, `${reference} ${format}: ${got.length} points for ${n} waypoints`);
        for (const [i, w] of waypoints.entries()) {
          const h = Number.parseFloat(w.height);
          // KML carries sea-level heights for takeoff and ellipsoid references
          // (absolute mode); GeoJSON keeps the height as entered, with its
          // reference named in the properties.
          const z = format === 'kml' ? h + offset : h;
          assert.ok(Math.abs(got[i][0] - w.lon) < 1e-9 && Math.abs(got[i][1] - w.lat) < 1e-9, `${reference} ${format} point ${i + 1}: ${got[i]} for ${w.lon}, ${w.lat}`);
          assert.ok(Math.abs(got[i][2] - z) < 1e-6, `${reference} ${format} point ${i + 1}: height ${got[i][2]} for ${z}`);
        }
      }
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  assert.equal(files, 32);
});
