// Vector diagrams (web/map-canvas "Canvas modes": vector diagram): unit-free
// engineering drawings for tools whose answer is a set of vectors. Each is
// SVG markup with CSS classes for color, so every display mode applies and
// nothing needs inline styles. Values come from the core's result wherever it
// has them; inputs are read only for what the result does not echo.


import { draw, measure, exaggeration, station } from './diagrams/kit.js';
import { DIAGRAMS as AVIATION } from './diagrams/aviation.js';
import { DIAGRAMS as DRONE } from './diagrams/drone.js';
import { DIAGRAMS as GEODESY } from './diagrams/geodesy.js';
import { DIAGRAMS as NAVIGATION } from './diagrams/navigation.js';
import { DIAGRAMS as RASTER } from './diagrams/raster.js';
import { DIAGRAMS as SURVEY } from './diagrams/survey.js';
import { DIAGRAMS as TIME } from './diagrams/time.js';

export { measure, exaggeration, station };

const PARTS = { aviation: AVIATION, drone: DRONE, geodesy: GEODESY, navigation: NAVIGATION, raster: RASTER, survey: SURVEY, time: TIME };
/** Every tool with a diagram, in the order they were added. */
const DIAGRAMS = Object.fromEntries(
  [
    'geodesy.frame.to-local',
    'survey.gnss.dop',
    'aviation.wind.heading-groundspeed',
    'aviation.wind.runway-components',
    'navigation.route.cpa',
    'navigation.route.fly-by',
    'aviation.performance.top-of-descent',
    'aviation.performance.vdp',
    'survey.curves.vertical-curve',
    'survey.land.deed-plot',
    'aviation.airspeed.cas-to-tas',
    'aviation.airspeed.tas-to-cas',
    'aviation.atmosphere.isa',
    'aviation.loading.weight-balance',
    'aviation.altimetry.true-altitude',
    'navigation.vector.operations',
    'survey.earthwork.profile-grades',
    'survey.curves.spiral',
    'raster.terrain.line-of-sight',
    'survey.earthwork.borrow-pit',
    'geodesy.height.convert',
    'aviation.ifr.hold-entry',
    'time.sun.mapping-window',
    'survey.curves.circular-curve',
    'aviation.performance.climb-gradient',
    'aviation.performance.climb-plan',
    'aviation.performance.etp-pnr',
    'navigation.los.horizon',
    'navigation.los.visibility',
    'navigation.los.fresnel',
    'survey.earthwork.average-end-area',
    'survey.earthwork.prismoidal',
    'drone.photogrammetry.gsd',
    'drone.photogrammetry.altitude-for-gsd',
    'drone.photogrammetry.trigger',
    'drone.photogrammetry.oblique-gsd',
    'drone.ops.part107-altitude',
    'drone.power.rth-budget',
    'aviation.wind.find-wind',
    'aviation.wind.course-from-heading',
    'aviation.wind.tas-from-groundspeed',
    'aviation.ifr.hold-wind-timing',
    'geodesy.magnetic.true-to-magnetic',
    'geodesy.datum.nad83',
    'geodesy.parse.bearing-difference',
    'navigation.los.dip',
    'survey.cogo.forward',
    'survey.cogo.inverse',
    'survey.cogo.area-by-coordinates',
    'survey.cogo.traverse-closure',
    'drone.ops.wind-limit',
    'drone.ops.speed-check',
    'drone.power.battery-energy',
    'aviation.airspeed.ias-to-tas',
    'aviation.airspeed.ias-to-mach',
    'aviation.airspeed.ias-to-eas',
    'aviation.airspeed.eas-to-tas',
    'aviation.airspeed.eas-to-mach',
    'aviation.airspeed.mach-to-eas',
    'aviation.airspeed.tas-to-eas',
  ].map((id) => [id, PARTS[id.split('.')[0]][id]]),
);

/**
 * The diagram for a tool's result, or null: { markup, desc }.
 * `at` names this drawing, so a second copy of the same diagram on one page
 * carries its own marker ids.
 */
export function diagram(id, args, result, at = '', view = {}) {
  return draw(DIAGRAMS[id], args, result, at, view);
}


export const DIAGRAM_TOOLS = Object.keys(DIAGRAMS);
