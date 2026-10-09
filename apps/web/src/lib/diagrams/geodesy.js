// The geodesy tools' diagrams. See diagrams.js for what a diagram is.

import { arrow, deg, disp, dot, esc, f1, line, path, svg, text, val, vec } from './kit.js';

/** Sky plot: the target's azimuth and elevation seen from the origin (zenith at the center). */
function skyPlot(args, result) {
  const az = val(result, 'azimuth');
  const el = val(result, 'elevation');
  if (![az, el].every(Number.isFinite)) return null;
  const [cx, cy, R0] = [160, 114, 90];
  const below = el < 0;
  // Radius grows from the zenith (0) to the horizon (R0); below the horizon the dot sits on the rim.
  const r = below ? R0 : (R0 * (90 - el)) / 90;
  const [dx, dy] = vec(az, r);
  const rings = [0, 30, 60].map((e) => `<circle class="${e === 0 ? 'dg-muted' : 'dg-grid'}" cx="${cx}" cy="${cy}" r="${((R0 * (90 - e)) / 90).toFixed(1)}" fill="none"/>`).join('');
  const ticks = [['N', 0], ['E', 90], ['S', 180], ['W', 270]]
    .map(([t, b]) => {
      const [tx, ty] = vec(b, R0 + 12);
      return `<text class="dg-muted-text" text-anchor="middle" x="${(cx + tx).toFixed(1)}" y="${(cy + ty + 4).toFixed(1)}">${t}</text>`;
    })
    .join('');
  const ringLabels = [30, 60].map((e) => `<text class="dg-muted-text" x="${(cx + 3).toFixed(1)}" y="${(cy - (R0 * (90 - e)) / 90 - 3).toFixed(1)}">${e}°</text>`).join('');
  const label = `Az ${disp(result, 'azimuth')}, El ${disp(result, 'elevation')}${below ? ' (below the horizon)' : ''}`;
  const body = [
    rings,
    ringLabels,
    ticks,
    r > 1 ? arrow(cx, cy, cx + dx, cy + dy, 'dg-accent', '', 0.5) : '',
    `<circle class="dg-dot" cx="${(cx + dx).toFixed(1)}" cy="${(cy + dy).toFixed(1)}" r="5"/>`,
    `<text class="dg-label" text-anchor="middle" x="160" y="236">${esc(label)}</text>`,
  ].join('');
  const title = `Sky plot: the target is at azimuth ${disp(result, 'azimuth')} and elevation ${disp(result, 'elevation')}${below ? ', below the horizon' : ''}, ${disp(result, 'range')} away.`;
  return { markup: svg(body, title), desc: title };
}

/** Height references at a point: the ellipsoid, the geoid above or below it, the terrain, and the height itself. */
function heightStack(args, result) {
  const h = val(result, 'ellipsoidal');
  const H = val(result, 'orthometric');
  const N = val(result, 'geoid_height');
  const agl = val(result, 'agl');
  if (![h, H, N].every(Number.isFinite)) return null;
  // Everything is measured from the ellipsoid, which is the drawing's datum.
  const ground = Number.isFinite(agl) ? H - agl + N : null;
  const marks = [0, N, h, ...(ground === null ? [] : [ground])];
  const [lo, hi] = [Math.min(...marks), Math.max(...marks)];
  const pad = (hi - lo) * 0.15 || 1;
  const Y = (v) => 190 - ((v - lo + pad) / (hi - lo + 2 * pad)) * 150;
  // Each surface is named just above its right end, so a long name stays in the drawing.
  const rule = (v, cls, label) => `${line(cls, [40, Y(v)], [300, Y(v)])}${text('dg-muted-text', 300, Y(v) - 4, label, 'end')}`;
  // The geoid is drawn wavy, because it is: it is an equipotential surface,
  // not a second ellipsoid.
  const wave = (v) => {
    const y = Y(v);
    let d = `M40 ${f1(y)}`;
    for (let x = 40; x < 285; x += 35) d += `q17.5 ${x % 70 === 40 ? -4 : 4} 35 0`;
    return `<path class="dg-muted dg-dash" d="${d}"/>${text('dg-muted-text', 300, y - 7, `Geoid, N ${disp(result, 'geoid_height')}`, 'end')}`;
  };
  const body = [
    rule(0, 'dg-grid', 'Ellipsoid'),
    wave(N),
    ground === null ? '' : rule(ground, 'dg-runway', `Terrain ${args.terrain ?? ''}`),
    dot(145, Y(h), 'dg-dot-now'),
    text('dg-label', 145, Y(h) - 10, `Here: ${disp(result, 'ellipsoidal')} above the ellipsoid`, 'middle'),
    arrow(60, Y(0), 60, Y(h), 'dg-accent', `h ${disp(result, 'ellipsoidal')}`, 0.5, -1),
    arrow(100, Y(N), 100, Y(h), 'dg-accent', `H ${disp(result, 'orthometric')}`, 0.5, 1),
    ground === null ? '' : arrow(200, Y(ground), 200, Y(h), 'dg-accent', `AGL ${disp(result, 'agl')}`, 0.5, 1),
  ].join('');
  const title = `Height references here: ${disp(result, 'ellipsoidal')} above the ellipsoid, ${disp(result, 'orthometric')} above the geoid, which is itself ${disp(result, 'geoid_height')} above the ellipsoid` +
    (ground === null ? '.' : `, and ${disp(result, 'agl')} above the terrain${agl < 0 ? ', which puts the point underground' : ''}.`);
  return { markup: svg(body, title), desc: title };
}

/** True and magnetic north, and one bearing measured from each. */
function magneticCompass(args, result) {
  const v = val(result, 'variation_used');
  const out = val(result, 'result');
  const input = deg(args.bearing);
  if (![v, out, input].every(Number.isFinite)) return null;
  const toTrue = args.direction === 'magnetic-to-true';
  const [t, m] = toTrue ? [out, input] : [input, out];
  const [cx, cy, r] = [160, 120, 80];
  const at = (b, len) => [cx + vec(b, len)[0], cy + vec(b, len)[1]];
  const round = (x) => `${Math.round(((x % 360) + 360) % 360)}°`;
  const body = [
    `<circle class="dg-grid" cx="${cx}" cy="${cy}" r="${r}"/>`,
    arrow(cx, cy, ...at(0, r + 12), 'dg-muted', 'True north', 0.95, 1),
    arrow(cx, cy, ...at(v, r), 'dg-muted dg-dash', 'Magnetic north', 0.9, v < 0 ? -1 : 1),
    arrow(cx, cy, ...at(t, r + 18), 'dg-accent', `${round(t)} true · ${round(m)} magnetic`, 0.75, 1),
    dot(cx, cy, 'dg-dot-now'),
    text('dg-muted-text', 160, 228, `Variation ${disp(result, 'variation_text')}`, 'middle'),
  ].join('');
  const title = `The bearing is ${round(t)} from true north and ${round(m)} from magnetic north, which lies ${disp(result, 'variation_text')} of true.`;
  return { markup: svg(body, title), desc: title };
}

/**
 * Datum shift: where the same point's coordinates land in the target frame,
 * drawn from the source frame's position as east and north parts, head to tail.
 */
function datumShift(args, result) {
  const [e, n, u, s, az] = ['east', 'north', 'up', 'shift', 'azimuth'].map((k) => val(result, k));
  if (![e, n, u, s, az].every(Number.isFinite) || s < 1e-6) return null;
  // Scaled so the shift is 110 units long, centered on its midpoint.
  const k = 110 / s;
  const [cx, cy] = [160 - (e * k) / 2, 118 + (n * k) / 2];
  const [ex, ny] = [cx + e * k, cy - n * k];
  const abs = (key) => disp(result, key).replace(/^-/, '');
  const ew = `${abs('east')} ${e >= 0 ? 'east' : 'west'}`;
  const ns = `${abs('north')} ${n >= 0 ? 'north' : 'south'}`;
  const ud = `${abs('up')} ${u >= 0 ? 'up' : 'down'}`;
  const [from, to] = [String(args.from ?? 'the first frame'), String(args.to ?? 'the second frame')];
  const body = [
    `<text class="dg-muted-text" x="12" y="22">N ↑</text>`,
    arrow(cx, cy, ex, cy, 'dg-muted dg-dash', ew, 0.5, n >= 0 ? 1 : -1),
    arrow(ex, cy, ex, ny, 'dg-muted dg-dash', ns, 0.5, e >= 0 ? -1 : 1),
    arrow(cx, cy, ex, ny, 'dg-accent', `${disp(result, 'shift')} toward ${disp(result, 'azimuth')}`, 0.45, e * n >= 0 ? -1 : 1),
    dot(cx, cy),
    text('dg-muted-text', 160, 228, `${from} → ${to} · ${ud}`, 'middle'),
  ].join('');
  const title = `The ${to} coordinates lie ${disp(result, 'shift')} from the ${from} ones, toward ${disp(result, 'azimuth')}: ${ew}, ${ns}, and ${ud}.`;
  return { markup: svg(body, title), desc: title };
}

/** Bearing difference: the shorter turn from one bearing to the other. */
function bearingTurn(args, result) {
  const [a, b, d] = [deg(args.from), deg(args.to), val(result, 'difference')];
  if (![a, b, d].every(Number.isFinite)) return null;
  const [cx, cy, r] = [160, 118, 78];
  const at = (x, len) => [cx + vec(x, len)[0], cy + vec(x, len)[1]];
  const turn = ((b - a + 540) % 360) - 180;
  const [p, q] = [at(a, 44), at(a + turn, 44)];
  const body = [
    `<circle class="dg-grid" cx="${cx}" cy="${cy}" r="${r}"/>`,
    text('dg-muted-text', cx, cy - r - 6, 'N', 'middle'),
    arrow(cx, cy, ...at(a, r), 'dg-muted', `From ${Math.round(a)}°`, 0.8, turn > 0 ? -1 : 1),
    arrow(cx, cy, ...at(b, r), 'dg-muted', `To ${Math.round(b)}°`, 0.8, turn > 0 ? 1 : -1),
    `<path class="dg-accent" d="M${f1(p[0])} ${f1(p[1])}A44 44 0 0 ${turn > 0 ? 1 : 0} ${f1(q[0])} ${f1(q[1])}"/>`,
    dot(cx, cy, 'dg-dot-now'),
    text('dg-label', 160, 228, result.summary ?? '', 'middle'),
  ].join('');
  const title = `From ${Math.round(a)}° to ${Math.round(b)}°: ${result.summary ?? ''}`;
  return { markup: svg(body, title), desc: title };
}

export const DIAGRAMS = {
  'geodesy.frame.to-local': skyPlot,
  'geodesy.height.convert': heightStack,
  'geodesy.magnetic.true-to-magnetic': magneticCompass,
  'geodesy.datum.nad83': datumShift,
  'geodesy.parse.bearing-difference': bearingTurn,
};
