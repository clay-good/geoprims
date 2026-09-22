// Vector diagrams (web/map-canvas "Canvas modes": vector diagram): unit-free
// engineering drawings for tools whose answer is a set of vectors. Each is
// SVG markup with CSS classes for color, so every display mode applies and
// nothing needs inline styles. Values come from the core's result wherever it
// has them; inputs are read only for what the result does not echo.

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const R = Math.PI / 180;

/** Speeds and lengths in base units (m/s, m) from "120 kt", "10 m/s", or a bare number in `unit`. */
const SPEED = { kt: 1852 / 3600, kts: 1852 / 3600, knots: 1852 / 3600, mph: 0.44704, 'km/h': 1 / 3.6, kph: 1 / 3.6, 'm/s': 1, 'ft/s': 0.3048 };
const LENGTH = { m: 1, km: 1000, nm: 1852, ft: 0.3048, mi: 1609.344, sm: 1609.344 };
const AREA = { m2: 1, ft2: 0.09290304, yd2: 0.83612736, ftus2: 0.09290341161 };
export function measure(v, table, unit) {
  if (typeof v === 'number') return v * table[unit];
  const m = /^\s*([-+]?[\d.]+(?:e[-+]?\d+)?)\s*([a-z/][a-z0-9/]*)?\s*$/i.exec(String(v ?? ''));
  if (!m) return null;
  const k = table[(m[2] || unit).toLowerCase()];
  return k ? Number(m[1]) * k : null;
}
const deg = (v) => {
  const n = Number.parseFloat(String(v ?? '').replace(/[°º]/g, ' '));
  return Number.isFinite(n) ? n : null;
};
const val = (result, k) => {
  const v = result.result?.[k];
  return typeof v === 'number' ? v : v?.value;
};
const disp = (result, k) => result.display?.[k] ?? '';
const unitOf = (result, k) => result.result?.[k]?.unit ?? '';

/** Compass vector: bearing (deg true) and length → SVG dx, dy (y down). */
const vec = (bearing, len) => [len * Math.sin(bearing * R), -len * Math.cos(bearing * R)];

/**
 * The scope the marker ids belong to. A page may draw the same diagram twice —
 * compact under the answer and full-size in the canvas — and two elements may
 * not share an id, so each drawing sets this before it builds its markup.
 */
let scope = '';

const headId = (cls) => `dg-head-${cls.includes('accent') ? 'accent' : 'muted'}${scope}`;

function arrow(x1, y1, x2, y2, cls, label, labelAt = 0.55, side = -1) {
  // The label sits beside the vector, `side` -1 to its left and 1 to its right.
  const len = Math.hypot(x2 - x1, y2 - y1) || 1;
  const [nx, ny] = [(-(y2 - y1) / len) * side, ((x2 - x1) / len) * side];
  const [lx, ly] = [x1 + (x2 - x1) * labelAt + nx * 12, y1 + (y2 - y1) * labelAt + ny * 12];
  const anchor = Math.abs(nx) > Math.abs(ny) ? (nx > 0 ? 'start' : 'end') : 'middle';
  const t = label ? `<text class="dg-label" text-anchor="${anchor}" x="${lx.toFixed(1)}" y="${(ly + (ny > 0.5 ? 10 : ny < -0.5 ? -2 : 4)).toFixed(1)}">${esc(label)}</text>` : '';
  return `<line class="dg-casing" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/><line class="${cls}" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" marker-end="url(#${headId(cls)})"/>${t}`;
}

/** Scales and centers points (in any units, y up) into the 320 × 240 drawing, returning a mapper. */
function fit(points, w = 220, h = 150) {
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const k = Math.min(w / Math.max(Math.max(...xs) - Math.min(...xs), 1e-9), h / Math.max(Math.max(...ys) - Math.min(...ys), 1e-9));
  const [mx, my] = [(Math.max(...xs) + Math.min(...xs)) / 2, (Math.max(...ys) + Math.min(...ys)) / 2];
  return ([x, y]) => [160 + (x - mx) * k, 128 - (y - my) * k];
}

function svg(body, title) {
  return `<svg viewBox="0 0 320 240" class="dg" role="img" aria-label="${esc(title)}" xmlns="http://www.w3.org/2000/svg"><defs>` +
    `<marker id="dg-head-accent${scope}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="dg-head-accent" d="M0 0L10 5L0 10z"/></marker>` +
    `<marker id="dg-head-muted${scope}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="dg-head-muted" d="M0 0L10 5L0 10z"/></marker>` +
    `</defs>${body}</svg>`;
}

/** Wind triangle: air vector (heading, TAS) + wind = ground vector (course, GS). */
function windTriangle(args, result) {
  const heading = val(result, 'heading');
  const course = deg(args.course);
  const tas = measure(args.tas, SPEED, 'kt');
  const gs = result.result.groundspeed ? measure(`${val(result, 'groundspeed')} ${result.result.groundspeed.unit}`, SPEED, 'kt') : null;
  if ([heading, course, tas, gs].some((x) => x === null || x === undefined || !Number.isFinite(x))) return null;
  // Compass vectors with y up: the air vector, then the wind, reach the ground vector's end.
  const up = (b, len) => [len * Math.sin(b * R), len * Math.cos(b * R)];
  const O = [0, 0];
  const A = up(heading, tas);
  const G = up(course, gs);
  const S = fit([O, A, G]);
  const [o, a, g] = [S(O), S(A), S(G)];
  const body = [
    `<text class="dg-muted-text" x="12" y="22">N ↑</text>`,
    arrow(...o, ...a, 'dg-muted', `Heading ${disp(result, 'heading')}`, 0.5, -1),
    arrow(...a, ...g, 'dg-muted dg-dash', 'Wind', 0.5, -1),
    arrow(...o, ...g, 'dg-accent', `Ground ${disp(result, 'groundspeed')}`, 0.5, 1),
  ].join('');
  const title = `Wind triangle: heading ${disp(result, 'heading')} and true airspeed plus the wind give a ground track at ${disp(result, 'groundspeed')}, a wind correction of ${disp(result, 'wind_correction_angle')}.`;
  return { markup: svg(body, title), desc: title };
}

/** Runway wind components: the wind arrow and its headwind and crosswind parts. */
function runwayComponents(args, result) {
  const rwy = val(result, 'runway_heading');
  const wdir = deg(args.wind_direction);
  const hw = val(result, 'headwind');
  const xw = val(result, 'crosswind');
  if ([rwy, wdir, hw, xw].some((x) => typeof x !== 'number' || !Number.isFinite(x))) return null;
  const [cx, cy] = [160, 125];
  const [rx, ry] = vec(rwy, 95);
  const total = Math.hypot(hw, xw) || 1;
  const k = 85 / total;
  // The wind blows from wdir, so the arrow points toward wdir + 180.
  const [wx, wy] = vec(wdir + 180, total * k);
  const [hx, hy] = vec(rwy + (hw >= 0 ? 180 : 0), Math.abs(hw) * k);
  const side = (((wdir - rwy) % 360) + 360) % 360 < 180 ? 90 : -90; // crosswind from the right pushes left
  const [xx, xy] = vec(rwy - side, Math.abs(xw) * k);
  const body = [
    `<line class="dg-runway" x1="${(cx - rx).toFixed(1)}" y1="${(cy - ry).toFixed(1)}" x2="${(cx + rx).toFixed(1)}" y2="${(cy + ry).toFixed(1)}"/>`,
    `<text class="dg-muted-text" x="12" y="22">N ↑ · runway ${esc(disp(result, 'runway_heading'))}</text>`,
    arrow(cx - wx, cy - wy, cx, cy, 'dg-muted dg-dash', 'Wind', 0.3),
    arrow(cx, cy, cx + hx, cy + hy, 'dg-accent', `${hw >= 0 ? 'Headwind' : 'Tailwind'} ${disp(result, 'headwind')}`),
    arrow(cx, cy, cx + xx, cy + xy, 'dg-accent', `Crosswind ${disp(result, 'crosswind')}`),
  ].join('');
  const title = `Runway ${disp(result, 'runway_heading')}: ${hw >= 0 ? 'headwind' : 'tailwind'} ${disp(result, 'headwind')} and crosswind ${disp(result, 'crosswind')}.`;
  return { markup: svg(body, title), desc: title };
}

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

/** Sky plot: the target's azimuth and elevation seen from the origin (zenith at the center). */
function skyPlot(args, result) {
  const az = val(result, 'azimuth');
  const el = val(result, 'elevation');
  if (![az, el].every(Number.isFinite)) return null;
  const [cx, cy, R0] = [160, 122, 96];
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

const f1 = (n) => n.toFixed(1);
const line = (cls, a, b) => `<line class="${cls}" x1="${f1(a[0])}" y1="${f1(a[1])}" x2="${f1(b[0])}" y2="${f1(b[1])}"/>`;
const text = (cls, x, y, t, anchor = 'start') => `<text class="${cls}" text-anchor="${anchor}" x="${f1(x)}" y="${f1(y)}">${esc(t)}</text>`;
const dot = (x, y, cls = 'dg-dot') => `<circle class="${cls}" cx="${f1(x)}" cy="${f1(y)}" r="4.5"/>`;

/** Descent profile: cruise, then a straight descent from the top of descent to the target altitude. */
function descentProfile(args, result) {
  const d = val(result, 'distance');
  if (!Number.isFinite(d) || d <= 0) return null;
  const [x0, x1, xe, yTop, yBot] = [24, 92, 300, 60, 176];
  const body = [
    line('dg-grid', [x0, yBot + 32], [xe, yBot + 32]),
    `<line class="dg-muted dg-dash" x1="${x0}" y1="${yTop}" x2="${x1}" y2="${yTop}"/>`,
    line('dg-casing', [x1, yTop], [xe, yBot]),
    line('dg-accent', [x1, yTop], [xe, yBot]),
    dot(x1, yTop),
    dot(xe, yBot, 'dg-dot-now'),
    text('dg-label', x1, yTop - 12, 'Top of descent', 'middle'),
    text('dg-muted-text', x0, yTop + 16, args.from_altitude ?? ''),
    text('dg-muted-text', xe, yBot + 18, args.to_altitude ?? '', 'end'),
    // The distance, measured along the ground under the descent.
    line('dg-muted', [x1, yBot + 32], [xe, yBot + 32]),
    line('dg-muted', [x1, yBot + 26], [x1, yBot + 38]),
    line('dg-muted', [xe, yBot + 26], [xe, yBot + 38]),
    text('dg-label', (x1 + xe) / 2, yBot + 52, disp(result, 'distance'), 'middle'),
    text('dg-muted-text', (x1 + xe) / 2 - 10, (yTop + yBot) / 2 + 22, `${disp(result, 'descent_angle')} · ${disp(result, 'vertical_speed')}`, 'end'),
  ].join('');
  const title = `Descent profile: start down ${disp(result, 'distance')} before the target, descending at ${disp(result, 'descent_angle')} and ${disp(result, 'vertical_speed')}. Not to scale vertically.`;
  return { markup: svg(body, title), desc: title };
}

/** Approach profile: level at the MDA, then down a constant angle from the VDP to the threshold. */
function approachProfile(args, result) {
  const d = val(result, 'distance');
  if (!Number.isFinite(d) || d <= 0) return null;
  const [x0, xv, xt, yMda, yRwy] = [20, 150, 262, 70, 190];
  const tch = yRwy - 14;
  const body = [
    `<line class="dg-runway" x1="${xt}" y1="${yRwy}" x2="304" y2="${yRwy}"/>`,
    line('dg-grid', [x0, yRwy], [xt, yRwy]),
    `<line class="dg-muted dg-dash" x1="${x0}" y1="${yMda}" x2="${xv}" y2="${yMda}"/>`,
    line('dg-casing', [xv, yMda], [xt, tch]),
    line('dg-accent', [xv, yMda], [xt, tch]),
    dot(xv, yMda),
    text('dg-label', xv, yMda - 12, 'VDP', 'middle'),
    text('dg-muted-text', x0, yMda + 18, 'MDA'),
    text('dg-muted-text', x0, yMda + 32, `${args.height_above_touchdown ?? ''} above touchdown`),
    text('dg-muted-text', xt, yRwy + 16, 'Threshold', 'middle'),
    line('dg-muted', [xv, yRwy + 28], [xt, yRwy + 28]),
    line('dg-muted', [xv, yRwy + 22], [xv, yRwy + 34]),
    line('dg-muted', [xt, yRwy + 22], [xt, yRwy + 34]),
    text('dg-label', (xv + xt) / 2, yRwy + 44, disp(result, 'distance'), 'middle'),
    text('dg-muted-text', (xv + xt) / 2 + 8, (yMda + tch) / 2 - 8, disp(result, 'vertical_speed')),
  ].join('');
  const title = `Approach profile: leave the MDA at the visual descent point, ${disp(result, 'distance')} from the threshold, descending at ${disp(result, 'vertical_speed')}. Not to scale vertically.`;
  return { markup: svg(body, title), desc: title };
}

/** A station like "7+00.00" as a number (700). */
export const station = (s) => {
  const m = /^\s*(-?\d+)\+(\d+(?:\.\d+)?)\s*$/.exec(String(s ?? ''));
  return m ? Number(m[1]) * 100 + Math.sign(Number(m[1]) || 1) * Number(m[2]) : Number.parseFloat(s);
};

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
  const S = fit(pts, 220, 150);
  const xy = pts.map(S);
  const path = xy.map((p, i) => `${i ? 'L' : 'M'}${p.map(f1).join(' ')}`).join('');
  const [first, last] = [xy[0], xy[xy.length - 1]];
  const body = [
    `<path class="dg-casing" fill="none" d="${path}"/><path class="dg-accent" d="${path}"/>`,
    ...xy.slice(0, -1).map((p, i) => `${dot(p[0], p[1], i === 0 ? 'dg-dot-now' : 'dg-dot')}${text('dg-muted-text', p[0] + 7, p[1] - 7, String(i + 1))}`),
    // The misclosure is usually far smaller than a pixel: ring it so it can be found.
    `<circle class="dg-muted dg-dash" cx="${f1((first[0] + last[0]) / 2)}" cy="${f1((first[1] + last[1]) / 2)}" r="12"/>`,
    text('dg-label', 160, 226, `Misclosure ${disp(result, 'misclosure')} · ${result.result.precision ?? ''}`, 'middle'),
    text('dg-muted-text', 296, 30, 'N ↑', 'end'),
  ].join('');
  const title = `Traverse sketch of ${pts.length - 1} courses; it misses closing by ${disp(result, 'misclosure')} (${result.result.precision ?? ''}), ringed at the point of beginning.`;
  return { markup: svg(body, title), desc: title };
}

/** Airspeed gauge: calibrated and true airspeed on one dial, with the Mach number. */
function airspeedGauge(args, result) {
  const [cas, tas] = [val(result, 'cas'), val(result, 'tas')];
  if (!Number.isFinite(cas) || !Number.isFinite(tas) || tas <= 0) return null;
  const unit = result.result.tas.unit;
  // A round top, so the ten ticks land on round numbers.
  // The V-speeds the pilot entered, as the marked arcs of an airspeed
  // indicator. Each is named in words as well as drawn, so the meaning never
  // rides on the color: an arc alone tells a color-blind pilot nothing.
  const v = Object.fromEntries(['vs0', 'vs1', 'vfe', 'vno', 'vne'].map((k) => [k, measure(args[k], SPEED, unit)]).filter(([, x]) => Number.isFinite(x) && x > 0));
  const asUnit = (x) => x / (SPEED[unit.toLowerCase()] ?? 1);
  const highest = Math.max(cas, tas, ...Object.values(v).map(asUnit));
  const step = highest * 1.2 > 200 ? 100 : 50;
  const top = Math.ceil((highest * 1.2) / step) * step;
  const [cx, cy, r] = [160, 108, 84];
  // 0 at the bottom left, the top of the scale at the bottom right: a 270° dial.
  const at = (v) => -135 + (270 * Math.min(v, top)) / top;
  const pt = (v, rr) => { const a = at(v) * R; return [cx + rr * Math.sin(a), cy - rr * Math.cos(a)]; };
  const ticks = [];
  for (let v = 0; v <= top; v += top / 10) {
    const [a, b] = [pt(v, r), pt(v, r - 8)];
    ticks.push(line('dg-muted', a, b), text('dg-muted-text', ...pt(v, r - 20).map((q, i) => q + (i ? 4 : 0)), String(Math.round(v)), 'middle'));
  }
  const arc = (v0, v1, cls = 'dg-grid', rr = r) => { const [a, b] = [pt(v0, rr), pt(v1, rr)]; return `<path class="${cls}" d="M${a.map(f1).join(' ')}A${rr} ${rr} 0 ${at(v1) - at(v0) > 180 ? 1 : 0} 1 ${b.map(f1).join(' ')}"/>`; };
  const marks = [];
  const band = (a, b, cls, name) => {
    if (!Number.isFinite(a) || !Number.isFinite(b) || b <= a) return;
    const [ra, rb] = [asUnit(a), asUnit(b)];
    marks.push(arc(ra, rb, cls, r - 4), text('dg-muted-text', ...pt((ra + rb) / 2, r - 34).map((q, i) => q + (i ? 4 : 0)), name, 'middle'));
  };
  band(v.vs0, v.vfe, 'dg-arc-white', 'Flaps');
  band(v.vs1, v.vno, 'dg-arc-green', 'Normal');
  band(v.vno, v.vne, 'dg-arc-yellow', 'Caution');
  if (Number.isFinite(v.vne)) {
    const [a, b] = [pt(asUnit(v.vne), r - 12), pt(asUnit(v.vne), r + 2)];
    marks.push(line('dg-arc-red', a, b), text('dg-muted-text', ...pt(asUnit(v.vne), r - 34).map((q, i) => q + (i ? 4 : 0)), 'Never exceed', 'middle'));
  }
  const needle = (v, cls) => arrow(cx, cy, ...pt(v, r - 32), cls, '');
  const body = [
    arc(0, top),
    ...marks,
    ...ticks,
    needle(cas, 'dg-muted'),
    needle(tas, 'dg-accent'),
    `<circle class="dg-dot-now" cx="${cx}" cy="${cy}" r="4"/>`,
    text('dg-muted-text', cx, cy + 24, unit, 'middle'),
    text('dg-label', cx, 216, `TAS ${disp(result, 'tas')}`, 'middle'),
    text('dg-muted-text', cx, 232, `CAS ${disp(result, 'cas')} · Mach ${disp(result, 'mach')}`, 'middle'),
  ].join('');
  const title = `Airspeed dial: calibrated airspeed ${disp(result, 'cas')} and true airspeed ${disp(result, 'tas')}, Mach ${disp(result, 'mach')}.`;
  return { markup: svg(body, title), desc: title };
}

/** ISA temperature profile: the standard lapse to the tropopause, then isothermal, with this altitude marked. */
function isaProfile(args, result) {
  const t = val(result, 'temperature');
  const hFt = val(result, 'geopotential_altitude');
  const unit = result.result.geopotential_altitude?.unit;
  if (!Number.isFinite(t) || !Number.isFinite(hFt)) return null;
  const km = unit === 'ft' ? (hFt * 0.3048) / 1000 : unit === 'm' ? hFt / 1000 : hFt;
  // The standard's two lowest layers: +15 °C at sea level, -6.5 °C/km to 11 km, then -56.5 °C to 20 km.
  const std = [[15, 0], [-56.5, 11], [-56.5, 20]];
  const X = (c) => 40 + ((c + 70) / 100) * 240;
  const Y = (k) => 200 - (Math.min(k, 20) / 20) * 170;
  const path = std.map(([c, k], i) => `${i ? 'L' : 'M'}${f1(X(c))} ${f1(Y(k))}`).join('');
  const inRange = km >= 0 && km <= 20;
  const body = [
    line('dg-grid', [40, 200], [280, 200]),
    line('dg-grid', [X(0), 30], [X(0), 200]),
    text('dg-muted-text', X(0) + 4, 42, '0 °C'),
    `<line class="dg-grid dg-dash" x1="40" y1="${f1(Y(11))}" x2="280" y2="${f1(Y(11))}"/>`,
    text('dg-muted-text', 278, Y(11) - 5, 'Tropopause, 11 km', 'end'),
    `<path class="dg-muted" d="${path}"/>`,
    inRange ? `${line('dg-grid', [40, Y(km)], [X(t), Y(km)])}${dot(X(t), Y(km))}${text('dg-label', X(t) + 10, Y(km) + 4, `${disp(result, 'temperature')} at ${disp(result, 'geopotential_altitude')}`)}` : '',
    text('dg-muted-text', 40, 216, 'Temperature →'),
    text('dg-muted-text', 44, 26, 'Altitude ↑'),
  ].join('');
  const title = `Standard atmosphere: ${disp(result, 'temperature')} at ${disp(result, 'geopotential_altitude')}, on the standard temperature profile (${result.result.layer}).`;
  return { markup: svg(body, title), desc: title };
}

/** Climb gradient: the rise over one nautical mile, the angle, and the vertical speed. */
function climbTriangle(args, result) {
  if (!Number.isFinite(val(result, 'gradient'))) return null;
  const [a, b, c] = [[50, 190], [270, 190], [270, 80]];
  const body = [
    line('dg-muted', a, b),
    line('dg-muted dg-dash', b, c),
    line('dg-casing', a, c), line('dg-accent', a, c),
    dot(...a), dot(...c),
    text('dg-label', 160, 210, '1 NM', 'middle'),
    text('dg-label', 262, 140, disp(result, 'gradient'), 'end'),
    text('dg-muted-text', 96, 182, disp(result, 'angle')),
    text('dg-muted-text', 50, 60, `${disp(result, 'vertical_speed')} · ${disp(result, 'gradient_percent')}`),
  ].join('');
  const title = `Climb of ${disp(result, 'gradient')} (${disp(result, 'gradient_percent')}, ${disp(result, 'angle')}), ${disp(result, 'vertical_speed')} at this groundspeed. Not to scale vertically.`;
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

/** Weight and balance: the CG envelope with the takeoff point, the burn path, and the landing point. */
function cgEnvelope(args, result) {
  const num = (v) => (typeof v === 'number' ? v : Number.parseFloat(String(v ?? '')));
  const rows = Array.isArray(args.envelope) ? args.envelope : [];
  const corners = rows.map((r) => [num(r.arm), num(r.weight)]).filter((p) => p.every(Number.isFinite));
  const to = [val(result, 'cg'), val(result, 'total_weight')];
  const land = [val(result, 'landing_cg'), val(result, 'landing_weight')];
  if (corners.length < 3 || !to.every(Number.isFinite)) return null;
  const burn = land.every(Number.isFinite) && land[1] > 0;
  // Arms and weights are different quantities, so each axis gets its own
  // scale: a shared one would squeeze the envelope into a sliver.
  const pts = [...corners, to, ...(burn ? [land] : [])];
  const span = (i, lo, hi) => {
    const [a, b] = [Math.min(...pts.map((p) => p[i])), Math.max(...pts.map((p) => p[i]))];
    const pad = (b - a) * 0.12 || 1;
    return (v) => lo + ((v - a + pad) / (b - a + 2 * pad)) * (hi - lo);
  };
  const [sx, sy] = [span(0, 45, 290), span(1, 200, 60)];
  const S = ([x, y]) => [sx(x), sy(y)];
  const ring = corners.map(S);
  const [t, l] = [S(to), burn ? S(land) : null];
  const body = [
    `<polygon class="dg-grid" points="${ring.map((q) => q.map(f1).join(',')).join(' ')}"/>`,
    burn ? line('dg-muted dg-dash', t, l) : '',
    burn ? dot(...l) : '',
    dot(...t, 'dg-dot-now'),
    text('dg-label', t[0], t[1] - 10, `Takeoff ${disp(result, 'total_weight')} at ${disp(result, 'cg')} (${result.result.takeoff_status})`, 'middle'),
    burn ? text('dg-muted-text', l[0], l[1] + 18, `Landing ${disp(result, 'landing_weight')} at ${disp(result, 'landing_cg')} (${result.result.landing_status})`, 'middle') : '',
    text('dg-muted-text', 30, 226, `Arm (${result.result.cg.unit}) \u2192`),
    text('dg-muted-text', 30, 24, `Weight (${result.result.total_weight.unit}) \u2191`),
  ].join('');
  const title = `Center of gravity envelope: takeoff at ${disp(result, 'cg')} and ${disp(result, 'total_weight')}, ${result.result.takeoff_status} the envelope` +
    (burn ? `, moving to ${disp(result, 'landing_cg')} at ${disp(result, 'landing_weight')} after the burn, ${result.result.landing_status} it.` : '.');
  return { markup: svg(body, title), desc: title };
}

/** A three-pointer altimeter face reading `alt`: the hundreds hand, the thousands hand, and the ten-thousands pointer. */
function altimeterFace(alt, unit) {
  const [cx, cy, r] = [62, 120, 50];
  const hand = (turns, len, cls, head) => {
    const a = turns * 360 * R;
    return head
      ? `<path class="${cls}" d="M${f1(cx + len * Math.sin(a))} ${f1(cy - len * Math.cos(a))}L${f1(cx + 6 * Math.cos(a))} ${f1(cy + 6 * Math.sin(a))}L${f1(cx - 6 * Math.cos(a))} ${f1(cy - 6 * Math.sin(a))}z"/>`
      : line(cls, [cx, cy], [cx + len * Math.sin(a), cy - len * Math.cos(a)]);
  };
  const ticks = [];
  for (let i = 0; i < 10; i += 1) {
    const a = (i / 10) * 360 * R;
    ticks.push(line('dg-muted', [cx + r * Math.sin(a), cy - r * Math.cos(a)], [cx + (r - 7) * Math.sin(a), cy - (r - 7) * Math.cos(a)]));
    ticks.push(text('dg-muted-text', cx + (r - 18) * Math.sin(a), cy - (r - 18) * Math.cos(a) + 4, String(i), 'middle'));
  }
  return [
    `<circle class="dg-grid" cx="${cx}" cy="${cy}" r="${r}" fill="none"/>`,
    ...ticks,
    hand(alt / 100000, r - 26, 'dg-muted', true), // ten thousands
    hand(alt / 10000, r - 22, 'dg-muted'), // thousands
    hand(alt / 1000, r - 8, 'dg-accent'), // hundreds
    `<circle class="dg-dot-now" cx="${cx}" cy="${cy}" r="3.5"/>`,
    text('dg-muted-text', cx, cy + 68, `Altimeter, ${unit}`, 'middle'),
  ];
}

/** Altimetry: the indicated altitude against the true altitude above the setting's station. */
function altimetry(args, result) {
  const num = (v) => (typeof v === 'number' ? v : Number.parseFloat(String(v ?? '')));
  const truth = val(result, 'true_altitude');
  const err = val(result, 'error');
  const ind = truth - err;
  const base = Number.isFinite(num(args.station_elevation)) ? num(args.station_elevation) : 0;
  if (![truth, err, ind].every(Number.isFinite) || truth <= base) return null;
  // Drawn to scale over a window around the two altitudes, since the gap
  // between them is a small part of the height: a few hundred feet in eight
  // thousand. The window leaves the ground out, so the axis is cut and the
  // cut is drawn, and nothing reads as a full column from the station up.
  const pad = Math.max(Math.abs(err) * 0.8, (Math.max(ind, truth) - base) * 0.02);
  const [lo, hi] = [Math.min(ind, truth) - pad, Math.max(ind, truth) + pad];
  const Y = (h) => 186 - ((h - lo) / (hi - lo)) * 126;
  const [ti, tt] = [Y(ind), Y(truth)];
  const body = [
    ...altimeterFace(ind, unitOf(result, 'true_altitude')),
    line('dg-grid', [130, 196], [300, 196]),
    `<path class="dg-grid" d="M130 192L160 192L168 186L184 198L192 192L300 192"/>`,
    text('dg-muted-text', 130, 212, `Station${base ? ` ${args.station_elevation}` : ' (sea level)'}, axis cut`),
    `<line class="dg-muted dg-dash" x1="150" y1="${f1(ti)}" x2="230" y2="${f1(ti)}"/>`,
    text('dg-muted-text', 234, ti + 4, `Indicated ${args.indicated ?? ''}`),
    line('dg-accent', [150, tt], [230, tt]),
    text('dg-label', 234, tt + 4, `True ${disp(result, 'true_altitude')}`),
    // The gap between the two, measured and named.
    arrow(170, ti, 170, tt, 'dg-accent', `${disp(result, 'error')}`, 0.5, err < 0 ? -1 : 1),
    text('dg-muted-text', 12, 24, `ISA deviation ${args.isa_deviation ?? ''}: true altitude ${err < 0 ? 'below' : 'above'} indicated`),
  ].join('');
  const title = `Altimetry: an indicated ${args.indicated} in air ${args.isa_deviation} from standard is a true ${disp(result, 'true_altitude')}, ${disp(result, 'error')} against the altimeter.`;
  return { markup: svg(body, title), desc: title };
}

/** Vector sum: the vectors head to tail in the east-north plane, and the resultant from the origin. */
function vectorSum(args, result) {
  const num = (v) => (typeof v === 'number' ? v : Number.parseFloat(String(v ?? '')));
  const rows = (Array.isArray(args.vectors) ? args.vectors : []).map((r) => [num(r.x), num(r.y), num(r.z) || 0]);
  if (rows.length < 1 || !rows.every((p) => Number.isFinite(p[0]) && Number.isFinite(p[1]))) return null;
  const sum = [val(result, 'x'), val(result, 'y')];
  if (!sum.every(Number.isFinite)) return null;
  // Head to tail: each vector starts where the last one ended.
  const chain = [[0, 0]];
  for (const [x, y] of rows) chain.push([chain.at(-1)[0] + x, chain.at(-1)[1] + y]);
  const S = fit([...chain, sum, [0, 0]]);
  const pts = chain.map(S);
  const flat = rows.every((p) => p[2] === 0);
  const body = [
    line('dg-grid', S([Math.min(...chain.map((p) => p[0])), 0]), S([Math.max(...chain.map((p) => p[0])), 0])),
    line('dg-grid', S([0, Math.min(...chain.map((p) => p[1]))]), S([0, Math.max(...chain.map((p) => p[1]))])),
    ...rows.map((v, i) => arrow(...pts[i], ...pts[i + 1], 'dg-muted', `(${v[0]}, ${v[1]}${flat ? '' : `, ${v[2]}`})`, 0.5, -1)),
    arrow(...S([0, 0]), ...S(sum), 'dg-accent', `Sum ${disp(result, 'magnitude')} at ${disp(result, 'direction')}`, 0.6, 1),
    text('dg-muted-text', 12, 22, flat ? 'East →, north ↑' : 'East →, north ↑, up in the labels'),
  ].join('');
  const title = `${rows.length} vectors head to tail in the east-north plane: the sum is ${disp(result, 'magnitude')} at ${disp(result, 'direction')}, components ${disp(result, 'x')} east and ${disp(result, 'y')} north.`;
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

const DIAGRAMS = {
  'geodesy.frame.to-local': skyPlot,
  'aviation.wind.heading-groundspeed': windTriangle,
  'aviation.wind.runway-components': runwayComponents,
  'navigation.route.cpa': cpa,
  'navigation.route.fly-by': flyBy,
  'aviation.performance.top-of-descent': descentProfile,
  'aviation.performance.vdp': approachProfile,
  'survey.curves.vertical-curve': verticalCurve,
  'survey.land.deed-plot': traverseSketch,
  'aviation.airspeed.cas-to-tas': airspeedGauge,
  'aviation.airspeed.tas-to-cas': airspeedGauge,
  'aviation.atmosphere.isa': isaProfile,
  'aviation.loading.weight-balance': cgEnvelope,
  'aviation.altimetry.true-altitude': altimetry,
  'navigation.vector.operations': vectorSum,
  'survey.earthwork.profile-grades': profileGrades,
  'aviation.performance.climb-gradient': climbTriangle,
  'navigation.los.horizon': horizonSketch,
  'navigation.los.visibility': sightLine,
  'navigation.los.fresnel': fresnelZone,
  'survey.earthwork.average-end-area': endAreas,
  'survey.earthwork.prismoidal': endAreas,
};

/**
 * The diagram for a tool's result, or null: { markup, desc }.
 * `at` names this drawing, so a second copy of the same diagram on one page
 * carries its own marker ids.
 */
export function diagram(id, args, result, at = '') {
  const f = DIAGRAMS[id];
  if (!f || !result?.ok) return null;
  scope = at ? `-${at}` : '';
  try {
    return f(args, result);
  } catch {
    return null;
  } finally {
    scope = '';
  }
}

export const DIAGRAM_TOOLS = Object.keys(DIAGRAMS);
