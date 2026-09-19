// Workflow prompts (add-local-mcp-server, "Resources and prompts"). Each
// prompt turns its arguments into one geoprims_pipeline call and tells the
// agent what to check and relay. Prompt arguments arrive as strings.

const arg = (name, description, required = true) => ({ name, description, required });

export const PROMPTS = [
  {
    name: 'preflight-performance',
    title: 'Preflight performance check',
    description: 'Pressure altitude, density altitude, and runway wind components for a departure, with the crosswind checked against a limit.',
    arguments: [
      arg('elevation', 'Field elevation, like "5000 ft"'),
      arg('altimeter', 'Altimeter setting, like "29.80 inHg"'),
      arg('temperature', 'Outside air temperature, like "30 degC"'),
      arg('runway', 'Runway number, like "27"'),
      arg('wind_direction', 'Wind direction in degrees true or magnetic to match the runway, like "300 deg"'),
      arg('wind_speed', 'Wind speed, like "15 kt"'),
      arg('max_crosswind', 'Demonstrated or personal crosswind limit, like "15 kt"', false),
    ],
    steps: (a) => [
      { id: 'aviation.altimetry.pressure-altitude', args: { elevation: a.elevation, altimeter: a.altimeter } },
      { id: 'aviation.altimetry.density-altitude', args: { elevation: a.elevation, altimeter: a.altimeter, temperature: a.temperature } },
      {
        id: 'aviation.wind.runway-components',
        args: { runway: a.runway, wind_direction: a.wind_direction, wind_speed: a.wind_speed, ...(a.max_crosswind ? { max_crosswind: a.max_crosswind } : {}) },
      },
    ],
    guide:
      'Report pressure and density altitude and how far density altitude is above the field. Report headwind or tailwind and crosswind with its side, and say plainly if the crosswind exceeds the limit. Use the aircraft flight manual for takeoff distance; these numbers are inputs to it, not a replacement.',
  },
  {
    name: 'photogrammetry-mission',
    title: 'Photogrammetry mission plan',
    description: 'Flight height for a target ground sample distance, then trigger interval and line spacing at the chosen overlaps, and the longest exposure that keeps motion blur under half a pixel.',
    arguments: [
      arg('target_gsd', 'Ground sample distance wanted, like "2 cm"'),
      arg('sensor_width', 'Sensor width, like "13.2 mm"'),
      arg('sensor_height', 'Sensor height, like "8.8 mm"'),
      arg('focal_length', 'Lens focal length, like "8.8 mm"'),
      arg('image_width', 'Image width in pixels, like "5472"'),
      arg('image_height', 'Image height in pixels, like "3648"'),
      arg('groundspeed', 'Flight speed, like "10 m/s"'),
      arg('front_overlap', 'Front overlap in percent, like "75"'),
      arg('side_overlap', 'Side overlap in percent, like "65"'),
    ],
    steps: (a) => {
      const camera = { sensor_width: a.sensor_width, sensor_height: a.sensor_height, focal_length: a.focal_length, image_width: Number(a.image_width) };
      return [
        { id: 'drone.photogrammetry.altitude-for-gsd', args: { ...camera, image_height: Number(a.image_height), target_gsd: a.target_gsd } },
        {
          id: 'drone.photogrammetry.trigger',
          args: { ...camera, groundspeed: a.groundspeed, front_overlap: Number(a.front_overlap), side_overlap: Number(a.side_overlap) },
          bind: { '/height': '0:/result/height' },
        },
        { id: 'drone.photogrammetry.motion-blur', args: { groundspeed: a.groundspeed, gsd: a.target_gsd, exposure: '0.001 s' } },
      ];
    },
    guide:
      'Report the flight height above ground, the trigger interval and distance, and the line spacing. Report the longest exposure for half-pixel blur and compare it with the camera settings. Check the height against the operating limit where the mission is flown (400 ft above ground under Part 107).',
  },
  {
    name: 'traverse-closure',
    title: 'Traverse closure check',
    description: 'Closes a traverse from its courses and reports misclosure, its bearing, and precision ratio.',
    arguments: [arg('courses', 'The courses as JSON, like [{"direction":"N 0 E","distance":300},...], directions as bearings or azimuths')],
    steps: (a) => [{ id: 'survey.cogo.traverse-closure', args: { courses: JSON.parse(a.courses) } }],
    guide:
      'Report the linear misclosure with its bearing and the precision ratio, and compare the ratio with the standard that applies to the survey (for example 1:10,000 for many boundary surveys). If it fails, suggest checking the course with the largest angular or distance doubt first.',
  },
  {
    name: 'coordinate-conversion-audit',
    title: 'Coordinate conversion audit',
    description: 'Parses a coordinate in any common notation, converts it to UTM, and converts back to show the round-trip error.',
    arguments: [arg('coordinate', 'A coordinate as written, like 40°26\'46"N 79°58\'56"W or 40.4461, -79.9822')],
    steps: (a) => [
      { id: 'geodesy.parse.coordinates', args: { text: a.coordinate } },
      { id: 'geodesy.utm.forward', args: {}, bind: { '/lat': '0:/result/lat', '/lon': '0:/result/lon' } },
      {
        id: 'geodesy.utm.inverse',
        args: {},
        bind: { '/zone': '1:/result/zone', '/hemisphere': '1:/result/hemisphere', '/easting': '1:/result/easting', '/northing': '1:/result/northing' },
      },
    ],
    guide:
      'Report how the input was read (notation and decimal degrees), the UTM zone, easting, and northing, and the difference between the parsed and round-tripped latitude and longitude. Name the datum the user assumed; geoprims does not change datums in this chain.',
  },
  {
    name: 'h3-resolution-choice',
    title: 'H3 resolution choice',
    description: 'Picks the H3 resolution whose average cell area is closest to a target, then shows the cell at a location and its real area there.',
    arguments: [
      arg('target_area', 'Cell area wanted, like "1 km2"'),
      arg('lat', 'Latitude of a representative point, like "40.4461"'),
      arg('lon', 'Longitude of a representative point, like "-79.9822"'),
    ],
    steps: (a) => [
      { id: 'indexing.h3.resolution-chooser', args: { target_area: a.target_area } },
      { id: 'indexing.h3.lat-lng-to-cell', args: { lat: Number(a.lat), lon: Number(a.lon) }, bind: { '/resolution': '0:/result/resolution' } },
      { id: 'indexing.h3.cell-info', args: {}, bind: { '/cell': '1:/result/cell' } },
    ],
    guide:
      'Report the chosen resolution with its average area and the next coarser one, then the cell id at the point and its actual area there, which differs from the average by latitude. Note that cell areas vary about twofold across the globe at any resolution.',
  },
];

/** The listing for prompts/list: name, title, description, and arguments. */
export const promptList = () => PROMPTS.map(({ name, title, description, arguments: args }) => ({ name, title, description, arguments: args }));

/** prompts/get: the pipeline to run and what to relay, or an error message for bad arguments. */
export function getPrompt(name, args = {}) {
  const p = PROMPTS.find((x) => x.name === name);
  if (!p) return { error: `Unknown prompt ${name}. Prompts: ${PROMPTS.map((x) => x.name).join(', ')}` };
  const missing = p.arguments.filter((a) => a.required && (args[a.name] ?? '') === '').map((a) => a.name);
  if (missing.length) return { error: `Prompt ${name} needs ${missing.join(', ')}.` };
  let steps;
  try {
    steps = p.steps(args);
  } catch (e) {
    return { error: `Prompt ${name}: ${e.message}` };
  }
  const text = [
    `${p.description}`,
    '',
    'Run geoprims_pipeline with these arguments:',
    '```json',
    JSON.stringify({ steps }, null, 2),
    '```',
    '',
    p.guide,
    'Relay every warning in the results and the meta notice. If a step fails, report its error and hint rather than guessing a value.',
  ].join('\n');
  return { description: p.description, messages: [{ role: 'user', content: { type: 'text', text } }], steps };
}
