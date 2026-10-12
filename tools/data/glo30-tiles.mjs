#!/usr/bin/env node
// Validates Copernicus DEM GLO-30 DGED tiles and rewrites them as deterministic,
// lossless float32 COGs. The signed index is the integrity and coverage contract
// consumed by the web and Node asset providers.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { signIndex } from './geoid-tiles.mjs';

export const GLO30 = Object.freeze({
  assetId: 'dem-glo30',
  version: '2024_1',
  title: 'Copernicus DEM GLO-30, DGED',
  sourceUrl: 'https://dataspace.copernicus.eu/explore-data/data-collections/copernicus-contributing-missions/collections-description/COP-DEM',
  licenseUrl: 'https://documentation.dataspace.copernicus.eu/APIs/SentinelHub/Data/DEM/resources/license/License-COPDEM-30.pdf',
  attribution: 'produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved',
  liability: 'The organisations in charge of the Copernicus programme by law or by delegation do not incur any liability for any use of the Copernicus WorldDEM-30.',
  horizontalCrs: 'EPSG:4326 (WGS 84-G1150)',
  verticalDatum: 'EPSG:3855 (EGM2008 height)',
  latitudeSpacingDegrees: 1 / 3600,
  absoluteVerticalAccuracy: '< 4 m at 90% confidence, excluding Antarctica and Greenland; local deviations can occur',
});

const NAME = /^Copernicus_DSM_COG_10_([NS])(\d{2})_00_([EW])(\d{3})_00_DEM\.tif$/;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const close = (a, b, tolerance = 1e-10) => Math.abs(a - b) <= tolerance;

export function coordinatesFromName(file) {
  const match = NAME.exec(basename(file));
  if (!match) throw new Error(`${basename(file)} is not a Copernicus GLO-30 DGED elevation tile name`);
  const south = Number(match[2]) * (match[1] === 'S' ? -1 : 1);
  const west = Number(match[4]) * (match[3] === 'W' ? -1 : 1);
  if (south < -90 || south >= 90 || west < -180 || west >= 180) throw new Error(`${basename(file)} has coordinates outside the world grid`);
  return { south, west, north: south + 1, east: west + 1 };
}

export function outputName(bounds) {
  const latitude = `${bounds.south < 0 ? 's' : 'n'}${String(Math.abs(bounds.south)).padStart(2, '0')}`;
  const longitude = `${bounds.west < 0 ? 'w' : 'e'}${String(Math.abs(bounds.west)).padStart(3, '0')}`;
  return `${latitude}-${longitude}.tif`;
}

function gdalInfo(file, command = 'gdalinfo') {
  const stdout = execFileSync(command, ['-json', '-checksum', file], { maxBuffer: 8 * 1024 * 1024 });
  return JSON.parse(stdout);
}

export function validateTile(file, info, options = {}) {
  const bounds = coordinatesFromName(file);
  const expectedLatitudeSpacing = options.expectedLatitudeSpacing ?? GLO30.latitudeSpacingDegrees;
  if (info.driverShortName !== 'GTiff') throw new Error(`${basename(file)} is not a GeoTIFF`);
  if (info.stac?.['proj:epsg'] !== 4326) throw new Error(`${basename(file)} is not EPSG:4326`);
  if (info.metadata?.['']?.AREA_OR_POINT !== 'Point') throw new Error(`${basename(file)} is not RasterPixelIsPoint`);
  if (info.bands?.length !== 1 || info.bands[0].type !== 'Float32') throw new Error(`${basename(file)} is not a one-band float32 DGED tile`);
  if (!Number.isSafeInteger(info.size?.[0]) || !Number.isSafeInteger(info.size?.[1]) || info.size[0] < 2 || info.size[1] < 2) {
    throw new Error(`${basename(file)} has an invalid raster size`);
  }
  const transform = info.geoTransform;
  const longitudeSpacing = transform?.[1];
  const latitudeSpacing = -transform?.[5];
  // RasterPixelIsPoint puts the named west/north coordinates at the center of
  // the first sample; GDAL's affine transform describes that pixel's outer edge.
  if (!Array.isArray(transform) || transform.length !== 6 || !close(transform[0] + longitudeSpacing / 2, bounds.west)
    || !close(transform[3] - latitudeSpacing / 2, bounds.north) || !close(transform[2], 0) || !close(transform[4], 0)
    || longitudeSpacing <= 0 || latitudeSpacing <= 0 || !close(longitudeSpacing * info.size[0], 1)
    || !close(latitudeSpacing * info.size[1], 1) || !close(latitudeSpacing, expectedLatitudeSpacing)) {
    throw new Error(`${basename(file)} is not the expected one-degree GLO-30 grid`);
  }
  return {
    ...bounds,
    width: info.size[0],
    height: info.size[1],
    longitudeSpacingDegrees: longitudeSpacing,
    latitudeSpacingDegrees: latitudeSpacing,
    checksum: info.bands[0].checksum,
  };
}

function translate(source, output, command = 'gdal_translate') {
  execFileSync(command, [
    '-q', '-of', 'COG', '-ot', 'Float32',
    '-co', 'COMPRESS=DEFLATE',
    '-co', 'PREDICTOR=3',
    '-co', 'BLOCKSIZE=512',
    '-co', 'OVERVIEWS=NONE',
    '-co', 'NUM_THREADS=1',
    '-co', 'BIGTIFF=IF_SAFER',
    source, output,
  ]);
}

export function buildTiles(sourceFiles, outputDirectory, options = {}) {
  if (!sourceFiles.length) throw new Error('no Copernicus GLO-30 DGED elevation tiles were found');
  mkdirSync(outputDirectory, { recursive: true });
  const tiles = [];
  const names = new Set();
  for (const source of [...sourceFiles].sort()) {
    const sourceInfo = gdalInfo(source, options.gdalInfo);
    const grid = validateTile(source, sourceInfo, options);
    const file = outputName(grid);
    if (names.has(file)) throw new Error(`duplicate Copernicus tile ${file}`);
    names.add(file);
    const output = join(outputDirectory, file);
    translate(source, output, options.gdalTranslate);
    const outputInfo = gdalInfo(output, options.gdalInfo);
    const rewritten = validateTile(source, outputInfo, options);
    if (rewritten.checksum !== grid.checksum || rewritten.width !== grid.width || rewritten.height !== grid.height) {
      throw new Error(`${basename(source)} changed elevation samples during COG conversion`);
    }
    const structure = outputInfo.metadata?.IMAGE_STRUCTURE ?? {};
    if (structure.LAYOUT !== 'COG' || structure.COMPRESSION !== 'DEFLATE' || structure.PREDICTOR !== '3') {
      throw new Error(`${file} is not a Deflate float-predictor COG`);
    }
    const bytes = readFileSync(output);
    tiles.push({
      file,
      bytes: bytes.length,
      sha256: sha256(bytes),
      sourceFile: basename(source),
      sourceBytes: statSync(source).size,
      sourceSha256: sha256(readFileSync(source)),
      bounds: [grid.south, grid.west, grid.north, grid.east],
      width: grid.width,
      height: grid.height,
      sampleChecksum: grid.checksum,
      longitudeSpacingDegrees: grid.longitudeSpacingDegrees,
      latitudeSpacingDegrees: grid.latitudeSpacingDegrees,
    });
  }
  return {
    schemaVersion: 1,
    assetId: GLO30.assetId,
    version: options.version ?? GLO30.version,
    source: { title: GLO30.title, url: GLO30.sourceUrl, release: options.version ?? GLO30.version },
    format: { driver: 'COG', dataType: 'float32', compression: 'DEFLATE', predictor: 3, pixelInterpretation: 'point' },
    horizontalCrs: GLO30.horizontalCrs,
    verticalDatum: GLO30.verticalDatum,
    surfaceModel: true,
    accuracy: GLO30.absoluteVerticalAccuracy,
    attribution: GLO30.attribution,
    liability: GLO30.liability,
    tiling: { scheme: 'one-degree latitude-longitude', minimumDegrees: 1 },
    tiles,
  };
}

export function tileAt(index, latitude, longitude) {
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude) || latitude < -90 || latitude > 90) return null;
  const lat = latitude === 90 ? 90 - Number.EPSILON : latitude;
  const lon = ((longitude + 180) % 360 + 360) % 360 - 180;
  const south = Math.floor(lat);
  const west = Math.floor(lon);
  return index.tiles.find((tile) => tile.bounds[0] === south && tile.bounds[1] === west) ?? null;
}

function findSources(directory) {
  const found = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) found.push(...findSources(path));
    else if (entry.isFile() && NAME.test(entry.name)) found.push(path);
  }
  return found;
}

export function build(sourceDirectory, outputDirectory, privateKeyFile, keyId) {
  const index = buildTiles(findSources(sourceDirectory), outputDirectory);
  const signed = signIndex(index, readFileSync(privateKeyFile), keyId);
  writeFileSync(join(outputDirectory, 'index.json'), `${JSON.stringify(signed, null, 2)}\n`);
  return signed;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 6) throw new Error('usage: glo30-tiles.mjs SOURCE_DIR OUTPUT_DIR PRIVATE_KEY.pem KEY_ID');
  const signed = build(...process.argv.slice(2));
  console.log(`${signed.index.assetId}: ${signed.index.tiles.length} signed tiles written to ${process.argv[3]}`);
}
