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
export function measure(v, table, unit) {
  if (typeof v === 'number') return v * table[unit];
  const m = /^\s*([-+]?[\d.]+(?:e[-+]?\d+)?)\s*([a-z/]*)\s*$/i.exec(String(v ?? ''));
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

/** Compass vector: bearing (deg true) and length → SVG dx, dy (y down). */
const vec = (bearing, len) => [len * Math.sin(bearing * R), -len * Math.cos(bearing * R)];

function arrow(x1, y1, x2, y2, cls, label, labelAt = 0.55, side = -1) {
  // The label sits beside the vector, `side` -1 to its left and 1 to its right.
  const len = Math.hypot(x2 - x1, y2 - y1) || 1;
  const [nx, ny] = [(-(y2 - y1) / len) * side, ((x2 - x1) / len) * side];
  const [lx, ly] = [x1 + (x2 - x1) * labelAt + nx * 12, y1 + (y2 - y1) * labelAt + ny * 12];
  const anchor = Math.abs(nx) > Math.abs(ny) ? (nx > 0 ? 'start' : 'end') : 'middle';
  const t = label ? `<text class="dg-label" text-anchor="${anchor}" x="${lx.toFixed(1)}" y="${(ly + (ny > 0.5 ? 10 : ny < -0.5 ? -2 : 4)).toFixed(1)}">${esc(label)}</text>` : '';
  return `<line class="dg-casing" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/><line class="${cls}" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" marker-end="url(#dg-head-${cls.includes('accent') ? 'accent' : 'muted'})"/>${t}`;
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
    `<marker id="dg-head-accent" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="dg-head-accent" d="M0 0L10 5L0 10z"/></marker>` +
    `<marker id="dg-head-muted" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="dg-head-muted" d="M0 0L10 5L0 10z"/></marker>` +
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
  const title = `Closest point of approach: ${disp(result, 'separation')} apart in ${disp(result, 'time')}.`;
  return { markup: svg(body, title), desc: title };
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

const DIAGRAMS = {
  'aviation.wind.heading-groundspeed': windTriangle,
  'aviation.wind.runway-components': runwayComponents,
  'navigation.route.cpa': cpa,
  'navigation.route.fly-by': flyBy,
};

/** The diagram for a tool's result, or null: { markup, desc }. */
export function diagram(id, args, result) {
  const f = DIAGRAMS[id];
  if (!f || !result?.ok) return null;
  try {
    return f(args, result);
  } catch {
    return null;
  }
}

export const DIAGRAM_TOOLS = Object.keys(DIAGRAMS);
