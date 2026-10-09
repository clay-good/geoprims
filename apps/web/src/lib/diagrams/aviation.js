// The aviation tools' diagrams. See diagrams.js for what a diagram is.

import { R, SPEED, arrow, deg, disp, dot, esc, f1, fit, kt, line, measure, path, svg, text, typed, unitOf, val, vec } from './kit.js';

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

/**
 * Airspeed gauge: calibrated and true airspeed on one dial, with the Mach
 * number. Without a temperature there is no true airspeed, so the second
 * needle is the equivalent airspeed, which needs none.
 */
function airspeedGauge(args, result) {
  const key = Number.isFinite(val(result, 'tas')) ? 'tas' : 'eas';
  const [cas, tas] = [val(result, 'cas'), val(result, key)];
  if (!Number.isFinite(cas) || !Number.isFinite(tas) || tas <= 0) return null;
  const unit = result.result[key].unit;
  const name = key === 'tas' ? 'true' : 'equivalent';
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
    text('dg-label', cx, 216, `${key.toUpperCase()} ${disp(result, key)}`, 'middle'),
    text('dg-muted-text', cx, 232, `CAS ${disp(result, 'cas')} · Mach ${disp(result, 'mach')}`, 'middle'),
  ].join('');
  const title = `Airspeed dial: calibrated airspeed ${disp(result, 'cas')} and ${name} airspeed ${disp(result, key)}, Mach ${disp(result, 'mach')}.`;
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

/** Climb to cruise, side view: from the runway up to the top of climb, then level. Not to scale vertically. */
function climbProfile(args, result) {
  const toc = val(result, 'top_of_climb');
  if (!Number.isFinite(toc) || toc <= 0) return null;
  const [x0, x1, xe, yTop, yBot] = [24, 220, 300, 70, 176];
  const fuel = result.result.fuel ? ` · ${disp(result, 'fuel')}` : '';
  const body = [
    line('dg-runway', [x0, yBot], [x0 + 36, yBot]),
    line('dg-casing', [x0 + 36, yBot], [x1, yTop]),
    line('dg-accent', [x0 + 36, yBot], [x1, yTop]),
    `<line class="dg-muted dg-dash" x1="${x1}" y1="${yTop}" x2="${xe}" y2="${yTop}"/>`,
    dot(x1, yTop),
    text('dg-label', x1, yTop - 12, 'Top of climb', 'middle'),
    text('dg-muted-text', x0, yBot + 18, typed(args.field_elevation, 'ft')),
    text('dg-muted-text', xe, yTop - 12, typed(args.cruise_altitude, 'ft'), 'end'),
    // The distance to the top of climb, measured along the ground under the climb.
    line('dg-muted', [x0 + 36, yBot + 32], [x1, yBot + 32]),
    line('dg-muted', [x0 + 36, yBot + 26], [x0 + 36, yBot + 38]),
    line('dg-muted', [x1, yBot + 26], [x1, yBot + 38]),
    text('dg-label', (x0 + 36 + x1) / 2, yBot + 52, disp(result, 'top_of_climb'), 'middle'),
    text('dg-muted-text', (x0 + 36 + x1) / 2 + 10, (yTop + yBot) / 2 + 26, `${disp(result, 'time')}${fuel}`),
  ].join('');
  const title = `Climb from ${typed(args.field_elevation, 'ft')} to ${typed(args.cruise_altitude, 'ft')}: top of climb ${disp(result, 'top_of_climb')} from departure after ${disp(result, 'time')}${result.result.fuel ? `, using ${disp(result, 'fuel')}` : ''}. Not to scale vertically.`;
  return { markup: svg(body, title), desc: title };
}

/** The leg from departure to destination, with the equal time point and the point of no return on it. */
function etpLine(args, result) {
  const etp = val(result, 'etp_distance');
  const rest = val(result, 'etp_to_destination');
  const pnr = result.result.pnr_distance ? val(result, 'pnr_distance') : null;
  if (![etp, rest].every(Number.isFinite) || etp + rest <= 0) return null;
  const total = etp + rest;
  const [x0, xe, y] = [30, 290, 120];
  const X = (d) => x0 + ((xe - x0) * Math.min(d, total)) / total;
  const beyond = pnr !== null && pnr > total;
  const body = [
    line('dg-casing', [x0, y], [xe, y]),
    line('dg-muted', [x0, y], [xe, y]),
    dot(x0, y, 'dg-dot-now'),
    dot(xe, y, 'dg-dot-now'),
    text('dg-muted-text', x0, y + 22, 'Departure', 'middle'),
    text('dg-muted-text', xe, y + 22, 'Destination', 'end'),
    line('dg-accent', [X(etp), y - 18], [X(etp), y + 18]),
    text('dg-label', X(etp), y - 26, `ETP ${disp(result, 'etp_distance')}`, 'middle'),
    pnr !== null && !beyond ? line('dg-accent dg-dash', [X(pnr), y - 18], [X(pnr), y + 40]) : '',
    pnr !== null ? text('dg-label', beyond ? xe : X(pnr), y + 56, beyond ? `PNR ${disp(result, 'pnr_distance')}, past the destination` : `PNR ${disp(result, 'pnr_distance')}`, beyond ? 'end' : 'middle') : '',
    text('dg-muted-text', x0, 200, `On at ${disp(result, 'groundspeed_out')}, back at ${disp(result, 'groundspeed_back')}`),
  ].join('');
  const title = `Equal time point ${disp(result, 'etp_distance')} from departure${pnr !== null ? `, point of no return ${disp(result, 'pnr_distance')} out` : ''}, going on at ${disp(result, 'groundspeed_out')} and back at ${disp(result, 'groundspeed_back')}.`;
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
    `<line class="dg-muted dg-dash" x1="150" y1="${f1(ti)}" x2="196" y2="${f1(ti)}"/>`,
    text('dg-muted-text', 200, ti + 4, `Indicated ${args.indicated ?? ''}`),
    line('dg-accent', [150, tt], [196, tt]),
    text('dg-label', 200, tt + 4, `True ${disp(result, 'true_altitude')}`),
    // The gap between the two, measured and named.
    arrow(170, ti, 170, tt, 'dg-accent', `${disp(result, 'error')}`, 0.5, err < 0 ? -1 : 1),
    text('dg-muted-text', 12, 24, `ISA ${args.isa_deviation ?? ''}: true ${err < 0 ? 'below' : 'above'} indicated`),
  ].join('');
  const title = `Altimetry: an indicated ${args.indicated} in air ${args.isa_deviation} from standard is a true ${disp(result, 'true_altitude')}, ${disp(result, 'error')} against the altimeter.`;
  return { markup: svg(body, title), desc: title };
}

/** Holding pattern: the racetrack, the AIM entry sectors, and where this heading falls among them. */
function holdEntry(args, result) {
  const course = val(result, 'outbound_course');
  if (!Number.isFinite(course)) return null;
  const inbound = (course + 180) % 360;
  const heading = deg(args.heading);
  const left = result.result.turns_used === 'left' || args.turns === 'left';
  // The holding side, square to the inbound course: right of it for right turns.
  const side = (inbound + (left ? 270 : 90)) % 360;
  const [fx, fy] = [150, 136];
  const [L, r] = [72, 21];
  const at = (b, len, from = [fx, fy]) => [from[0] + vec(b, len)[0], from[1] + vec(b, len)[1]];
  // The inbound leg ends at the fix; the outbound leg is parallel, one turn
  // diameter to the holding side.
  const A = at(inbound + 180, L);
  const C = at(side, 2 * r);
  const D = at(inbound + 180, L, C);
  const sweep = left ? 0 : 1;
  const arc = (from, to) => `<path class="dg-muted" d="M${from.map(f1).join(' ')}A${r} ${r} 0 0 ${sweep} ${to.map(f1).join(' ')}"/>`;
  // The sectors of AIM 5-3-8, as the three headings where the entry changes.
  const bounds = [(inbound + 180) % 360, (inbound + (left ? 70 : 110)) % 360, (inbound + (left ? 250 : 290)) % 360];
  // Each sector as the headings it runs over, clockwise from lo to hi.
  const [b0, b1, b2] = bounds;
  const sectors = left
    ? [['Parallel', b1, b0], ['Teardrop', b0, b2], ['Direct', b2, b1]]
    : [['Teardrop', b1, b0], ['Parallel', b0, b2], ['Direct', b2, b1]];
  // Drawn where the aircraft comes FROM, as the AIM figure draws them: an
  // aircraft on heading h arrives from the bearing h + 180 off the fix, so a
  // sector of headings sits on the opposite side of the fix. (The 70° line
  // maps onto itself; the course boundary becomes the inbound course
  // extended beyond the fix.)
  const from = (h) => (h + 180) % 360;
  // Teardrop and parallel are labeled between their lines, near the fix. The
  // arrival's own label sits clockwise of its arrow, so when the aircraft
  // arrives through a sector, that sector's label moves into the part of it
  // counterclockwise of the arrow. Direct is 180° wide and holds both legs, so
  // its label goes on the non-holding side, off the inbound leg, above the
  // caption.
  const place = (name, lo, hi) => {
    const span = (hi - lo + 360) % 360;
    const d = heading === null ? -1 : (heading - lo + 360) % 360;
    let spot;
    if (name === 'Direct') spot = at((inbound + (left ? 110 : 250)) % 360, 95);
    else if (d > 0 && d < span) spot = at(from(lo + (d >= 24 ? d / 2 : (d + span) / 2 + 8)), 55);
    else spot = at(from(lo + span / 2), 55);
    return [spot[0], Math.min(spot[1] + 4, 208)];
  };
  const body = [
    ...bounds.map((b) => line('dg-grid dg-dash', [fx, fy], at(from(b), 108))),
    ...sectors.map(([name, lo, hi]) => text('dg-muted-text', ...place(name, lo, hi), name, 'middle')),
    arrow(...A, fx, fy, 'dg-muted', `Inbound ${Math.round(inbound)}\u00b0`, 0.3),
    line('dg-muted', C, D),
    arc([fx, fy], C),
    arc(D, A),
    heading === null ? '' : arrow(...at(heading + 180, 96), fx, fy, 'dg-accent', `Arriving ${Math.round(heading)}\u00b0`, 0.15),
    dot(fx, fy, 'dg-dot-now'),
    text('dg-label', 160, 226, `${result.result.entry[0].toUpperCase()}${result.result.entry.slice(1)} entry, ${left ? 'left' : 'right'} turns`, 'middle'),
  ].join('');
  const title = `Holding on the ${Math.round(inbound)}\u00b0 inbound course with ${left ? 'left' : 'right'} turns: arriving on ${Math.round(heading ?? inbound)}\u00b0 puts the aircraft in the ${result.result.entry} sector. ${result.display?.explanation ?? ''}`;
  return { markup: svg(body, title), desc: title };
}

/** A wind triangle from any two of its sides: air vector + wind = ground vector. */
function triangleFrom({ heading, tas, course, gs }, answer, title) {
  if ([heading, tas, course, gs].some((x) => x === null || x === undefined || !Number.isFinite(x))) return null;
  const up = (b, len) => [len * Math.sin(b * R), len * Math.cos(b * R)];
  const S = fit([[0, 0], up(heading, tas), up(course, gs)]);
  const [o, a, g] = [S([0, 0]), S(up(heading, tas)), S(up(course, gs))];
  const cls = (k) => (k === answer.key ? 'dg-accent' : k === 'wind' ? 'dg-muted dg-dash' : 'dg-muted');
  const body = [
    `<text class="dg-muted-text" x="12" y="22">N ↑</text>`,
    arrow(...o, ...a, cls('air'), answer.air, 0.5, -1),
    arrow(...a, ...g, cls('wind'), answer.wind, 0.5, -1),
    arrow(...o, ...g, cls('ground'), answer.ground, 0.5, 1),
    answer.caption ? text('dg-label', 160, 226, answer.caption, 'middle') : '',
  ].join('');
  return { markup: svg(body, title), desc: title };
}
const ktOf = (result, k) => (result.result[k] ? measure(`${val(result, k)} ${result.result[k].unit}`, SPEED, 'kt') : null);

function findWind(args, result) {
  return triangleFrom(
    { heading: deg(args.heading), tas: kt(args.tas), course: deg(args.track), gs: kt(args.groundspeed) },
    { key: 'wind', air: 'Heading', wind: 'Wind', ground: 'Track', caption: `Wind from ${disp(result, 'wind_direction')} at ${disp(result, 'wind_speed')}` },
    `Wind triangle: the difference between where the nose points and where the aircraft goes is a wind from ${disp(result, 'wind_direction')} at ${disp(result, 'wind_speed')}.`,
  );
}
function courseFromHeading(args, result) {
  return triangleFrom(
    { heading: deg(args.heading), tas: kt(args.tas), course: val(result, 'course'), gs: ktOf(result, 'groundspeed') },
    { key: 'ground', air: 'Heading', wind: 'Wind', ground: 'Course', caption: `Course ${disp(result, 'course')} at ${disp(result, 'groundspeed')}` },
    `Wind triangle: heading ${typed(args.heading, '°')} with this wind makes good a course of ${disp(result, 'course')} at ${disp(result, 'groundspeed')}.`,
  );
}
function tasFromGroundspeed(args, result) {
  return triangleFrom(
    { heading: val(result, 'heading'), tas: ktOf(result, 'tas'), course: deg(args.course), gs: kt(args.groundspeed) },
    { key: 'air', air: 'Heading', wind: 'Wind', ground: 'Course', caption: `Heading ${disp(result, 'heading')} at ${disp(result, 'tas')} true airspeed` },
    `Wind triangle: to make good the course at that groundspeed, fly heading ${disp(result, 'heading')} at ${disp(result, 'tas')} true airspeed.`,
  );
}

/** Holding wind correction: the racetrack with the headings and outbound time that fly it. */
function holdTiming(args, result) {
  const course = deg(args.inbound_course);
  const wd = deg(args.wind_direction);
  if (course === null || !Number.isFinite(val(result, 'outbound_heading'))) return null;
  const left = args.turns === 'left';
  const side = (course + (left ? 270 : 90)) % 360;
  const [L, r] = [80, 22];
  // The fix placed so the whole racetrack is centered, whatever the course.
  const mid = [vec(course + 180, L / 2)[0] + vec(side, r)[0], vec(course + 180, L / 2)[1] + vec(side, r)[1]];
  const [fx, fy] = [160 - mid[0], 112 - mid[1]];
  const at = (b, len, from = [fx, fy]) => [from[0] + vec(b, len)[0], from[1] + vec(b, len)[1]];
  const A = at(course + 180, L);
  const C = at(side, 2 * r);
  const D = at(course + 180, L, C);
  const sweep = left ? 0 : 1;
  const arc = (from, to) => `<path class="dg-muted" d="M${from.map(f1).join(' ')}A${r} ${r} 0 0 ${sweep} ${to.map(f1).join(' ')}"/>`;
  const body = [
    `<text class="dg-muted-text" x="12" y="22">N ↑</text>`,
    arc([fx, fy], C),
    arrow(...C, ...D, 'dg-accent', 'Out', 0.5, left ? -1 : 1),
    arc(D, A),
    arrow(...A, fx, fy, 'dg-accent', 'In', 0.5, left ? 1 : -1),
    // The wind blows from its direction: the arrow starts upwind of its tip.
    wd === null ? '' : arrow(...at(wd, 40, [56, 186]), 56, 186, 'dg-muted dg-dash', 'Wind', 0.3),
    dot(fx, fy, 'dg-dot-now'),
    text('dg-label', 160, 212, `In ${disp(result, 'inbound_heading')} · out ${disp(result, 'outbound_heading')} for ${disp(result, 'outbound_time')}`, 'middle'),
    text('dg-muted-text', 160, 228, `Groundspeed ${disp(result, 'inbound_groundspeed')} in, ${disp(result, 'outbound_groundspeed')} out`, 'middle'),
  ].join('');
  const title = `Hold on the ${Math.round(course)}° inbound course: fly ${disp(result, 'inbound_heading')} inbound and ${disp(result, 'outbound_heading')} outbound for ${disp(result, 'outbound_time')}.`;
  return { markup: svg(body, title), desc: title };
}

export const DIAGRAMS = {
  'aviation.wind.heading-groundspeed': windTriangle,
  'aviation.wind.runway-components': runwayComponents,
  'aviation.performance.top-of-descent': descentProfile,
  'aviation.performance.vdp': approachProfile,
  'aviation.airspeed.cas-to-tas': airspeedGauge,
  'aviation.airspeed.tas-to-cas': airspeedGauge,
  ...Object.fromEntries(['ias-to-tas', 'ias-to-mach', 'ias-to-eas', 'eas-to-tas', 'eas-to-mach', 'mach-to-eas', 'tas-to-eas'].map((p) => [`aviation.airspeed.${p}`, airspeedGauge])),
  'aviation.atmosphere.isa': isaProfile,
  'aviation.loading.weight-balance': cgEnvelope,
  'aviation.altimetry.true-altitude': altimetry,
  'aviation.ifr.hold-entry': holdEntry,
  'aviation.performance.climb-gradient': climbTriangle,
  'aviation.performance.climb-plan': climbProfile,
  'aviation.performance.etp-pnr': etpLine,
  'aviation.wind.find-wind': findWind,
  'aviation.wind.course-from-heading': courseFromHeading,
  'aviation.wind.tas-from-groundspeed': tasFromGroundspeed,
  'aviation.ifr.hold-wind-timing': holdTiming,
};
