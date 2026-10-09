// The survey tools' diagrams. See diagrams.js for what a diagram is.

import { LENGTH, R, arrow, deg, disp, dot, exaggeration, f1, fit, line, measure, path, plane, station, svg, text, typed, unitOf, val, vec } from './kit.js';

const AREA = { m2: 1, ft2: 0.09290304, yd2: 0.83612736, ftus2: 0.09290341161 };
/**
 * GNSS planning: the sky at the start (zenith in the middle, the horizon the
 * rim) with each satellite as the core placed it, filled when it is used and
 * hollow when the mask or the skyline leaves it out, beside PDOP through the
 * window on the same time axis the table lists.
 */
function gnssSky(args, result) {
  const r = result.result;
  const sats = r.satellites ?? [];
  const steps = r.timeline ?? [];
  if (!steps.length) return null;
  const [cx, cy, R0] = [92, 112, 72];
  const mask = deg(args.mask) ?? 10;
  const ring = (e) => (R0 * (90 - e)) / 90;
  const sky = (az, el) => { const [dx, dy] = vec(az, ring(Math.max(el, 0))); return [cx + dx, cy + dy]; };
  const parts = [
    `<circle class="dg-muted" cx="${cx}" cy="${cy}" r="${R0}" fill="none"/>`,
    ...[30, 60].map((e) => `<circle class="dg-grid" cx="${cx}" cy="${cy}" r="${f1(ring(e))}" fill="none"/>`),
    `<circle class="dg-grid dg-dash" cx="${cx}" cy="${cy}" r="${f1(ring(mask))}" fill="none"/>`,
    ...[['N', 0], ['E', 90], ['S', 180], ['W', 270]].map(([t, b]) => { const [tx, ty] = vec(b, R0 + 10); return text('dg-muted-text', cx + tx, cy + ty + 4, t, 'middle'); }),
  ];
  const skyline = (Array.isArray(args.horizon) ? args.horizon : []).map((h) => [deg(h.azimuth), deg(h.elevation)]).filter((p) => p.every(Number.isFinite)).sort((a, b) => a[0] - b[0]);
  if (skyline.length > 1) {
    const pts = [];
    for (let az = 0; az <= 360; az += 5) {
      const k = skyline.findIndex((p) => p[0] > (az % 360));
      const [a, b] = k <= 0 ? [skyline.at(-1), skyline[0]] : [skyline[k - 1], skyline[k]];
      const span = ((b[0] - a[0]) % 360 + 360) % 360 || 360;
      const x = ((((az % 360) - a[0]) % 360) + 360) % 360 / span;
      pts.push(sky(az, a[1] + (b[1] - a[1]) * x));
    }
    parts.push(`<path class="dg-muted" fill="none" d="M${pts.map((p) => `${f1(p[0])} ${f1(p[1])}`).join('L')}Z"/>`);
  }
  for (const s of sats) {
    const [x, y] = sky(val({ result: s }, 'azimuth'), val({ result: s }, 'elevation'));
    const used = s.used === 'yes';
    parts.push(`<circle class="${used ? 'dg-dot' : 'dg-hollow'}" cx="${f1(x)}" cy="${f1(y)}" r="${used ? 4.5 : 3.5}"/>`);
    parts.push(text('dg-muted-text', x + 6, y - 5, String(s.prn)));
  }
  // PDOP through the window, 0 at the bottom, at least 6 at the top.
  const [x0, x1, y0, y1] = [196, 306, 186, 52];
  const pd = steps.map((t) => (typeof t.pdop === 'number' ? t.pdop : null));
  const top = Math.max(6, ...pd.filter((p) => p !== null).map((p) => Math.ceil(p)));
  const X = (i) => (steps.length === 1 ? (x0 + x1) / 2 : x0 + ((x1 - x0) * i) / (steps.length - 1));
  const Y = (p) => y0 - ((y0 - y1) * Math.min(p, top)) / top;
  parts.push(line('dg-grid', [x0, y0], [x1, y0]), line('dg-grid', [x0, y0], [x0, y1]));
  parts.push(text('dg-muted-text', x0 - 3, y1 + 4, String(top), 'end'), text('dg-muted-text', x0 - 3, y0 + 4, '0', 'end'));
  if (top >= 6) parts.push(line('dg-grid dg-dash', [x0, Y(6)], [x1, Y(6)]), text('dg-muted-text', x1, Y(6) + 11, 'GPS limit 6', 'end'));
  for (let i = 1; i < pd.length; i++) if (pd[i - 1] !== null && pd[i] !== null) parts.push(line('dg-accent', [X(i - 1), Y(pd[i - 1])], [X(i), Y(pd[i])]));
  pd.forEach((p, i) => { if (p !== null) parts.push(dot(X(i), Y(p))); });
  parts.push(text('dg-muted-text', x0, y0 + 14, steps[0].time.slice(11, 16)), text('dg-muted-text', x1, y0 + 14, steps.at(-1).time.slice(11, 16), 'end'));
  parts.push(text('dg-label', (x0 + x1) / 2, 36, 'PDOP, UTC', 'middle'));
  const used = sats.filter((s) => s.used === 'yes').length;
  parts.push(text('dg-label', 160, 226, `${used} of ${sats.length} in the sky used · hollow: masked`, 'middle'));
  const title = `Sky plot at the start: ${used} satellites used of ${sats.length} above the horizon, with a ${mask}° mask${skyline.length > 1 ? ' and the skyline drawn' : ''}. ` +
    `PDOP through the window${r.worst_pdop ? `, worst ${disp(result, 'worst_pdop')} at ${r.worst_at.slice(11, 16)} UTC` : ''}.`;
  return { markup: svg(parts.join(''), title), desc: title };
}

/** Vertical curve: the two grade tangents meeting at the PVI, and the parabola between PVC and PVT. */
function verticalCurve(args, result) {
  const [sc, st] = [station(result.result.pvc_station), station(result.result.pvt_station)];
  const [yc, yt] = [val(result, 'pvc_elevation'), val(result, 'pvt_elevation')];
  const [g1, g2] = [Number(args.g1), Number(args.g2)];
  if (![sc, st, yc, yt, g1, g2].every(Number.isFinite) || st <= sc) return null;
  const L = st - sc;
  const y = (x) => yc + (g1 / 100) * x + ((g2 - g1) / (200 * L)) * x * x;
  const pvi = [sc + L / 2, yc + (g1 / 100) * (L / 2)];
  const curve = Array.from({ length: 41 }, (_, i) => [sc + (L * i) / 40, y((L * i) / 40)]);
  // Grades are a few percent: scale height and length apart, and say so.
  const pts = [...curve, pvi];
  const [xmin, xmax] = [Math.min(...pts.map((p) => p[0])), Math.max(...pts.map((p) => p[0]))];
  const [ymin, ymax] = [Math.min(...pts.map((p) => p[1])), Math.max(...pts.map((p) => p[1]))];
  const S = ([x, yy]) => [40 + ((x - xmin) / (xmax - xmin)) * 240, 170 - ((yy - ymin) / Math.max(ymax - ymin, 1e-9)) * 110];
  const path = curve.map((p, i) => `${i ? 'L' : 'M'}${S(p).map(f1).join(' ')}`).join('');
  const [c, t, v] = [S([sc, yc]), S([st, yt]), S(pvi)];
  const turn = result.result.turning_station ? S([station(result.result.turning_station), val(result, 'turning_elevation')]) : null;
  const body = [
    `<line class="dg-muted dg-dash" x1="${f1(c[0])}" y1="${f1(c[1])}" x2="${f1(v[0])}" y2="${f1(v[1])}"/>`,
    `<line class="dg-muted dg-dash" x1="${f1(v[0])}" y1="${f1(v[1])}" x2="${f1(t[0])}" y2="${f1(t[1])}"/>`,
    `<path class="dg-casing" fill="none" d="${path}"/><path class="dg-accent" d="${path}"/>`,
    dot(...c), dot(...t), dot(v[0], v[1], 'dg-dot-now'),
    text('dg-label', c[0], c[1] + 20, `PVC ${result.result.pvc_station}`, 'middle'),
    text('dg-label', t[0], t[1] + 20, `PVT ${result.result.pvt_station}`, 'middle'),
    text('dg-muted-text', v[0] + 8, v[1] + (result.result.curve_type === 'high' ? -8 : 16), 'PVI'),
    turn ? `${dot(turn[0], turn[1], 'dg-dot-now')}${text('dg-muted-text', turn[0], result.result.curve_type === 'high' ? 36 : 206, `${result.result.curve_type === 'high' ? 'High' : 'Low'} point ${result.result.turning_station}`, 'middle')}${line('dg-grid', [turn[0], result.result.curve_type === 'high' ? 42 : 196], turn)}` : '',
    text('dg-muted-text', 160, 232, `${g1 > 0 ? '+' : ''}${g1}% to ${g2 > 0 ? '+' : ''}${g2}% · K ${result.result.k}`, 'middle'),
  ].join('');
  const title = `Vertical curve from PVC ${result.result.pvc_station} to PVT ${result.result.pvt_station}, grades ${g1}% to ${g2}%${turn ? `, ${result.result.curve_type} point at ${result.result.turning_station}` : ''}. Vertical scale exaggerated.`;
  return { markup: svg(body, title), desc: title };
}

/** Traverse sketch: the courses as plotted, numbered, with the gap that does not close. */
function traverseSketch(args, result) {
  const pts = (result.result.points ?? []).map((p) => [val({ result: p }, 'easting'), val({ result: p }, 'northing')]);
  if (pts.length < 3 || !pts.flat().every(Number.isFinite)) return null;
  const [a, z] = [pts[0], pts[pts.length - 1]];
  // The misclosure runs from the point of beginning to where the last call
  // ends. It is usually far smaller than a pixel, so it is drawn larger by a
  // stated round factor (cogo-and-traverse "Misclosure exaggeration").
  const [dx, dy] = [z[0] - a[0], z[1] - a[1]];
  const miss = Math.hypot(dx, dy) > 1e-12 * (val(result, 'total_length') || 1);
  const box = [220, 140];
  const { factor, end: far, S } = miss ? misclosureFrame(pts, a, [dx, dy], box) : { factor: 1, S: fit(pts, ...box) };
  const xy = pts.map(S);
  const path = xy.map((p, i) => `${i ? 'L' : 'M'}${p.map(f1).join(' ')}`).join('');
  const o = S(a);
  const end = miss ? S(far) : o;
  const body = [
    `<path class="dg-casing" fill="none" d="${path}"/><path class="dg-accent" d="${path}"/>`,
    ...xy.slice(0, -1).map((p, i) => `${dot(p[0], p[1], i === 0 ? 'dg-dot-now' : 'dg-dot')}${text('dg-muted-text', p[0] + 7, p[1] - 7, String(i + 1))}`),
    miss ? arrow(o[0], o[1], end[0], end[1], 'dg-muted', factor > 1 ? `Misclosure ×${group(factor)}` : 'Misclosure', 0.9, dx >= 0 ? 1 : -1) : '',
    text('dg-label', 160, 226, `Misclosure ${disp(result, 'misclosure')} · ${result.result.precision ?? ''}`, 'middle'),
    text('dg-muted-text', 296, 30, 'N ↑', 'end'),
  ].join('');
  const scale = miss ? (factor > 1 ? `, drawn ${group(factor)} times its size from the point of beginning` : ', drawn from the point of beginning') : '';
  const title = `Traverse sketch of ${pts.length - 1} courses; it misses closing by ${disp(result, 'misclosure')} (${result.result.precision ?? ''})${scale}.`;
  return { markup: svg(body, title), desc: title };
}

/** Earthwork: two end sections a length apart, sized by their areas, and the volume between. */
function endAreas(args, result) {
  const [a1, a2] = [measure(args.area1, AREA, 'ft2'), measure(args.area2, AREA, 'ft2')];
  if (!Number.isFinite(a1) || !Number.isFinite(a2) || a1 <= 0 || a2 <= 0) return null;
  const k = 70 / Math.sqrt(Math.max(a1, a2));
  const [s1, s2] = [Math.sqrt(a1) * k, Math.sqrt(a2) * k];
  // Trapezoid sections, like a road cut: the top wider than the base.
  const section = (x, y, w) => [[x - w * 0.35, y], [x + w * 0.35, y], [x + w * 0.6, y - w * 0.7], [x - w * 0.6, y - w * 0.7]];
  const p1 = section(90, 170, s1);
  const p2 = section(230, 130, s2);
  const poly = (p, cls) => `<polygon class="${cls}" points="${p.map((q) => q.map(f1).join(',')).join(' ')}"/>`;
  const body = [
    ...p1.map((q, i) => line('dg-grid', q, p2[i])),
    poly(p1, 'dg-muted'), poly(p2, 'dg-accent'),
    text('dg-muted-text', 90, 188, args.area1 ?? '', 'middle'),
    text('dg-muted-text', 230, 148, args.area2 ?? '', 'middle'),
    text('dg-muted-text', 160, 196, args.length ?? '', 'middle'),
    text('dg-label', 160, 226, `Volume ${disp(result, 'volume')}`, 'middle'),
  ].join('');
  const title = `Earthwork between end areas of ${args.area1} and ${args.area2}, ${args.length} apart: ${disp(result, 'volume')}. Schematic.`;
  return { markup: svg(body, title), desc: title };
}

/** Ground profile: elevation against distance, with the segments over the grade limit called out. */
function profileGrades(args, result) {
  const num = (v) => (typeof v === 'number' ? v : Number.parseFloat(String(v ?? '')));
  const pts = (Array.isArray(args.points) ? args.points : []).map((p) => [num(p.distance), num(p.elevation)]);
  const segs = result.result.segments ?? [];
  if (pts.length < 2 || pts.some((p) => !p.every(Number.isFinite)) || segs.length !== pts.length - 1) return null;
  // Distance and elevation are both lengths, but a profile is read with the
  // heights exaggerated; the drawing says by how much rather than implying
  // slopes it does not have.
  const span = (i, lo, hi) => {
    const [a, b] = [Math.min(...pts.map((p) => p[i])), Math.max(...pts.map((p) => p[i]))];
    const pad = (b - a) * 0.08 || 1;
    return { at: (v) => lo + ((v - a + pad) / (b - a + 2 * pad)) * (hi - lo), per: (hi - lo) / (b - a + 2 * pad) };
  };
  const [sx, sy] = [span(0, 45, 300), span(1, 195, 55)];
  const P = ([x, y]) => [sx.at(x), sy.at(y)];
  const exaggeration = Math.abs(sy.per / sx.per);
  const body = [
    line('dg-grid', [45, 195], [300, 195]),
    ...segs.map((seg, i) => {
      const over = seg.over_limit === 'yes';
      const [a, b] = [P(pts[i]), P(pts[i + 1])];
      const label = over ? text('dg-label', (a[0] + b[0]) / 2, Math.min(a[1], b[1]) - 8, `${seg.grade}% over limit`, 'middle') : '';
      return line(over ? 'dg-accent' : 'dg-muted', a, b) + label;
    }),
    ...pts.map((p) => dot(...P(p), 'dg-dot')),
    text('dg-muted-text', 45, 212, `Distance \u2192 · heights \u00d7${exaggeration.toFixed(1)}`),
    text('dg-muted-text', 12, 26, 'Elevation \u2191'),
    text('dg-muted-text', 300, 26, `Steepest ${disp(result, 'max_grade')}%`, 'end'),
  ].join('');
  const flagged = val(result, 'flagged');
  const title = `Ground profile over ${pts.length} points: steepest ${disp(result, 'max_grade')}%, average ${disp(result, 'average_grade')}%, ` +
    `${flagged ? `${flagged} segment${flagged === 1 ? '' : 's'} over the limit` : 'nothing over the limit'}. Heights exaggerated ${exaggeration.toFixed(1)} times.`;
  return { markup: svg(body, title), desc: title };
}

/** Borrow pit: the grid of nodes marked cut or fill, with the balance line through the crossings. */
function borrowPit(args, result) {
  const num = (v) => (typeof v === 'number' ? v : Number.parseFloat(String(v ?? '')));
  const rows = (Array.isArray(args.existing) ? args.existing : []).map((r) =>
    String(r.elevations ?? '').split(/[,\s]+/).filter(Boolean).map(Number),
  );
  const cell = measure(args.cell_size, LENGTH, 'ft');
  const grade = num(args.finished_grade);
  // The crossings come back in the result's own unit, the cell in the input's.
  const crossings = (result.result.balance_points ?? []).map((p) => [measure(`${p.x.value} ${p.x.unit}`, LENGTH, 'ft'), measure(`${p.y.value} ${p.y.unit}`, LENGTH, 'ft')]);
  if (rows.length < 2 || !cell || !Number.isFinite(grade) || rows.some((r) => r.length !== rows[0].length || r.some((x) => !Number.isFinite(x)))) return null;
  const [w, h] = [(rows[0].length - 1) * cell, (rows.length - 1) * cell];
  const k = Math.min(200 / w, 130 / h);
  const P = ([x, y]) => [60 + x * k, 50 + y * k];
  // The balance line runs through the crossings; nearest neighbour from the
  // westmost one chains them into the boundary between cut and fill.
  const left = [...crossings].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const chain = left.length ? [left.shift()] : [];
  while (left.length) {
    const i = left.reduce((best, p, j) => (Math.hypot(p[0] - chain.at(-1)[0], p[1] - chain.at(-1)[1]) < Math.hypot(left[best][0] - chain.at(-1)[0], left[best][1] - chain.at(-1)[1]) ? j : best), 0);
    chain.push(left.splice(i, 1)[0]);
  }
  const body = [
    ...rows.map((_, i) => line('dg-grid', P([0, i * cell]), P([w, i * cell]))),
    ...rows[0].map((_, j) => line('dg-grid', P([j * cell, 0]), P([j * cell, h]))),
    ...chain.slice(1).map((p, i) => line('dg-accent dg-dash', P(chain[i]), P(p))),
    ...chain.map((p) => dot(...P(p), 'dg-dot-now')),
    ...rows.flatMap((r, i) =>
      r.map((e, j) => {
        const d = e - grade;
        const [x, y] = P([j * cell, i * cell]);
        return text('dg-muted-text', x, y - 6, `${d > 0 ? '+' : ''}${d.toFixed(1)}`, 'middle');
      }),
    ),
    text('dg-label', 160, 22, `Cut ${disp(result, 'cut_volume')}, fill ${disp(result, 'fill_volume')}`, 'middle'),
    text('dg-muted-text', 60, 212, `+ is cut, \u2212 is fill, against ${args.finished_grade ?? 'the finished grade'}`),
  ].join('');
  const title = `Borrow pit over ${rows.length} by ${rows[0].length} nodes: cut ${disp(result, 'cut_volume')} against fill ${disp(result, 'fill_volume')}, net ${disp(result, 'net_cubic_yards')}. ` +
    (chain.length ? `The balance line crosses the grid at ${chain.length} points.` : 'Every node is on one side of the grade, so there is no balance line.');
  return { markup: svg(body, title), desc: title };
}

/** Circular curve, plan view: the two tangents meeting at the PI, the arc from PC to PT, and the long chord. */
function circularCurve(args, result) {
  const [R_, delta, T, L] = ['radius', 'delta', 'tangent', 'length'].map((k) => val(result, k));
  if (![R_, delta, T].every(Number.isFinite) || delta <= 0 || delta >= 180) return null;
  // Drawn as the surveyor sets it out: the back tangent along the page, the
  // forward tangent deflected by delta, and the arc tangent to both.
  const k = 105 / Math.max(T * 1.35, R_ * Math.tan((delta / 2) * R) * 1.35);
  const [t, r] = [T * k, R_ * k];
  const [px, py] = [160, 96 + r * (1 - Math.cos((delta / 2) * R))];
  const dir = (b, len, from = [px, py]) => [from[0] + len * Math.sin(b * R), from[1] - len * Math.cos(b * R)];
  // The back tangent runs into the PI from the left, the forward one leaves it deflected right.
  const back = dir(270, t);
  const fwd = dir(90 - delta, t);
  const pc = back;
  const pt = fwd;
  const body = [
    line('dg-grid dg-dash', dir(270, t * 1.5), [px, py]),
    line('dg-grid dg-dash', [px, py], dir(90 - delta, t * 1.5)),
    `<path class="dg-accent" d="M${pc.map(f1).join(' ')}A${f1(r)} ${f1(r)} 0 0 1 ${pt.map(f1).join(' ')}"/>`,
    line('dg-muted dg-dash', pc, pt),
    dot(...pc, 'dg-dot'),
    dot(px, py, 'dg-dot-now'),
    dot(...pt, 'dg-dot'),
    text('dg-label', pc[0], pc[1] + 18, `PC ${result.result.pc_station ?? ''}`, 'middle'),
    text('dg-label', px, py - 10, `PI ${args.pi_station ?? ''}`, 'middle'),
    text('dg-label', pt[0] + 6, pt[1] + 4, `PT ${result.result.pt_station ?? ''}`),
    text('dg-muted-text', 12, 214, `R ${disp(result, 'radius')} · \u0394 ${disp(result, 'delta')} · T ${disp(result, 'tangent')} · L ${disp(result, 'length')}`),
    text('dg-muted-text', 12, 230, `Chord ${disp(result, 'chord')} · middle ordinate ${disp(result, 'middle_ordinate')}`),
  ].join('');
  const title = `Circular curve in plan: a ${disp(result, 'delta')} deflection on a ${disp(result, 'radius')} radius, tangent ${disp(result, 'tangent')}, arc ${disp(result, 'length')} from PC ${result.result.pc_station ?? ''} to PT ${result.result.pt_station ?? ''}.`;
  return { markup: svg(body, title), desc: title };
}

/**
 * Spiral-curve-spiral in plan: the core's layout points, the back tangent
 * along the page into the PI and the ahead tangent out of it, the entry and
 * exit spirals dashed and the circular arc solid, each named point with its
 * station. One scale both ways, so the shape is the curve's own.
 */
function spiralCurve(args, result) {
  const r = result.result;
  const pts = (r.layout ?? []).map((p) => [p.x?.value, p.y?.value, p.point]);
  const T = val(result, 'total_tangent');
  if (pts.length < 4 || !Number.isFinite(T) || pts.some((p) => !Number.isFinite(p[0]) || !Number.isFinite(p[1]))) return null;
  const at = (name) => pts.findIndex((p) => p[2] === name);
  const [ts, sc, cs, st] = ['TS', 'SC', 'CS', 'ST'].map(at);
  if ([ts, sc, cs, st].some((i) => i < 0)) return null;
  const pi = [T, 0];
  const P = fit([...pts, pi], 230, 110);
  const chain = (from, to, cls) => {
    const seg = pts.slice(from, to + 1).map(P);
    return `<path class="${cls}" d="${seg.map((q, i) => `${i ? 'L' : 'M'}${f1(q[0])} ${f1(q[1])}`).join('')}"/>`;
  };
  const station = { TS: r.ts_station, SC: r.sc_station, CS: r.cs_station, ST: r.st_station };
  const name = (i, dx, dy, anchor = 'middle') => {
    const [x, y] = P(pts[i]);
    return dot(x, y) + text('dg-label', x + dx, y + dy, `${pts[i][2]} ${station[pts[i][2]] ?? ''}`, anchor);
  };
  const [px, py] = P(pi);
  const body = [
    line('dg-grid dg-dash', P(pts[ts]), [px, py]),
    line('dg-grid dg-dash', [px, py], P(pts[st])),
    chain(ts, sc, 'dg-accent dg-dash'),
    chain(sc, cs, 'dg-accent'),
    chain(cs, st, 'dg-accent dg-dash'),
    dot(px, py, 'dg-dot-now'),
    text('dg-label', px, py + 16, `PI ${args.pi_station ?? ''}`, 'middle'),
    name(ts, 0, 16), name(sc, -6, -8, 'end'), name(cs, -8, 4, 'end'), name(st, -8, -6, 'end'),
    text('dg-muted-text', 12, 214, `Ls ${args.spiral_length ?? ''} · R ${args.radius ?? ''} · Ts ${disp(result, 'total_tangent')}`),
    text('dg-muted-text', 12, 230, 'Dashed: spirals · solid: circular arc'),
  ].join('');
  const title = `Spiral-curve-spiral in plan: spiral from TS ${r.ts_station} to SC ${r.sc_station}, circular arc to CS ${r.cs_station}, spiral to ST ${r.st_station}; total tangent ${disp(result, 'total_tangent')}.`;
  return { markup: svg(body, title), desc: title };
}

// Drawings for the drone, flight, and plane-survey tools whose answer is a
// shape: what a photo covers, how photos overlap, which way a bearing points.
// Every length and angle drawn is the core's result or the reader's own input.

/** A typed number to 6 significant digits, so 141.421356237 reads 141.421. */
const tidy = (v) => (typeof v === 'number' ? Number(v.toPrecision(6)) : v);
/** COGO forward: from a known point along a direction and distance to the new point. */
function cogoForward(args, result) {
  const unit = unitOf(result, 'northing') || 'ft';
  const a = plane(args.northing, args.easting, unit);
  const b = plane(val(result, 'northing'), val(result, 'easting'), unit);
  if (![...a, ...b].every(Number.isFinite)) return null;
  const S = fit([a, b], 200, 100);
  const [p, q] = [S(a), S(b)];
  const body = [
    text('dg-muted-text', 296, 22, 'N ↑', 'end'),
    arrow(...p, ...q, 'dg-accent', '', 0.5, 1),
    dot(...p, 'dg-dot-now'), dot(...q),
    text('dg-muted-text', p[0], p[1] + 18, 'Start', 'middle'),
    text('dg-label', q[0], q[1] - 10, 'New point', 'middle'),
    text('dg-label', 160, 212, `${args.direction ?? ''}, ${typed(tidy(args.distance), unit)}`, 'middle'),
    text('dg-muted-text', 160, 228, `New point N ${disp(result, 'northing')}, E ${disp(result, 'easting')}`, 'middle'),
  ].join('');
  const title = `From the start point, ${args.direction ?? ''} for ${typed(tidy(args.distance), unit)} reaches northing ${disp(result, 'northing')}, easting ${disp(result, 'easting')}.`;
  return { markup: svg(body, title), desc: title };
}

/** COGO inverse: the bearing and distance between two known points. */
function cogoInverse(args, result) {
  const unit = unitOf(result, 'distance') || 'ft';
  const a = plane(args.northing1, args.easting1, unit);
  const b = plane(args.northing2, args.easting2, unit);
  if (![...a, ...b].every(Number.isFinite)) return null;
  const S = fit([a, b], 200, 100);
  const [p, q] = [S(a), S(b)];
  const body = [
    text('dg-muted-text', 296, 22, 'N ↑', 'end'),
    arrow(...p, ...q, 'dg-accent', '', 0.5, 1),
    dot(...p, 'dg-dot-now'), dot(...q),
    text('dg-muted-text', p[0], p[1] + 18, 'Point 1', 'middle'),
    text('dg-muted-text', q[0], q[1] - 10, 'Point 2', 'middle'),
    text('dg-label', 160, 228, `${disp(result, 'bearing')}, ${disp(result, 'distance')}`, 'middle'),
  ].join('');
  const title = `From point 1 to point 2: ${disp(result, 'bearing')}, ${disp(result, 'distance')}.`;
  return { markup: svg(body, title), desc: title };
}

/** A parcel in plan from its corner coordinates, numbered, with its area. */
function parcelPlan(pts, caption, title, extra = () => [], box = [220, 150], frame = pts) {
  if (pts.length < 3 || !pts.flat().every(Number.isFinite)) return null;
  const xy = pts.map(fit(frame, ...box));
  const body = [
    `<path class="dg-fill" d="${path(xy, true)}"/>`,
    `<path class="dg-accent" d="${path(xy, true)}"/>`,
    ...xy.map((p, i) => `${dot(p[0], p[1], i === 0 ? 'dg-dot-now' : 'dg-dot')}${text('dg-muted-text', p[0] + 7, p[1] - 7, String(i + 1))}`),
    text('dg-muted-text', 296, 22, 'N ↑', 'end'),
    text('dg-label', 160, 226, caption, 'middle'),
    ...extra(xy),
  ].join('');
  return { markup: svg(body, title), desc: title };
}

/**
 * The drawing of a traverse whose misclosure (dx, dy) leaves point `a`: the
 * round factor that makes it visible, and a mapper framed around both the
 * figure and the arrow's far end, so the arrow never runs off the drawing.
 */
function misclosureFrame(pts, a, [dx, dy], box) {
  const S0 = fit(pts, ...box);
  const k = Math.abs(S0([a[0] + 1, a[1]])[0] - S0(a)[0]);
  const factor = exaggeration(Math.hypot(dx, dy) * k, 40);
  const end = [a[0] + dx * factor, a[1] + dy * factor];
  const frame = [...pts, end];
  return { factor, end, frame, S: fit(frame, ...box) };
}

function areaPlan(args, result) {
  const pts = (Array.isArray(args.points) ? args.points : []).map((p) => [Number.parseFloat(p.easting), Number.parseFloat(p.northing)]);
  return parcelPlan(pts, `${disp(result, 'area')} · ${disp(result, 'acres')} · perimeter ${disp(result, 'perimeter')}`,
    `Parcel of ${pts.length} corners: ${disp(result, 'area')} (${disp(result, 'acres')}), perimeter ${disp(result, 'perimeter')}.`);
}
function traverseClosure(args, result) {
  const rows = result.result.adjusted ?? [];
  const pts = rows.map((p) => [val({ result: p }, 'easting'), val({ result: p }, 'northing')]);
  // The adjusted traverse closes on its first point; draw each corner once.
  const [a, z] = [pts[0], pts[pts.length - 1]];
  if (pts.length > 3 && a && z && Math.hypot(a[0] - z[0], a[1] - z[1]) < 1e-6) pts.pop();
  // The misclosure: where the unadjusted traverse ended, from the point of
  // beginning, by the core's sums of departures and latitudes. It is far
  // smaller than the figure, so it is drawn larger by a stated round factor.
  const [dep, lat] = [val(result, 'sum_departures'), val(result, 'sum_latitudes')];
  // Closed to within 1e-12 of the length is a perfect closure (cogo-and-traverse).
  const miss = [dep, lat].every(Number.isFinite) && Math.hypot(dep, lat) > 1e-12 * (val(result, 'total_length') || 1);
  if (pts.length < 3 || !pts.flat().every(Number.isFinite)) return null;
  // Shorter than the area plan's box, so the leg labels clear the caption.
  const box = [220, 128];
  const { factor, end: far, frame, S } = miss ? misclosureFrame(pts, a, [dep, lat], box) : { factor: 1, frame: pts, S: fit(pts, ...box) };
  const extra = (xy) => {
    // Each course's adjusted length, set just outside the figure beside its leg.
    const c = [xy.reduce((t, p) => t + p[0], 0) / xy.length, xy.reduce((t, p) => t + p[1], 0) / xy.length];
    const legs = rows.length <= 13 ? rows.slice(1).map((r, i) => {
      const d = val({ result: r }, 'distance');
      const [p, q] = [xy[i], xy[(i + 1) % xy.length]];
      if (!Number.isFinite(d) || !p || !q) return '';
      const m = [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
      const len = Math.hypot(q[0] - p[0], q[1] - p[1]) || 1;
      let n = [-(q[1] - p[1]) / len, (q[0] - p[0]) / len];
      if (n[0] * (m[0] - c[0]) + n[1] * (m[1] - c[1]) < 0) n = [-n[0], -n[1]];
      const anchor = Math.abs(n[0]) > Math.abs(n[1]) ? (n[0] > 0 ? 'start' : 'end') : 'middle';
      return text('dg-muted-text', m[0] + n[0] * 8, m[1] + n[1] * 8 + (n[1] > 0.5 ? 9 : Math.abs(n[1]) <= 0.5 ? 4 : 0), String(Number(d.toFixed(2))), anchor);
    }) : [];
    if (!miss) return legs;
    const [o, end] = [S(a), S(far)];
    return [...legs, arrow(o[0], o[1], end[0], end[1], 'dg-muted', factor > 1 ? `Misclosure ×${group(factor)}` : 'Misclosure', 0.9, dep >= 0 ? 1 : -1)];
  };
  const scale = factor > 1 ? `, drawn ${group(factor)} times its size` : '';
  return parcelPlan(pts, `Precision ${disp(result, 'precision')} · misclosure ${disp(result, 'misclosure')} before adjustment`,
    `Adjusted traverse of ${pts.length} corners. Before adjustment it missed closing by ${disp(result, 'misclosure')} toward ${disp(result, 'misclosure_bearing')}${scale}, a precision of ${disp(result, 'precision')}.`, extra, box, frame);
}

const group = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ',');

export const DIAGRAMS = {
  'survey.gnss.dop': gnssSky,
  'survey.curves.vertical-curve': verticalCurve,
  'survey.land.deed-plot': traverseSketch,
  'survey.earthwork.profile-grades': profileGrades,
  'survey.curves.spiral': spiralCurve,
  'survey.earthwork.borrow-pit': borrowPit,
  'survey.curves.circular-curve': circularCurve,
  'survey.earthwork.average-end-area': endAreas,
  'survey.earthwork.prismoidal': endAreas,
  'survey.cogo.forward': cogoForward,
  'survey.cogo.inverse': cogoInverse,
  'survey.cogo.area-by-coordinates': areaPlan,
  'survey.cogo.traverse-closure': traverseClosure,
};
