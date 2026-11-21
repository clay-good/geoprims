#!/usr/bin/env node
// Reads the Geodetic TIFF Grids the issuers and PROJ-data publish and packs
// them into the two grid packages the core reads: `HGRID`/`HGRIDS`
// (gp_geo::hgrid), latitude and longitude offsets in arcseconds, and `VGRID`
// (gp_geo::vgrid), one value in metres. Nothing is resampled or rounded: the
// nodes keep the float32 values the issuer published.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/** A GeoTIFF's header and node values, by GDAL. `path` may be a
 * `GTIFF_DIR:<page>:<file>` page of a multi-grid file. `bands` names the
 * bands to read, in order, and they must be the file's own names. */
export function readGeoTiff(path, bands) {
  const info = JSON.parse(execFileSync('gdalinfo', ['-json', path], { encoding: 'utf8' }));
  const [x0, dx, , y0, , dy] = info.geoTransform;
  const [cols, rows] = info.size;
  const names = info.bands.map((b) => b.description);
  for (const [i, want] of bands.entries()) {
    if (names[i] !== want) throw new Error(`${path}: band ${i + 1} is ${names[i]}, expected ${want}`);
  }
  if (info.metadata?.['']?.AREA_OR_POINT !== 'Point') throw new Error(`${path}: not a pixel-is-point grid`);
  const lon = info.bands[bands.indexOf('longitude_offset')];
  if (lon && lon.metadata?.['']?.positive_value !== 'east') throw new Error(`${path}: longitude offsets are not east positive`);
  // GDAL always reports the geotransform at the outer corner of the corner
  // pixel, shifting a pixel-is-point file's origin by half a cell to do so,
  // so the nodes are always half a cell in from it.
  const half = 0.5;
  const dir = mkdtempSync(join(tmpdir(), 'grid-'));
  const args = ['-q', '-of', 'ENVI'];
  for (let b = 1; b <= bands.length; b++) args.push('-b', String(b));
  execFileSync('gdal_translate', [...args, path, join(dir, 'g.bin')]);
  const raw = readFileSync(join(dir, 'g.bin'));
  if (raw.length !== cols * rows * bands.length * 4) throw new Error(`${path}: unexpected raw size`);
  const band = (b, r, c) => raw.readFloatLE(((b * rows) + r) * cols * 4 + c * 4);
  return {
    west: x0 + half * dx,
    north: y0 + half * dy,
    dlon: dx,
    dlat: -dy,
    cols,
    rows,
    // Row 0 of the TIFF is the north row.
    at: (northRow, col) => bands.map((_, b) => band(b, northRow, col)),
  };
}

/** A grid's header line, in degrees at the nodes, south-west corner first. */
function header(kind, g) {
  const south = g.north - (g.rows - 1) * g.dlat;
  const r = (v) => Number(v.toFixed(9));
  return Buffer.from(`${kind} ${r(g.west)} ${r(south)} ${r(g.dlon)} ${r(g.dlat)} ${g.cols} ${g.rows}\n`);
}

/** Node values south row first and west to east, `values` floats per node. */
function body(g, values) {
  const out = Buffer.alloc(g.cols * g.rows * values * 4);
  let i = 0;
  for (let row = g.rows - 1; row >= 0; row--) {
    for (let col = 0; col < g.cols; col++) {
      const v = g.at(row, col);
      for (let k = 0; k < values; k++) {
        out.writeFloatLE(v[k], i);
        i += 4;
      }
    }
  }
  return out;
}

/** One horizontal-offset grid: latitude and longitude offsets in arcseconds. */
export function packHgrid(g) {
  return Buffer.concat([header('HGRID', g), body(g, 2)]);
}

/** Several horizontal-offset grids in one package, coarsest first. */
export function packHgrids(grids) {
  return Buffer.concat([
    Buffer.from(`HGRIDS ${grids.length}\n`),
    ...grids.map((g) => packHgrid(g)),
  ]);
}

/** One vertical grid: a single value in metres at each node. */
export function packVgrid(g) {
  return Buffer.concat([header('VGRID', g), body(g, 1)]);
}
