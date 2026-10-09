// The raster tools' diagrams. See diagrams.js for what a diagram is.

import { LENGTH, disp, dot, line, measure, svg, text, unitOf, val } from './kit.js';

/**
 * Terrain line of sight: the ground profile, both masts, and the sight line
 * between their tops at the heights the core worked out (in elevation, so the
 * Earth's curve bends it), with the floor of 60% of the Fresnel zone when a
 * frequency was given and the point that comes closest called out in words.
 */
function terrainSightLine(args, result) {
  const r = result.result;
  const rows = r.profile ?? [];
  const eu = unitOf(result, 'clearance');
  const du = unitOf(result, 'clearance_at');
  const ground = (Array.isArray(args.points) ? args.points : []).map((p) => [measure(p.distance, LENGTH, du), measure(p.elevation, LENGTH, eu)]);
  const [ho, ht] = [measure(args.observer_height, LENGTH, eu), measure(args.target_height, LENGTH, eu)];
  if (ground.length !== rows.length + 2 || ground.flat().some((v) => !Number.isFinite(v)) || !Number.isFinite(ho) || !Number.isFinite(ht) || !LENGTH[eu] || !LENGTH[du]) return null;
  const s0 = ground[0][0];
  const n = ground.length - 1;
  const sight = [[0, ground[0][1] + ho], ...rows.map((w, i) => [ground[i + 1][0] - s0, val({ result: w }, 'sight_line') * LENGTH[eu]]), [ground[n][0] - s0, ground[n][1] + ht]];
  const floor = rows.map((w, i) => [sight[i + 1][0], sight[i + 1][1] - 0.6 * (val({ result: w }, 'fresnel_radius') ?? NaN) * LENGTH[eu]]);
  const fresnel = floor.every((p) => Number.isFinite(p[1])) && rows.length > 0 && rows.every((w) => w.fresnel_radius);
  const flat = ground.map(([d, e]) => [d - s0, e]);
  const all = [...flat, ...sight, ...(fresnel ? floor : [])];
  const span = (i, lo, hi) => {
    const [a, b] = [Math.min(...all.map((p) => p[i])), Math.max(...all.map((p) => p[i]))];
    const pad = (b - a) * 0.06 || 1;
    return { at: (v) => lo + ((v - a + pad) / (b - a + 2 * pad)) * (hi - lo), per: (hi - lo) / (b - a + 2 * pad) };
  };
  const [sx, sy] = [span(0, 30, 300), span(1, 190, 50)];
  const P = ([x, y]) => [sx.at(x), sy.at(y)];
  const k = Math.abs(sy.per / sx.per);
  const chain = (pts, cls) => pts.slice(1).map((p, i) => line(cls, P(pts[i]), P(p))).join('');
  const visible = r.visible === 'yes';
  const at = rows.findIndex((w) => w.distance?.value === r.clearance_at?.value);
  const worst = at >= 0 ? P(flat[at + 1]) : null;
  const body = [
    chain(flat, 'dg-muted'),
    line('dg-muted', P(flat[0]), P(sight[0])), line('dg-muted', P(flat[n]), P(sight[n])),
    fresnel ? chain([sight[0], ...floor, sight[n]], 'dg-grid dg-dash') : '',
    chain(sight, visible ? 'dg-accent' : 'dg-accent dg-dash'),
    dot(...P(sight[0])), dot(...P(sight[n])),
    worst ? dot(...worst, 'dg-dot-now') + text('dg-label', Math.min(Math.max(worst[0], 90), 230), 206, visible ? `Closest: ${disp(result, 'clearance')} clear` : `Blocks the view by ${disp(result, 'obstruction_height')}`, 'middle') : '',
    text('dg-muted-text', P(sight[0])[0], P(sight[0])[1] - 8, 'Observer'),
    text('dg-muted-text', P(sight[n])[0], P(sight[n])[1] - 8, 'Target', 'end'),
    text('dg-muted-text', 30, 222, `Distance → · heights ×${k.toFixed(0)}`),
    fresnel ? text('dg-muted-text', 30, 236, 'Dashed: 60% of the Fresnel zone') : '',
    text('dg-label', 160, 26, visible ? (fresnel && r.fresnel_clear !== 'yes' ? 'In sight, but the Fresnel zone is not clear' : 'In sight') : 'Not in sight', 'middle'),
  ].join('');
  const title = `Ground profile of ${ground.length} points: ${visible ? `the target is in sight, with ${disp(result, 'clearance')} at the closest point` : `the ground at ${disp(result, 'clearance_at')} blocks the view by ${disp(result, 'obstruction_height')}`}` +
    `${fresnel ? `; 60% of the Fresnel zone is ${r.fresnel_clear === 'yes' ? 'clear' : 'not clear'}` : ''}. Heights exaggerated ${k.toFixed(0)} times.`;
  return { markup: svg(body, title), desc: title };
}

export const DIAGRAMS = {
  'raster.terrain.line-of-sight': terrainSightLine,
};
