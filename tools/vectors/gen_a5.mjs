// Generates A5 0.10.1 differential fixtures. Install the pinned reference in
// a temporary directory, then pass its built module and the output path:
//   npm install --prefix /tmp/a5-ref a5-js@0.10.1
//   node tools/vectors/gen_a5.mjs /tmp/a5-ref/node_modules/a5-js/dist/a5.js core/crates/gp-indexing/tests/data/a5_parity.json
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const [modulePath, outputPath] = process.argv.slice(2);
if (!modulePath || !outputPath) throw new Error('usage: gen_a5.mjs A5_MODULE OUTPUT_JSON');
const a5 = await import(pathToFileURL(modulePath));

let state = 0x5a17c0de;
const random = () => ((state = (1664525 * state + 1013904223) >>> 0) / 2 ** 32);
const fixed = [
  [-0.1276, 51.5074, 10],
  [-79.982222, 40.446111, 0],
  [179.999, 0.1, 1],
  [-179.999, -0.1, 2],
  [0, 89.999, 5],
  [120, -89.999, 12],
  [151.2093, -33.8688, 20],
  [-74.006, 40.7128, 29],
  [2.3522, 48.8566, 30],
];
const cases = [...fixed];
for (let i = 0; i < 111; i++) cases.push([-180 + 360 * random(), -89.9 + 179.8 * random(), i % 31]);

const rows = cases.map(([lon, lat, requestedResolution]) => {
  const cell = a5.lonLatToCell([lon, lat], requestedResolution);
  const resolution = a5.getResolution(cell);
  return {
    lon,
    lat,
    requestedResolution,
    cell: a5.u64ToHex(cell).padStart(16, '0'),
    resolution,
    center: a5.cellToLonLat(cell),
    boundary: a5.cellToBoundary(cell),
    area: a5.cellArea(resolution),
    parent: a5.u64ToHex(a5.cellToParent(cell)).padStart(16, '0'),
    children: resolution < 30 ? [...a5.cellToChildren(cell)].map((child) => a5.u64ToHex(child).padStart(16, '0')) : [],
  };
});

writeFileSync(outputPath, `${JSON.stringify({ reference: 'a5-js 0.10.1', rows }, null, 2)}\n`);
