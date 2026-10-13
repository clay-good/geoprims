// Every constant a tool uses that the caller does not supply is declared with
// its value, unit, and source (trust/citations, "Citation record per tool").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { metaschemaProblems } from './metaschema.mjs';

const root = join(new URL('.', import.meta.url).pathname, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const ledger = JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8')).sources;
const sourceIds = new Set(ledger.map((r) => r.id));
const byId = (id) => catalog.tools.find((t) => t.id === id);

/**
 * The tools whose answers rest on constants the caller never sees. The list
 * grows as constants are declared; it may not shrink, so a tool cannot quietly
 * stop saying what it assumed.
 */
const MUST_DECLARE = [
  'aviation.atmosphere.isa',
  'aviation.altimetry.pressure-altitude',
  'aviation.altimetry.density-altitude',
  'geodesy.utm.forward',
  'geodesy.utm.inverse',
  'geodesy.ups.forward',
  'geodesy.ups.inverse',
  'geodesy.grid-ref.mgrs-forward',
  'geodesy.grid-ref.mgrs-inverse',
  'geodesy.projection.web-mercator-forward',
  'geodesy.projection.web-mercator-inverse',
  'indexing.s2.covering',
  'indexing.s2.lat-lng-to-cell',
  'indexing.s2.cell-info',
  'navigation.route.cross-track',
  'geometry.area.polygon',
  'indexing.geohash.encode',
  'indexing.plus-code.encode',
  'drone.power.hover-power',
  'drone.power.max-payload',
  'drone.power.calibrate-hover',
  'aviation.airspeed.cas-to-tas',
  'aviation.airspeed.tas-to-cas',
  'aviation.altimetry.cold-temperature',
  'geodesy.magnetic.declination',
  'geodesy.magnetic.true-to-magnetic',
  'geodesy.magnetic.grivation',
  'time.sun.events',
  'indexing.tile.from-point',
  'indexing.tile.ground-resolution',
  'indexing.tile.cover',
  'aviation.altimetry.isa-temperature',
  'aviation.altimetry.true-altitude',
  'time.scale.julian-date',
  'aviation.performance.turn',
  'aviation.performance.top-of-descent',
  'aviation.performance.pivotal-altitude',
  'aviation.performance.vdp',
  'aviation.airspeed.tat-sat',
  'survey.reduction.edm-correction',
  'time.scale.gps-week',
  'time.scale.gps-to-utc',
  'aviation.atmosphere.humidity',
  'aviation.atmosphere.cloud-base',
  'raster.scale.reflectance',
  'raster.index.evi',
  'raster.index.evi2',
  'raster.index.savi',
  'raster.index.dnbr',
  'geometry.buffer.geodesic',
  'time.sun.position',
  'time.sun.mapping-window',
  'time.sun.hotspot',
  'drone.photogrammetry.gsd',
  'drone.photogrammetry.altitude-for-gsd',
  'drone.photogrammetry.trigger',
  'drone.photogrammetry.motion-blur',
  'drone.photogrammetry.image-count',
  'drone.mission.survey-grid',
  'drone.mission.orbit',
  'drone.mission.export',
  'drone.power.endurance',
  'drone.mission.facade',
  'drone.sensors.thermal-footprint',
  'drone.sensors.dataset-size',
  'drone.sensors.lidar-plan',
  'navigation.los.horizon',
  'navigation.los.visibility',
  'navigation.los.dip',
  'navigation.los.fresnel',
  'navigation.route.fly-by',
  'raster.terrain.line-of-sight',
  'survey.gnss.dop',
  'raster.terrain.slope-aspect',
  'raster.terrain.ruggedness',
  'raster.terrain.curvature',
  'survey.reduction.slope',
  'survey.reduction.curvature-refraction',
  'survey.reduction.inaccessible-height',
  'survey.reduction.stadia',
  'aviation.ifr.hold-entry',
  'aviation.ifr.hold-speed-limit',
  'aviation.ifr.hold-wind-timing',
  'aviation.ifr.dme-arc-lead',
  'aviation.ifr.time-to-station',
  'aviation.ifr.radial-intercept',
  'aviation.wind.one-in-sixty',
  'aviation.weather.metar-decode',
  'aviation.weather.taf-decode',
  'aviation.loading.fuel-plan',
  'drone.ops.part107-altitude',
  'drone.ops.speed-check',
  'drone.ops.kinetic-energy',
  'drone.ops.easa-subcategory',
  'drone.sensors.vlos',
  'drone.ops.vlos-check',
  'drone.ops.wind-limit',
  'drone.links.link-budget',
  'drone.photogrammetry.asprs-accuracy',
  'drone.photogrammetry.gcp-plan',
];

test('the density-altitude tool says which gas constant it used', () => {
  // The scenario: R = 287.05287 J/(kg·K), cited to ICAO Doc 7488.
  const t = byId('aviation.altimetry.density-altitude');
  const r = (t['x-assumptions'] ?? []).find((a) => /gas constant/i.test(a.name));
  assert.ok(r, 'no gas constant declared');
  assert.equal(r.value, '287.05287');
  assert.equal(r.unit, 'J/(kg K)');
  assert.equal(r.source, 'icao-7488');
  const row = ledger.find((x) => x.id === r.source);
  assert.match(row.name, /ICAO Standard Atmosphere/);
});

test('the humidity model is declared too, since it changes the answer', () => {
  const t = byId('aviation.altimetry.density-altitude');
  const magnus = (t['x-assumptions'] ?? []).filter((a) => a.source === 'alduchov');
  assert.equal(magnus.length, 3, 'the Magnus coefficients are not all declared');
  assert.deepEqual(magnus.map((a) => a.value).sort(), ['17.625', '243.04', '6.1094'].sort());
});

test('the cloud-base tool exposes every published rule and physical constant', () => {
  const assumptions = byId('aviation.atmosphere.cloud-base')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['400', 'ft/degC', 'faa-balloon-handbook'],
    ['56', 'K', 'bolton-1980'],
    ['800', 'K', 'bolton-1980'],
    ['9.80665', 'm/s2', 'icao-7488'],
    ['1004', 'J/(kg K)', 'nws-forecasters-reference'],
    ['1.98', 'degC/1,000 ft', 'icao-7488'],
  ]);
});

test('the vegetation indices expose every published coefficient and default', () => {
  const evi = byId('raster.index.evi')['x-assumptions'];
  assert.deepEqual(evi.map(({ value, unit, source }) => [value, unit, source]), [
    ['2.5', '1', 'huete-2002'],
    ['6', '1', 'huete-2002'],
    ['7.5', '1', 'huete-2002'],
    ['1', '1', 'huete-2002'],
  ]);

  const evi2 = byId('raster.index.evi2')['x-assumptions'];
  assert.deepEqual(evi2.map(({ value, unit, source }) => [value, unit, source]), [
    ['2.5', '1', 'jiang-2008'],
    ['2.4', '1', 'jiang-2008'],
    ['1', '1', 'jiang-2008'],
  ]);

  const savi = byId('raster.index.savi')['x-assumptions'];
  assert.deepEqual(savi.map(({ value, unit, source }) => [value, unit, source]), [
    ['0.5', '1', 'huete-1988'],
  ]);
});

test('the dNBR tool exposes its complete published classification', () => {
  const assumptions = byId('raster.index.dnbr')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['1000', '1', 'key-benson-2006'],
    ['-250', 'scaled dNBR', 'key-benson-2006'],
    ['-100', 'scaled dNBR', 'key-benson-2006'],
    ['100', 'scaled dNBR', 'key-benson-2006'],
    ['270', 'scaled dNBR', 'key-benson-2006'],
    ['440', 'scaled dNBR', 'key-benson-2006'],
    ['660', 'scaled dNBR', 'key-benson-2006'],
    ['-550', 'scaled dNBR', 'key-benson-2006'],
    ['1350', 'scaled dNBR', 'key-benson-2006'],
  ]);
});

test('the geodesic buffer exposes its published default mitre limit', () => {
  const assumptions = byId('geometry.buffer.geodesic')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['5', '1', 'jts-buffer-parameters'],
  ]);
});

test('the solar tools expose every default that affects their sun direction', () => {
  const position = byId('time.sun.position')['x-assumptions'];
  assert.deepEqual(position.map(({ value, unit, source }) => [value, unit, source]), [
    ['0', 'm', 'pvlib-solar-position'],
    ['1013.25', 'hPa', 'pvlib-solar-position'],
    ['12', 'degC', 'pvlib-solar-position'],
    ['0.5667', 'deg', 'nrel-spa'],
  ]);

  const assumptions = byId('time.sun.mapping-window')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['30', 'deg', 'geocue-sun-angle'],
    ['0', 'm', 'pvlib-solar-position'],
    ['1013.25', 'hPa', 'pvlib-solar-position'],
    ['12', 'degC', 'pvlib-solar-position'],
    ['0.5667', 'deg', 'nrel-spa'],
  ]);
});

test('the hotspot tool names both the camera and solar defaults', () => {
  const assumptions = byId('time.sun.hotspot')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['84', 'deg', 'dji-phantom-4-pro'],
    ['0', 'm', 'pvlib-solar-position'],
    ['1013.25', 'hPa', 'pvlib-solar-position'],
    ['12', 'degC', 'pvlib-solar-position'],
    ['0.5667', 'deg', 'nrel-spa'],
  ]);
});

test('the aviation-nights tool exposes its four regulatory boundaries', () => {
  const assumptions = byId('time.sun.aviation-nights')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['6', 'deg', 'cfr-14-1'],
    ['60', 'min', 'cfr-14-61'],
    ['0', 'min', 'cfr-14-91'],
    ['30', 'min', 'cfr-14-107'],
  ]);
});

test('the hover-power tool exposes its published coaxial interference default', () => {
  const assumptions = byId('drone.power.hover-power')['x-assumptions'];
  assert.ok(assumptions.some((a) =>
    a.value === '1.2818' && a.unit === '1' && a.source === 'leishman-syal-2008'
  ));
});

test('the stadia tool exposes both conventional instrument defaults', () => {
  const assumptions = byId('survey.reduction.stadia')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['100', '1', 'ghilani'],
    ['0', 'm', 'ghilani'],
  ]);
});

test('the photogrammetry defaults name their published values', () => {
  const gsd = byId('drone.photogrammetry.gsd')['x-assumptions'];
  assert.deepEqual(gsd.map(({ value, unit, source }) => [value, unit, source]), [
    ['36', 'mm', 'wolf-photogrammetry'],
    ['24', 'mm', 'wolf-photogrammetry'],
  ]);

  const altitude = byId('drone.photogrammetry.altitude-for-gsd')['x-assumptions'];
  assert.ok(altitude.some((a) => a.value === '400' && a.unit === 'ft' && a.source === 'cfr-14-107'));

  const trigger = byId('drone.photogrammetry.trigger')['x-assumptions'];
  assert.deepEqual(
    trigger.filter((a) => a.source === 'pix4d-overlap').map((a) => [a.value, a.unit]),
    [['75', '%'], ['60', '%'], ['85', '%'], ['85', '%']],
  );

  for (const id of ['drone.photogrammetry.image-count', 'drone.mission.survey-grid']) {
    const assumptions = byId(id)['x-assumptions'];
    assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
      ['2', 'photos per end', 'psu-geog892'],
    ], id);
  }
});

test('the endurance tool exposes every product-defined heuristic value', () => {
  const assumptions = byId('drone.power.endurance')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['20', 'degC', 'geoprims-endurance-spec'],
    ['20', '%', 'geoprims-endurance-spec'],
    ['1', '%/degC', 'geoprims-endurance-spec'],
    ['50', '%', 'geoprims-endurance-spec'],
    ['1', 'h', 'geoprims-endurance-spec'],
  ]);
});

test('the mission tools expose their product-defined operational defaults', () => {
  const orbit = byId('drone.mission.orbit')['x-assumptions'];
  assert.deepEqual(orbit.map(({ value, unit, source }) => [value, unit, source]), [
    ['36', 'photos', 'geoprims-mission-patterns-spec'],
  ]);

  const missionExport = byId('drone.mission.export')['x-assumptions'];
  assert.deepEqual(missionExport.map(({ value, unit, source }) => [value, unit, source]), [
    ['15', 'm', 'geoprims-mission-patterns-spec'],
  ]);
});

test('the photogrammetry tools expose their sourced planning defaults', () => {
  const motionBlur = byId('drone.photogrammetry.motion-blur')['x-assumptions'];
  assert.deepEqual(motionBlur.map(({ value, unit, source }) => [value, unit, source]), [
    ['0.5', 'px', 'grenzdoerffer-uav-image-motion-2012'],
  ]);
});

test('the facade and thermal defaults cite the guidance they apply', () => {
  const facade = byId('drone.mission.facade')['x-assumptions'];
  assert.deepEqual(facade.map(({ value, unit, source }) => [value, unit, source]), [
    ['75', '%', 'pix4d-overlap'],
    ['60', '%', 'pix4d-overlap'],
  ]);

  const thermal = byId('drone.sensors.thermal-footprint')['x-assumptions'];
  assert.deepEqual(thermal.map(({ value, unit, source }) => [value, unit, source]), [
    ['3', 'px', 'flir-3x3'],
  ]);
});

test('the sensing tools expose their published LAS and USGS tables', () => {
  const dataset = byId('drone.sensors.dataset-size')['x-assumptions'];
  assert.deepEqual(dataset.map(({ value, unit, source }) => [value, unit, source]), [
    ['375', 'bytes', 'asprs-las-1-4'],
    ['20, 28, 26, 34, 57, 63, 30, 36, 38, 59, 67', 'bytes', 'asprs-las-1-4'],
    ['6', '1', 'asprs-las-1-4'],
  ]);

  const lidar = byId('drone.sensors.lidar-plan')['x-assumptions'];
  assert.deepEqual(lidar.map(({ value, unit, source }) => [value, unit, source]), [
    ['8.0', 'pulses/m2', 'usgs-lbs'],
    ['2.0', 'pulses/m2', 'usgs-lbs'],
    ['0.5', 'pulses/m2', 'usgs-lbs'],
  ]);
});

test('the slope and hillshade tool exposes its geodetic and rendering defaults', () => {
  const assumptions = byId('raster.terrain.slope-aspect')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['6378137', 'm', 'nga-wgs84'],
    ['298.257223563', '1', 'nga-wgs84'],
    ['1', '1', 'gdal-hillshade'],
    ['315', 'deg', 'gdal-hillshade'],
    ['45', 'deg', 'gdal-hillshade'],
  ]);
});

test('the Fresnel tools expose every physical constant and clearance screen in their models', () => {
  const point = byId('navigation.los.fresnel')['x-assumptions'];
  assert.deepEqual(point.map(({ value, unit, source }) => [value, unit, source]), [
    ['6371000', 'm', 'grs80'],
    ['299792458', 'm/s', 'nist-sp-811'],
    ['4/3', '1', 'itu-r-p530'],
    ['60', '%', 'itu-r-p530'],
  ]);

  const terrain = byId('raster.terrain.line-of-sight')['x-assumptions'];
  assert.deepEqual(terrain.map(({ value, unit, source }) => [value, unit, source]), [
    ['6371000', 'm', 'grs80'],
    ['0.13', '1', 'esri-viewshed'],
    ['4/3', '1', 'itu-r-p530'],
    ['299792458', 'm/s', 'nist-sp-811'],
    ['60', '%', 'itu-r-p530'],
  ]);
});

test('every default optical refraction coefficient names its source', () => {
  const expected = new Map([
    ['navigation.los.horizon', 'esri-viewshed'],
    ['navigation.los.visibility', 'esri-viewshed'],
    ['navigation.los.dip', 'esri-viewshed'],
    ['raster.terrain.line-of-sight', 'esri-viewshed'],
    ['survey.reduction.slope', 'trimble-access'],
    ['survey.reduction.curvature-refraction', 'trimble-access'],
    ['survey.reduction.inaccessible-height', 'trimble-access'],
  ]);
  for (const [id, source] of expected) {
    const assumption = byId(id)['x-assumptions'].find((a) => a.value === '0.13' && a.unit === '1');
    assert.ok(assumption, `${id} does not declare its default k = 0.13`);
    assert.equal(assumption.source, source, id);
  }
});

test('the fly-by tool exposes its gravity and standard-rate defaults', () => {
  const assumptions = byId('navigation.route.fly-by')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['9.80665', 'm/s2', 'icao-7488'],
    ['3', 'deg/s', 'faa-ifh'],
  ]);
});

test('the DME arc tool exposes its published standard-rate and lead rule', () => {
  const assumptions = byId('aviation.ifr.dme-arc-lead')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['3', 'deg/s', 'faa-ifh'],
    ['60', 'deg', 'faa-ifh'],
  ]);
});

test('the time-to-station tool exposes its published 60-times rule', () => {
  const assumptions = byId('aviation.ifr.time-to-station')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['60', 'deg', 'faa-ifh'],
  ]);
});

test('the off-course tool exposes its published 1-in-60 factor', () => {
  const assumptions = byId('aviation.wind.one-in-sixty')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['60', 'deg', 'uk-mod-ap3456'],
  ]);
});

test('the radial-intercept tool exposes its published default technique', () => {
  const assumptions = byId('aviation.ifr.radial-intercept')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['2', '1', 'dubuque-instrument-sop'],
    ['20', 'deg', 'dubuque-instrument-sop'],
    ['90', 'deg', 'dubuque-instrument-sop'],
  ]);
});

test('the turn tool exposes gravity and its standard-rate default', () => {
  const assumptions = byId('aviation.performance.turn')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['9.80665', 'm/s2', 'icao-7488'],
    ['3', 'deg/s', 'faa-ifh'],
  ]);
});

test('the top-of-descent tool exposes its published planning rules', () => {
  const assumptions = byId('aviation.performance.top-of-descent')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['3', 'deg', 'faa-iph'],
    ['3', 'NM per 1,000 ft', 'faa-iph'],
    ['5', 'ft/min per kt', 'faa-iph'],
  ]);
});

test('the TAT-to-SAT tool exposes its gas and probe defaults', () => {
  const assumptions = byId('aviation.airspeed.tat-sat')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['1.4', '1', 'icao-7488'],
    ['1', '1', 'gracey'],
  ]);
});

test('the VDP tool exposes its published three-degree rule', () => {
  const assumptions = byId('aviation.performance.vdp')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['3', 'deg', 'faa-iph'],
    ['300', 'ft/NM', 'faa-iph'],
  ]);
});

test('the holding tools expose their published sectors, limits, and defaults', () => {
  const entry = byId('aviation.ifr.hold-entry')['x-assumptions'];
  assert.deepEqual(entry.map(({ value, unit, source }) => [value, unit, source]), [
    ['180', 'deg', 'aim'],
    ['70', 'deg', 'aim'],
    ['110', 'deg', 'aim'],
    ['30', 'deg', 'faa-ifh'],
    ['5', 'deg', 'icao-8168'],
  ]);

  const speed = byId('aviation.ifr.hold-speed-limit')['x-assumptions'];
  assert.deepEqual(speed.map(({ value, unit, source }) => [value, unit, source]), [
    ['6000', 'ft', 'aim'],
    ['14000', 'ft', 'aim'],
    ['200', 'KIAS', 'aim'],
    ['230', 'KIAS', 'aim'],
    ['265', 'KIAS', 'aim'],
  ]);

  const wind = byId('aviation.ifr.hold-wind-timing')['x-assumptions'];
  assert.deepEqual(wind.map(({ value, unit, source }) => [value, unit, source]), [
    ['1', 'min', 'aim'],
    ['3', '1', 'faa-ifh'],
    ['3', 'deg/s', 'faa-ifh'],
  ]);
});

test('the weather decoders expose the FAA flight-category boundaries', () => {
  const expected = [
    ['500', 'ft', 'faa-awh'],
    ['1', 'mi', 'faa-awh'],
    ['1000', 'ft', 'faa-awh'],
    ['3', 'mi', 'faa-awh'],
    ['3000', 'ft', 'faa-awh'],
    ['5', 'mi', 'faa-awh'],
  ];
  for (const id of ['aviation.weather.metar-decode', 'aviation.weather.taf-decode']) {
    const assumptions = byId(id)['x-assumptions'];
    assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), expected, id);
  }
});

test('the fuel planner exposes every regulatory reserve duration', () => {
  const assumptions = byId('aviation.loading.fuel-plan')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['30', 'min', 'cfr-14-91'],
    ['45', 'min', 'cfr-14-91'],
    ['20', 'min', 'cfr-14-91'],
    ['45', 'min', 'cfr-14-91'],
    ['30', 'min', 'cfr-14-91'],
  ]);
});

test('the Part 107 tools expose the regulatory altitude and speed limits', () => {
  const altitude = byId('drone.ops.part107-altitude')['x-assumptions'];
  assert.deepEqual(altitude.map(({ value, unit, source }) => [value, unit, source]), [
    ['400', 'ft', 'cfr-14-107'],
    ['400', 'ft', 'cfr-14-107'],
    ['400', 'ft', 'cfr-14-107'],
  ]);

  const speed = byId('drone.ops.speed-check')['x-assumptions'];
  assert.deepEqual(speed.map(({ value, unit, source }) => [value, unit, source]), [
    ['87', 'kt', 'cfr-14-107'],
  ]);
});

test('the drone energy screener exposes both regulatory comparisons', () => {
  const assumptions = byId('drone.ops.kinetic-energy')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['900', 'g', 'eu-2019-945'],
    ['80', 'J', 'eu-2019-945'],
    ['11', 'ft*lbf', 'cfr-14-107'],
  ]);
});

test('the EASA classifier exposes its complete regulatory decision table', () => {
  const assumptions = byId('drone.ops.easa-subcategory')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['25', 'kg', 'eu-2019-947'],
    ['250', 'g', 'eu-2019-945'],
    ['900', 'g', 'eu-2019-945'],
    ['80', 'J', 'eu-2019-945'],
    ['4', 'kg', 'eu-2019-945'],
    ['25', 'kg', 'eu-2019-945'],
    ['250', 'g', 'easa-open-category'],
    ['30', 'm', 'eu-2019-947'],
    ['5', 'm', 'eu-2019-947'],
    ['150', 'm', 'eu-2019-947'],
    ['2024-01-01', 'ISO date', 'eu-2019-947'],
    ['2025-12-31', 'ISO date', 'eu-2019-947'],
  ]);
});

test('both VLOS tools expose the shared EASA and LBA guidance constants', () => {
  const expected = [
    ['327', '1', 'easa-guidelines'],
    ['20', 'm', 'easa-guidelines'],
    ['490', '1', 'easa-guidelines'],
    ['30', 'm', 'easa-guidelines'],
    ['0.3', '1', 'easa-guidelines'],
    ['5', 'km', 'easa-guidelines'],
    ['5', 'km', 'easa-guidelines'],
    ['5', 'km', 'lba-flight-geography'],
  ];
  for (const id of ['drone.sensors.vlos', 'drone.ops.vlos-check']) {
    const assumptions = byId(id)['x-assumptions'];
    assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), expected, id);
  }
});

test('the wind-limit tool exposes every terrain exponent and its report height', () => {
  const assumptions = byId('drone.ops.wind-limit')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['0.09', '1', 'nrel-small-wind-site'],
    ['1/7', '1', 'nrel-wind-resource-handbook'],
    ['0.19', '1', 'nrel-small-wind-site'],
    ['0.24', '1', 'nrel-small-wind-site'],
    ['0.31', '1', 'nrel-small-wind-site'],
    ['0.43', '1', 'nrel-small-wind-site'],
    ['10', 'm', 'nrel-small-wind-site'],
  ]);
});

test('the link budget exposes its physical constant and both regulatory limits', () => {
  const assumptions = byId('drone.links.link-budget')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['299792458', 'm/s', 'nist-sp-811'],
    ['32.44778322188338', 'dB', 'itu-r-p525'],
    ['36', 'dBm EIRP', 'cfr-47-15'],
    ['20', 'dBm EIRP', 'etsi-en-300-328'],
  ]);
});

test('the ASPRS accuracy tool exposes every acceptance threshold', () => {
  const assumptions = byId('drone.photogrammetry.asprs-accuracy')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['30', 'checkpoints', 'asprs-pas'],
    ['25', '%', 'asprs-pas'],
    ['3', 'times target RMSE', 'asprs-pas'],
    ['2', '1', 'asprs-pas'],
  ]);
});

test('the GCP planner exposes its complete ASPRS table and product default', () => {
  const assumptions = byId('drone.photogrammetry.gcp-plan')['x-assumptions'];
  assert.deepEqual(assumptions.map(({ value, unit, source }) => [value, unit, source]), [
    ['30', 'checkpoints', 'asprs-pas'],
    ['500', 'km2', 'asprs-pas'],
    ['5', 'checkpoints', 'asprs-pas'],
    ['250', 'km2', 'asprs-pas'],
    ['70', 'checkpoints', 'asprs-pas'],
    ['2500', 'km2', 'asprs-pas'],
    ['2', '1', 'asprs-pas'],
    ['2', '1', 'asprs-pas'],
    ['10', 'm', 'geoprims-drone-planning-spec'],
  ]);
});

test('every tool that rests on constants declares them', () => {
  const missing = MUST_DECLARE.filter((id) => !(byId(id)?.['x-assumptions'] ?? []).length);
  assert.deepEqual(missing, []);
});

test('every declared constant is well formed and cites a known source', () => {
  const problems = metaschemaProblems(catalog, sourceIds).filter((p) => p.includes('assumption'));
  assert.deepEqual(problems, []);
  let declared = 0;
  for (const t of catalog.tools) declared += (t['x-assumptions'] ?? []).length;
  assert.ok(declared >= 20, `only ${declared} constants declared`);
});

test('the gate bites', () => {
  const t = { id: 'fixture.assume', examples: [], inputs: { properties: {} }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, 'x-assumptions': [{ name: 'R', value: 287, unit: 'J/(kg K)', source: 'nope' }] }] }, sourceIds), [
    'fixture.assume: assumption R cites unknown source nope',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, 'x-assumptions': [{ name: 'R', unit: '1', source: 'icao-7488' }] }] }, sourceIds), [
    'fixture.assume: assumption R has no value',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, 'x-assumptions': [{ name: 'R', value: 1, unit: '1', source: 'icao-7488', note: 'x' }] }] }, sourceIds), [
    'fixture.assume: assumption R has no field note',
  ]);
});

test('the page and the agent see the same constants', () => {
  const html = readFileSync(join(root, 'apps/web/dist/aviation/altimetry/density-altitude/index.html'), 'utf8');
  for (const a of byId('aviation.altimetry.density-altitude')['x-assumptions']) {
    assert.ok(html.includes(a.name), `${a.name} is not on the page`);
    assert.ok(html.includes(a.value), `${a.value} is not on the page`);
  }
  assert.match(readFileSync(join(root, 'mcp/meta.mjs'), 'utf8'), /assumptions: m\['x-assumptions'\]/);
});
