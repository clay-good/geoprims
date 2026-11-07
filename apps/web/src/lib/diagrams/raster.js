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

/** The grid the reader entered, as a frame and a scale to draw inside. */
function gridFrame(args) {
  const rows = Array.isArray(args.elevations) ? args.elevations : [];
  const h = rows.length;
  const w = h ? String(rows[0]?.row ?? '').split(/[,\s]+/).filter(Boolean).length : 0;
  const cell = measure(args.cell_size, LENGTH, 'm');
  if (h < 2 || w < 2 || !Number.isFinite(cell) || cell <= 0) return null;
  const side = Math.min(250 / w, 160 / h);
  const [x0, y0] = [(320 - side * w) / 2, 46];
  return {
    h, w, cell, side, x0, y0,
    X: (east) => x0 + (east / cell) * side,
    Y: (south) => y0 + (south / cell) * side,
    cx: (column) => x0 + (column - 0.5) * side,
    cy: (row) => y0 + (row - 0.5) * side,
    box: `<rect class="dg-grid" fill="none" x="${x0.toFixed(1)}" y="${y0.toFixed(1)}" width="${(side * w).toFixed(1)}" height="${(side * h).toFixed(1)}"/>`,
  };
}

/**
 * The stream network over the reader's own grid: every link drawn through the
 * cells it runs, thicker as its Strahler order rises, so the main stem reads
 * at a glance and the headwaters read as headwaters.
 */
function streamNetwork(args, result) {
  const links = result.result?.links ?? [];
  const verts = result.result?.vertices ?? [];
  const g = gridFrame(args);
  if (!g || !links.length || !verts.length) return null;
  const order = new Map(links.map((l) => [l.link, l.order]));
  const byLink = new Map();
  for (const v of verts) {
    const pt = [g.X(v.east?.value ?? v.east), g.Y(v.south?.value ?? v.south)];
    if (!Number.isFinite(pt[0]) || !Number.isFinite(pt[1])) return null;
    if (!byLink.has(v.link)) byLink.set(v.link, []);
    byLink.get(v.link).push(pt);
  }
  const top = Math.max(...links.map((l) => l.order));
  const body = [
    g.box,
    ...[...byLink].map(([id, pts]) => {
      const ord = order.get(id) ?? 1;
      const width = (1.2 + 1.3 * (ord - 1)).toFixed(1);
      const d = pts.map((p, i) => `${i ? 'L' : 'M'}${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join('');
      return `<path class="dg-accent" fill="none" stroke-width="${width}" d="${d}"/>`;
    }),
    // The outlet of the highest-order link: where all of it leaves.
    ...(() => {
      const main = links.filter((l) => l.order === top).sort((a, b) => (b.outlet_area?.value ?? 0) - (a.outlet_area?.value ?? 0))[0];
      const pts = main ? byLink.get(main.link) : null;
      return pts ? [dot(...pts[pts.length - 1], 'dg-dot-now')] : [];
    })(),
    text('dg-label', 160, 30, `${links.length} links, highest order ${top}`, 'middle'),
    text('dg-muted-text', 12, 214, `${disp(result, 'channel_cells')} channel cells · ${disp(result, 'network_length')}`),
    text('dg-muted-text', 12, 228, 'Thicker is higher order'),
  ].join('');
  const title = `Stream network over the ${g.h} by ${g.w} grid: ${links.length} links, the highest Strahler order ${top}, ${disp(result, 'network_length')} of channel in all at a threshold of ${disp(result, 'threshold_used')} cells. Thicker lines are higher order; the dot is the network's outlet. To scale.`;
  return { markup: svg(body, title), desc: title };
}

/**
 * The basin above a pour point: the cells that drain to it shaded over the
 * reader's grid, with the longest flow path drawn down it and the outlet
 * marked, so "what drains here" is a picture rather than a number.
 */
function basinMap(args, result) {
  const g = gridFrame(args);
  const basin = result.result?.basin ?? [];
  const path = result.result?.path ?? [];
  if (!g || basin.length !== g.h || !path.length) return null;
  const cells = basin.map((row) => String(row.row ?? '').split(/[,\s]+/).filter(Boolean));
  if (cells.some((row) => row.length !== g.w)) return null;
  const squares = [];
  for (let r = 0; r < g.h; r++) {
    for (let c = 0; c < g.w; c++) {
      if (cells[r][c] !== '1') continue;
      squares.push(
        `<rect class="dg-wash" x="${(g.x0 + c * g.side).toFixed(1)}" y="${(g.y0 + r * g.side).toFixed(1)}" width="${g.side.toFixed(1)}" height="${g.side.toFixed(1)}"/>`,
      );
    }
  }
  const pts = path.map((s) => [g.cx(s.column), g.cy(s.row)]);
  const d = pts.map((p, i) => `${i ? 'L' : 'M'}${p[0].toFixed(1)} ${p[1].toFixed(1)}`).join('');
  const outlet = [g.cx(val(result, 'outlet_column')), g.cy(val(result, 'outlet_row'))];
  const body = [
    ...squares,
    g.box,
    `<path class="dg-accent" fill="none" stroke-width="1.8" d="${d}"/>`,
    dot(...pts[0]),
    dot(...outlet, 'dg-dot-now'),
    text('dg-label', 160, 30, `${disp(result, 'basin_cells')} cells drain to the dot`, 'middle'),
    text('dg-muted-text', 12, 214, `Basin ${disp(result, 'basin_area')} · relief ${disp(result, 'relief')}`),
    text('dg-muted-text', 12, 228, `Longest path ${disp(result, 'longest_path')}`),
  ].join('');
  const title = `The basin above row ${val(result, 'outlet_row')}, column ${val(result, 'outlet_column')} of the ${g.h} by ${g.w} grid: ${disp(result, 'basin_cells')} shaded cells, ${disp(result, 'basin_area')} in all, with the longest flow path drawn from its top down to the outlet, ${disp(result, 'longest_path')} long. To scale.`;
  return { markup: svg(body, title), desc: title };
}

export const DIAGRAMS = {
  'raster.terrain.line-of-sight': terrainSightLine,
  'raster.terrain.fresnel-terrain': terrainSightLine,
  'raster.hydrology.streams': streamNetwork,
  'raster.hydrology.watershed': basinMap,
};
