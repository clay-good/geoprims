import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { generateKeyPairSync } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { GLO30, buildTiles, coordinatesFromName, outputName, tileAt, validateTile } from './glo30-tiles.mjs';
import { signIndex, verifyIndex } from './geoid-tiles.mjs';

const hasGdal = spawnSync('gdalinfo', ['--version']).status === 0
  && spawnSync('gdal_create', ['--help'], { stdio: 'ignore' }).status === 0
  && spawnSync('gdal_translate', ['--version']).status === 0;

test('Copernicus names map to one-degree privacy tiles', () => {
  const northEast = coordinatesFromName('Copernicus_DSM_COG_10_N41_00_E000_00_DEM.tif');
  assert.deepEqual(northEast, { south: 41, west: 0, north: 42, east: 1 });
  assert.equal(outputName(northEast), 'n41-e000.tif');
  const southWest = coordinatesFromName('Copernicus_DSM_COG_10_S07_00_W123_00_DEM.tif');
  assert.deepEqual(southWest, { south: -7, west: -123, north: -6, east: -122 });
  assert.equal(outputName(southWest), 's07-w123.tif');
  assert.throws(() => coordinatesFromName('N41E000.tif'), /not a Copernicus/);
});

test('the adapted-data notice and liability sentence match the official licence', () => {
  assert.equal(GLO30.attribution, 'produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved');
  assert.equal(GLO30.liability, 'The organisations in charge of the Copernicus programme by law or by delegation do not incur any liability for any use of the Copernicus WorldDEM-30.');
  assert.equal(GLO30.verticalDatum, 'EPSG:3855 (EGM2008 height)');
  assert.match(GLO30.absoluteVerticalAccuracy, /< 4 m.*90%/);
});

test('the re-tiler preserves float32 posts and emits signed, lossless COGs', { skip: !hasGdal && 'GDAL is not installed' }, () => {
  const root = mkdtempSync(join(tmpdir(), 'geoprims-glo30-'));
  try {
    const sourceDirectory = join(root, 'source');
    const first = join(sourceDirectory, 'Copernicus_DSM_COG_10_N41_00_E000_00_DEM.tif');
    const second = join(sourceDirectory, 'Copernicus_DSM_COG_10_N41_00_E001_00_DEM.tif');
    mkdirSync(sourceDirectory);
    for (const [file, west, value] of [[first, 0, 123.25], [second, 1, -8.5]]) {
      execFileSync('gdal_create', [
        '-q', '-of', 'GTiff', '-ot', 'Float32', '-outsize', '8', '8', '-burn', String(value),
        '-a_srs', 'EPSG:4326', '-a_ullr', String(west - 1 / 16), String(42 + 1 / 16), String(west + 1 - 1 / 16), String(41 + 1 / 16),
        '-mo', 'AREA_OR_POINT=Point', file,
      ]);
    }
    const output = join(root, 'output');
    const index = buildTiles([second, first], output, { expectedLatitudeSpacing: 1 / 8 });
    assert.equal(index.assetId, 'dem-glo30');
    assert.equal(index.tiles.length, 2);
    assert.deepEqual(index.tiles.map((tile) => tile.file), ['n41-e000.tif', 'n41-e001.tif']);
    assert.ok(index.tiles.every((tile) => tile.bounds[2] - tile.bounds[0] >= 1 && tile.bounds[3] - tile.bounds[1] >= 1));
    assert.equal(tileAt(index, 41.5, 0.5)?.file, 'n41-e000.tif');
    assert.equal(tileAt(index, 41.5, 2.5), null, 'missing coverage was silently filled');
    for (const tile of index.tiles) {
      const info = JSON.parse(execFileSync('gdalinfo', ['-json', '-checksum', join(output, tile.file)]));
      assert.equal(info.bands[0].checksum, tile.sampleChecksum);
      assert.equal(info.metadata.IMAGE_STRUCTURE.LAYOUT, 'COG');
      assert.equal(info.metadata.IMAGE_STRUCTURE.COMPRESSION, 'DEFLATE');
      assert.equal(info.metadata.IMAGE_STRUCTURE.PREDICTOR, '3');
      assert.equal(readFileSync(join(output, tile.file)).length, tile.bytes);
    }
    const { privateKey } = generateKeyPairSync('ed25519');
    const signed = signIndex(index, privateKey.export({ type: 'pkcs8', format: 'pem' }), 'glo30-test');
    assert.equal(verifyIndex(signed), true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('the source validator rejects altered product geometry and alignment', () => {
  const file = 'Copernicus_DSM_COG_10_N41_00_E000_00_DEM.tif';
  const valid = {
    driverShortName: 'GTiff',
    size: [3_600, 3_600],
    stac: { 'proj:epsg': 4326 },
    geoTransform: [-1 / 7_200, 1 / 3_600, 0, 42 + 1 / 7_200, 0, -1 / 3_600],
    metadata: { '': { AREA_OR_POINT: 'Point' } },
    bands: [{ type: 'Float32', checksum: 1 }],
  };
  assert.equal(validateTile(file, valid).height, 3_600);
  assert.throws(() => validateTile(file, { ...valid, size: [3_600, 1_800] }), /expected one-degree/);
  assert.throws(() => validateTile(file, { ...valid, metadata: { '': { AREA_OR_POINT: 'Area' } } }), /RasterPixelIsPoint/);
  assert.throws(() => validateTile(file, { ...valid, bands: [{ type: 'Int16' }] }), /float32/);
});
