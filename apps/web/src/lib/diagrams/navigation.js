// The navigation tools' diagrams. See diagrams.js for what a diagram is.

import { LENGTH, R, SPEED, arrow, deg, disp, dot, esc, f1, fit, kt, line, measure, path, plane, svg, text, typed, val, vec } from './kit.js';

/** Closest point of approach: both tracks from now to the CPA moment. */
function cpa(args, result) {
  const t = val(result, 'time');
  const va = [deg(args.a_course), measure(args.a_speed, SPEED, 'kt')];
  const vb = [deg(args.b_course), measure(args.b_speed, SPEED, 'kt')];
  const b0 = [measure(args.b_east, LENGTH, 'm'), measure(args.b_north, LENGTH, 'm')];
  if ([t, ...va, ...vb, ...b0].some((x) => x === null || !Number.isFinite(x))) return null;
  const pos = ([course, speed], [x, y], s) => [x + speed * s * Math.sin(course * R), y + speed * s * Math.cos(course * R)];
  const end = Math.max(t * 1.4, 1);
  const pts = [[0, 0], pos(va, [0, 0], end), b0, pos(vb, b0, end), pos(va, [0, 0], t), pos(vb, b0, t)];
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  const span = Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys), 1);
  const k = 190 / span;
  const mx = (Math.max(...xs) + Math.min(...xs)) / 2;
  const my = (Math.max(...ys) + Math.min(...ys)) / 2;
  const S = ([x, y]) => [160 + (x - mx) * k, 125 - (y - my) * k];
  const [a0, a1, bb0, b1, ac, bc] = pts.map(S);
  const body = [
    `<text class="dg-muted-text" x="12" y="22">N ↑</text>`,
    arrow(...a0, ...a1, 'dg-muted', 'A', 0.1),
    arrow(...bb0, ...b1, 'dg-muted', 'B', 0.1),
    `<line class="dg-accent dg-dash" x1="${ac[0].toFixed(1)}" y1="${ac[1].toFixed(1)}" x2="${bc[0].toFixed(1)}" y2="${bc[1].toFixed(1)}"/>`,
    `<circle class="dg-dot" cx="${ac[0].toFixed(1)}" cy="${ac[1].toFixed(1)}" r="4"/><circle class="dg-dot" cx="${bc[0].toFixed(1)}" cy="${bc[1].toFixed(1)}" r="4"/>`,
    `<text class="dg-label" x="${((ac[0] + bc[0]) / 2 + 8).toFixed(1)}" y="${((ac[1] + bc[1]) / 2).toFixed(1)}">CPA ${esc(disp(result, 'separation'))} in ${esc(disp(result, 'time'))}</text>`,
  ].join('');
  // At a scene time (playback), the core's positions then, with their separation.
  let scene = '';
  const ax = val(result, 'a_east_at');
  if (typeof ax === 'number') {
    const toM = (k) => measure(`${val(result, k)} ${result.result[k].unit}`, LENGTH, 'm');
    const [pa, pb] = [S([toM('a_east_at'), toM('a_north_at')]), S([toM('b_east_at'), toM('b_north_at')])];
    const at = measure(args.at_time, { s: 1, min: 60, h: 3600 }, 's');
    scene = `<line class="dg-muted" x1="${pa[0].toFixed(1)}" y1="${pa[1].toFixed(1)}" x2="${pb[0].toFixed(1)}" y2="${pb[1].toFixed(1)}"/>` +
      `<circle class="dg-dot-now" cx="${pa[0].toFixed(1)}" cy="${pa[1].toFixed(1)}" r="5"/><circle class="dg-dot-now" cx="${pb[0].toFixed(1)}" cy="${pb[1].toFixed(1)}" r="5"/>` +
      `<text class="dg-muted-text" x="12" y="228">t = ${esc(at === null ? '' : `${Math.round(at)} s`)} · ${esc(disp(result, 'separation_at'))} apart</text>`;
  }
  const title = `Closest point of approach: ${disp(result, 'separation')} apart in ${disp(result, 'time')}.` + (scene ? ` At the scene time they are ${disp(result, 'separation_at')} apart.` : '');
  return { markup: svg(body + scene, title), desc: title };
}

/** Fly-by turn: inbound and outbound legs, the turn arc, and the lead distance. */
function flyBy(args, result) {
  const inb = deg(args.inbound);
  const outb = deg(args.outbound);
  const turn = val(result, 'turn_angle');
  const right = result.result.direction === 'right';
  if ([inb, outb, turn].some((x) => x === null || !Number.isFinite(x)) || turn < 1) return null;
  const L = 110;
  const lead = Math.min(L * 0.9, 45 * Math.tan((turn / 2) * R));
  const radius = lead / Math.tan((turn / 2) * R);
  const [wx, wy] = [170, 140];
  const [ix, iy] = vec(inb + 180, L);
  const [ox, oy] = vec(outb, L);
  const [sx, sy] = vec(inb + 180, lead);
  const [ex, ey] = vec(outb, lead);
  const body = [
    `<text class="dg-muted-text" x="12" y="22">N ↑</text>`,
    arrow(wx + ix, wy + iy, wx, wy, 'dg-muted', 'Inbound', 0.2),
    arrow(wx, wy, wx + ox, wy + oy, 'dg-muted', 'Outbound', 0.8),
    `<path class="dg-accent" d="M${(wx + sx).toFixed(1)} ${(wy + sy).toFixed(1)} A${radius.toFixed(1)} ${radius.toFixed(1)} 0 0 ${right ? 1 : 0} ${(wx + ex).toFixed(1)} ${(wy + ey).toFixed(1)}"/>`,
    `<circle class="dg-dot" cx="${wx}" cy="${wy}" r="4"/>`,
    `<text class="dg-label" x="${(wx + sx + 8).toFixed(1)}" y="${(wy + sy + 16).toFixed(1)}">Start turn ${esc(disp(result, 'lead_distance'))} before</text>`,
  ].join('');
  const title = `Fly-by turn of ${disp(result, 'turn_angle')} to the ${result.result.direction}: start ${disp(result, 'lead_distance')} before the waypoint, radius ${disp(result, 'radius')}.`;
  return { markup: svg(body, title), desc: title };
}

/** The earth's curve as an arc across the drawing, and a height above it at x. */
const EARTH = { cx: 160, cy: 760, r: 600 };
const onEarth = (x) => [x, EARTH.cy - Math.sqrt(EARTH.r ** 2 - (x - EARTH.cx) ** 2)];
const earthArc = () => { const [a, b] = [onEarth(10), onEarth(310)]; return `<path class="dg-grid" d="M${f1(a[0])} ${f1(a[1])}A${EARTH.r} ${EARTH.r} 0 0 1 ${f1(b[0])} ${f1(b[1])}"/>`; };

/** Distance to the horizon: a line from the eye, tangent to the curve. */
function horizonSketch(args, result) {
  if (!Number.isFinite(val(result, 'optical'))) return null;
  const g = onEarth(60);
  const eye = [60, g[1] - 60];
  const t = onEarth(240);
  const body = [
    earthArc(),
    line('dg-muted', g, eye),
    line('dg-casing', eye, t), line('dg-accent', eye, t),
    dot(...eye), dot(t[0], t[1], 'dg-dot-now'),
    text('dg-muted-text', eye[0] - 6, eye[1] + 30, args.height ?? '', 'end'),
    text('dg-label', 160, 40, `${disp(result, 'optical')} to the horizon, with refraction`, 'middle'),
    text('dg-muted-text', 160, 226, `Geometric ${disp(result, 'geometric')} · radio ${disp(result, 'radio')}`, 'middle'),
  ].join('');
  const title = `Horizon: ${disp(result, 'optical')} away with standard refraction, ${disp(result, 'geometric')} geometrically. Schematic, not to scale.`;
  return { markup: svg(body, title), desc: title };
}

/** Line of sight: observer and target over the bulge of the earth between them. */
function sightLine(args, result) {
  const visible = result.result.visible;
  if (!visible) return null;
  const [o, t] = [onEarth(40), onEarth(280)];
  const [eye, top] = [[40, o[1] - 24], [280, t[1] - 90]];
  const hidden = val(result, 'hidden_height');
  const body = [
    earthArc(),
    line('dg-muted', o, eye), line('dg-muted', t, top),
    line('dg-casing', eye, top), line(visible === 'yes' ? 'dg-accent' : 'dg-muted dg-dash', eye, top),
    dot(...eye), dot(...top),
    text('dg-muted-text', eye[0] + 8, eye[1] + 18, 'Observer'),
    text('dg-muted-text', top[0] - 6, top[1] - 8, 'Target', 'end'),
    Number.isFinite(hidden) && hidden > 0 ? `<line class="dg-muted dg-dash" x1="${top[0] - 10}" y1="${f1(t[1])}" x2="${top[0] - 10}" y2="${f1(t[1] - 40)}"/>${text('dg-muted-text', top[0] - 16, t[1] - 20, `${disp(result, 'hidden_height')} hidden`, 'end')}` : '',
    text('dg-label', 160, 226, visible === 'yes' ? `In sight · ${disp(result, 'midpoint_clearance')} clear at midpoint` : 'Below the horizon', 'middle'),
  ].join('');
  const title = `Line of sight: ${visible === 'yes' ? 'the target is in sight' : 'the target is hidden'}; the lowest ${disp(result, 'hidden_height')} of it is below the horizon. Schematic, not to scale.`;
  return { markup: svg(body, title), desc: title };
}

/** Fresnel zone: the first zone's ellipse between two antennas, with the clearance it needs. */
function fresnelZone(args, result) {
  if (!Number.isFinite(val(result, 'fresnel_radius'))) return null;
  const [a, b] = [[40, 90], [280, 90]];
  const ry = 44;
  const body = [
    earthArc(),
    `<ellipse class="dg-muted" cx="160" cy="90" rx="120" ry="${ry}"/>`,
    `<ellipse class="dg-grid dg-dash" cx="160" cy="90" rx="120" ry="${f1(ry * 0.6)}"/>`,
    line('dg-casing', a, b), line('dg-accent', a, b),
    dot(...a), dot(...b),
    line('dg-muted', [160, 90], [160, 90 + ry]),
    text('dg-label', 166, 90 + ry / 2 + 4, `${disp(result, 'fresnel_radius')} radius`),
    text('dg-muted-text', 160, 30, `Keep ${disp(result, 'required_clearance')} clear below the path`, 'middle'),
    text('dg-muted-text', 160, 226, `60% of the zone ${disp(result, 'clearance_60')} · earth bulge ${disp(result, 'earth_bulge')}`, 'middle'),
  ].join('');
  const title = `First Fresnel zone: ${disp(result, 'fresnel_radius')} radius at midpath; keep ${disp(result, 'required_clearance')} clear, 60% of the zone plus ${disp(result, 'earth_bulge')} of earth bulge. Schematic, not to scale.`;
  return { markup: svg(body, title), desc: title };
}

/** Vector sum: the vectors head to tail in the east-north plane, and the resultant from the origin. */
/**
 * Vectors head to tail (vector-3d "Vector diagrams"). A result with an up
 * component can be turned and tilted (`view`: turn about up, and the tilt
 * the scene is seen from, 90° straight down); the default is the plan view.
 */
function vectorSum(args, result, view = {}) {
  const num = (v) => (typeof v === 'number' ? v : Number.parseFloat(String(v ?? '')));
  const rows = (Array.isArray(args.vectors) ? args.vectors : []).map((r) => [num(r.x), num(r.y), num(r.z) || 0]);
  if (rows.length < 1 || !rows.every((p) => Number.isFinite(p[0]) && Number.isFinite(p[1]))) return null;
  const sum = [val(result, 'x'), val(result, 'y'), val(result, 'z') || 0];
  if (!sum.slice(0, 2).every(Number.isFinite)) return null;
  const flat = rows.every((p) => p[2] === 0);
  const turn = flat ? 0 : (view.turn ?? 0);
  const tilt = flat ? 90 : (view.tilt ?? 90);
  const plan = turn === 0 && tilt === 90;
  // East, north, up onto the drawing: turn about up, then look down at the tilt.
  const [ct, st, sv, cv] = [Math.cos(turn * R), Math.sin(turn * R), Math.sin(tilt * R), Math.cos(tilt * R)];
  const P = ([e, n, u]) => [e * ct - n * st, (e * st + n * ct) * sv + u * cv];
  // Head to tail: each vector starts where the last one ended.
  const chain = [[0, 0, 0]];
  for (const [x, y, z] of rows) chain.push([chain.at(-1)[0] + x, chain.at(-1)[1] + y, chain.at(-1)[2] + z]);
  const ext = Math.max(...chain.flat().map(Math.abs), 1e-9);
  const axes = flat ? [[1, 0, 0], [0, 1, 0]] : [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
  const S = fit([...chain.map(P), P(sum), [0, 0], ...(plan ? [] : axes.map((a) => P(a.map((c) => c * ext))))]);
  const pts = chain.map((q) => S(P(q)));
  const o = S([0, 0]);
  const body = [
    ...(plan
      ? [
          line('dg-grid', S([Math.min(...chain.map((p) => p[0])), 0]), S([Math.max(...chain.map((p) => p[0])), 0])),
          line('dg-grid', S([0, Math.min(...chain.map((p) => p[1]))]), S([0, Math.max(...chain.map((p) => p[1]))])),
        ]
      : axes.flatMap((a, k) => {
          const end = S(P(a.map((c) => c * ext)));
          // The label sits just past the axis's end, along it, clear of the arrows.
          const d = Math.hypot(end[0] - o[0], end[1] - o[1]) || 1;
          const [lx, ly] = [end[0] + ((end[0] - o[0]) / d) * 12, end[1] + ((end[1] - o[1]) / d) * 12 + 4];
          return [line('dg-grid', o, end), text('dg-muted-text', lx, ly, ['E', 'N', 'Up'][k], 'middle')];
        })),
    ...rows.map((v, i) => arrow(...pts[i], ...pts[i + 1], 'dg-muted', `(${v[0]}, ${v[1]}${flat ? '' : `, ${v[2]}`})`, 0.5, -1)),
    arrow(...o, ...S(P(sum)), 'dg-accent', `Sum ${disp(result, 'magnitude')} at ${disp(result, 'direction')}`, 0.6, 1),
    text('dg-muted-text', 12, 22, flat ? 'East →, north ↑' : plan ? 'East →, north ↑, up in the labels; turn or tilt to see up' : `Turned ${Math.round(turn)}°, seen from ${Math.round(tilt)}° above`),
  ].join('');
  const title = `${rows.length} vectors head to tail ${flat || plan ? 'in the east-north plane' : 'in east, north, and up'}: the sum is ${disp(result, 'magnitude')} at ${disp(result, 'direction')}, components ${disp(result, 'x')} east and ${disp(result, 'y')} north${flat ? '' : ` and ${disp(result, 'z')} up`}.`;
  return { markup: svg(body, title), desc: title, rotatable: !flat };
}

/** Dip of the horizon: eye level against the line to the horizon, not to scale. */
function horizonDip(args, result) {
  if (!Number.isFinite(val(result, 'dip'))) return null;
  const g = onEarth(60);
  const eye = [60, g[1] - 60];
  const t = onEarth(250);
  const body = [
    earthArc(),
    line('dg-muted', g, eye),
    line('dg-muted dg-dash', eye, [300, eye[1]]),
    text('dg-muted-text', 300, eye[1] - 6, 'Eye level', 'end'),
    line('dg-casing', eye, t), line('dg-accent', eye, t),
    dot(...eye), dot(t[0], t[1], 'dg-dot-now'),
    text('dg-muted-text', eye[0] - 6, eye[1] + 30, typed(args.height, 'm'), 'end'),
    text('dg-label', 160, 40, `The horizon dips ${disp(result, 'dip')} below eye level`, 'middle'),
    text('dg-muted-text', 160, 226, `Rule of thumb ${disp(result, 'rule')} · off by ${disp(result, 'rule_error')}`, 'middle'),
  ].join('');
  const title = `From ${typed(args.height, 'm')} up, the horizon dips ${disp(result, 'dip')} below eye level. Schematic, not to scale.`;
  return { markup: svg(body, title), desc: title };
}

// ---- drone mission planning (add-flight-and-drone-planning-tools) ----

export const DIAGRAMS = {
  'navigation.route.cpa': cpa,
  'navigation.route.fly-by': flyBy,
  'navigation.vector.operations': vectorSum,
  'navigation.los.horizon': horizonSketch,
  'navigation.los.visibility': sightLine,
  'navigation.los.fresnel': fresnelZone,
  'navigation.los.dip': horizonDip,
};
