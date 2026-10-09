// The drone tools' diagrams. See diagrams.js for what a diagram is.

import { LENGTH, R, SPEED, arrow, deg, disp, dot, f1, fit, kt, line, measure, path, svg, text, typed, unitOf, val } from './kit.js';

/** A result quantity in base units (m, m/s, Wh) through a unit table, or null. */
const qty = (result, k, table) => {
  const v = val(result, k);
  return Number.isFinite(v) ? measure(`${v} ${unitOf(result, k) || ''}`.trim(), table, Object.keys(table)[0]) : null;
};
/** Camera footprint, side view: the height, the view cone, and the ground one photo spans. */
function gsdFootprint(args, result) {
  const h = result.result.height ? qty(result, 'height', LENGTH) : measure(args.height, LENGTH, 'm');
  const w = qty(result, 'footprint_across', LENGTH);
  if (!(h > 0) || !(w > 0)) return null;
  const S = fit([[-w / 2, 0], [w / 2, 0], [0, h]], 240, 140);
  const [cam, l, r] = [S([0, h]), S([-w / 2, 0]), S([w / 2, 0])];
  const height = result.result.height ? disp(result, 'height') : typed(args.height, 'm');
  const gsd = result.result.gsd ? disp(result, 'gsd') : typed(args.target_gsd, 'cm');
  const body = [
    line('dg-grid', [12, l[1]], [308, l[1]]),
    `<path class="dg-fill" d="${path([cam, l, r], true)}"/>`,
    line('dg-muted dg-dash', cam, [cam[0], l[1]]),
    text('dg-muted-text', cam[0] + 6, (cam[1] + l[1]) / 2, height),
    line('dg-casing', l, r), line('dg-accent', l, r),
    dot(...cam, 'dg-dot-now'),
    text('dg-label', 12, 22, `1 pixel = ${gsd} of ground`),
    text('dg-label', 160, l[1] + 18, `One photo spans ${disp(result, 'footprint_across')} across track`, 'middle'),
    result.result.footprint_along ? text('dg-muted-text', 160, l[1] + 32, `and ${disp(result, 'footprint_along')} along it`, 'middle') : '',
  ].join('');
  const title = `Camera ${height} above flat ground: each pixel covers ${gsd}, and one photo spans ${disp(result, 'footprint_across')} across track.`;
  return { markup: svg(body, title), desc: title };
}

/** Photo overlap, plan view: frames along two flight lines, overlaps reading darker. */
function triggerOverlap(args, result) {
  const [fa, fl, d, s] = ['footprint_across', 'footprint_along', 'trigger_distance', 'line_spacing'].map((k) => qty(result, k, LENGTH));
  if (![fa, fl, d, s].every((x) => x > 0)) return null;
  const frames = [[0, 0], [0, d], [0, 2 * d], [s, 0], [s, d]];
  const corners = ([x, y]) => [[x - fa / 2, y - fl / 2], [x + fa / 2, y - fl / 2], [x + fa / 2, y + fl / 2], [x - fa / 2, y + fl / 2]];
  const S = fit(frames.flatMap(corners), 220, 150);
  const pct = (v) => (v === undefined || v === null || v === '' ? '' : `${String(v).replace('%', '').trim()}%`);
  const front = pct(args.front_overlap);
  const side = pct(args.side_overlap);
  const body = [
    ...frames.map((f, i) => `<path class="${i === 0 ? 'dg-fill dg-accent' : 'dg-fill'}" d="${path(corners(f).map(S), true)}"/>`),
    arrow(...S([0, -fl / 2]), ...S([0, 2 * d + fl / 2]), 'dg-muted dg-dash', ''),
    arrow(...S([s, d + fl / 2]), ...S([s, -fl / 2]), 'dg-muted dg-dash', ''),
    ...frames.slice(0, 3).map((f) => dot(...S(f))),
    text('dg-label', 12, 22, 'Flight lines, seen from above'),
    text('dg-muted-text', 12, 214, `Photo every ${disp(result, 'trigger_distance')} (${disp(result, 'trigger_interval')})${front ? `, ${front} front overlap` : ''}`),
    text('dg-muted-text', 12, 230, `Lines ${disp(result, 'line_spacing')} apart${side ? `, ${side} side overlap` : ''}`),
  ].join('');
  const title = `Photos every ${disp(result, 'trigger_distance')} along each line and lines ${disp(result, 'line_spacing')} apart; each photo covers ${disp(result, 'footprint_across')} by ${disp(result, 'footprint_along')}, so neighbors overlap.`;
  return { markup: svg(body, title), desc: title };
}

/** Oblique view, side on: the rays to the near edge, center, and far edge, and the ground between. */
function obliqueFootprint(args, result) {
  const h = measure(args.height, LENGTH, 'm');
  const near = qty(result, 'near_distance', LENGTH);
  const far0 = qty(result, 'far_distance', LENGTH);
  const tilt = deg(args.pitch);
  if (!(h > 0) || !Number.isFinite(near) || tilt === null) return null;
  const center = h * Math.tan(Math.min(Math.abs(tilt), 89) * R);
  // With the horizon in the frame the far edge never reaches the ground: draw the ray running on.
  const sky = !(far0 > 0);
  const far = sky ? Math.max(center * 2.5, near * 4, h) : far0;
  const S = fit([[-far * 0.18, 0], [far, 0], [0, h]], 270, 130);
  const [cam, g0, n, c, f] = [S([0, h]), S([0, 0]), S([near, 0]), S([center, 0]), S([far, 0])];
  const body = [
    line('dg-grid', [12, g0[1]], [308, g0[1]]),
    `<path class="dg-fill" d="${path([cam, n, f], true)}"/>`,
    line('dg-muted dg-dash', cam, g0),
    line('dg-muted', cam, n), line(sky ? 'dg-muted dg-dash' : 'dg-muted', cam, f),
    line('dg-casing', cam, c), line('dg-accent', cam, c),
    line('dg-casing', n, f), line('dg-accent', n, f),
    dot(...cam, 'dg-dot-now'),
    text('dg-muted-text', cam[0] - 6, (cam[1] + g0[1]) / 2, typed(args.height, 'm'), 'end'),
    text('dg-label', 12, 22, `Tilted ${typed(args.pitch, '°').replace(' °', '°')} from straight down`),
    text('dg-muted-text', n[0], g0[1] + 16, `Near ${disp(result, 'gsd_near')}`, 'middle'),
    text('dg-label', c[0] + 6, c[1] - 30, `Center ${disp(result, 'gsd_center')}`),
    text('dg-muted-text', Math.min(f[0], 300), g0[1] + 16, sky ? 'Far edge: sky' : `Far ${disp(result, 'gsd_far')}`, 'end'),
    text('dg-muted-text', 12, 230, `Straight down: ${disp(result, 'gsd_nadir')} per pixel`),
  ].join('');
  const title = `Camera tilted ${typed(args.pitch, '°')}: a pixel covers ${disp(result, 'gsd_near')} at the near edge, ${disp(result, 'gsd_center')} at the center, and ${sky ? 'the top of the frame sees sky' : `${disp(result, 'gsd_far')} at the far edge`}.`;
  return { markup: svg(body, title), desc: title };
}

/** Part 107 ceiling, side view: the structure, where the drone is, and how high it may go there. */
function part107Ceiling(args, result) {
  const top = qty(result, 'max_agl', LENGTH);
  const sh = args.structure_height === undefined || args.structure_height === '' ? null : measure(args.structure_height, LENGTH, 'ft');
  const sd = args.structure_distance === undefined || args.structure_distance === '' ? null : measure(args.structure_distance, LENGTH, 'ft');
  if (!(top > 0)) return null;
  const base = 400 * 0.3048;
  const hasStructure = sh > 0 && Number.isFinite(sd);
  const x = hasStructure ? Math.max(sd, 30) : 0;
  const S = fit([[-150, 0], [Math.max(x, 60) + 120, 0], [0, Math.max(top, base, sh ?? 0)]], 250, 150);
  const g = S([0, 0])[1];
  const drone = S([x, top]);
  const body = [
    line('dg-grid', [12, g], [308, g]),
    line('dg-grid dg-dash', [12, S([0, base])[1]], [308, S([0, base])[1]]),
    text('dg-muted-text', 12, S([0, base])[1] - 4, '400 ft above ground'),
    hasStructure ? `<path class="dg-fill" d="${path([S([-8, 0]), S([8, 0]), S([8, sh]), S([-8, sh])], true)}"/>` : '',
    hasStructure ? text('dg-muted-text', S([0, sh])[0] - 12, S([0, sh])[1] + 4, `Structure ${typed(args.structure_height, 'ft')}`, 'end') : '',
    line('dg-casing', [drone[0], g], drone), line('dg-accent', [drone[0], g], drone),
    dot(...drone),
    text('dg-label', drone[0] + 8, drone[1] + 4, `Up to ${disp(result, 'max_agl')}`),
    hasStructure ? text('dg-muted-text', (S([0, 0])[0] + drone[0]) / 2, g + 16, `${typed(args.structure_distance, 'ft')} away`, 'middle') : '',
    text('dg-muted-text', 12, 237, 'Summary of 14 CFR 107.51(b). Not legal advice.'),
  ].join('');
  const title = `Here you may fly up to ${disp(result, 'max_agl')} above the ground${hasStructure ? `, ${typed(args.structure_distance, 'ft')} from a ${typed(args.structure_height, 'ft')} structure` : ''}. Not legal advice.`;
  return { markup: svg(body, title), desc: title };
}

/** Return-to-home energy: the battery left, split into the trip home, the reserve, and the margin. */
function rthBudget(args, result) {
  const ENERGY = { wh: 1, kwh: 1000, j: 1 / 3600, kj: 1 / 3.6 };
  const e = (v) => measure(v, ENERGY, 'wh');
  const home = e(`${val(result, 'return_energy')} ${unitOf(result, 'return_energy')}`);
  const margin = e(`${val(result, 'margin')} ${unitOf(result, 'margin')}`);
  const left = e(args.remaining_energy);
  if (![home, margin, left].every(Number.isFinite) || left <= 0) return null;
  const reserve = Math.max(left - home - margin, 0);
  const need = home + reserve;
  const full = Math.max(left, need);
  const [x0, w, y, hgt] = [20, 280, 96, 34];
  const X = (v) => x0 + (w * v) / full;
  const seg = (a, b, cls) => (b > a ? `<path class="${cls}" d="${path([[X(a), y], [X(b), y], [X(b), y + hgt], [X(a), y + hgt]], true)}"/>` : '');
  const short = margin < 0;
  const body = [
    seg(0, home, 'dg-fill dg-accent'),
    seg(home, need, 'dg-fill'),
    `<path class="dg-muted" d="${path([[X(0), y], [X(left), y], [X(left), y + hgt], [X(0), y + hgt]], true)}"/>`,
    text('dg-label', X(0), y - 10, `Trip home ${disp(result, 'return_energy')}`),
    reserve > 0 ? text('dg-muted-text', X(home) + 4, y + hgt + 16, `Reserve ${typed(args.reserve_energy, 'Wh')}`) : '',
    text(short ? 'dg-label' : 'dg-muted-text', X(full), y - 10, short ? `Short by ${disp(result, 'margin').replace('-', '')}` : `Margin ${disp(result, 'margin')}`, 'end'),
    text('dg-muted-text', 20, 190, `Battery left: ${typed(args.remaining_energy, 'Wh')} (outlined)`),
    text('dg-muted-text', 20, 206, `Home at ${disp(result, 'return_groundspeed')} over the ground, ${disp(result, 'return_time')}`),
  ].join('');
  const title = `Of ${typed(args.remaining_energy, 'Wh')} left, the trip home takes ${disp(result, 'return_energy')}${reserve > 0 ? ` and the reserve ${typed(args.reserve_energy, 'Wh')}` : ''}, ${short ? `leaving you short by ${disp(result, 'margin').replace('-', '')}` : `leaving ${disp(result, 'margin')} of margin`}.`;
  return { markup: svg(body, title), desc: title };
}

/** Wind at flying height: the sustained wind and the gust against the drone's limit, on one scale. */
function windLimit(args, result) {
  const w = qty(result, 'wind_at_height', SPEED);
  const g = result.result.gust_at_height ? qty(result, 'gust_at_height', SPEED) : null;
  const lim = qty(result, 'rating', SPEED);
  if (![w, lim].every(Number.isFinite) || lim <= 0 || (g !== null && !Number.isFinite(g))) return null;
  const top = Math.max(w, g ?? 0, lim) * 1.15;
  const [x0, wd, y, h] = [20, 280, 84, 30];
  const X = (v) => x0 + (wd * v) / top;
  const bar = (a, b, cls) => (b > a ? `<path class="${cls}" d="${path([[X(a), y], [X(b), y], [X(b), y + h], [X(a), y + h]], true)}"/>` : '');
  const over = (g ?? w) > lim;
  const reported = `Reported ${typed(args.wind_speed, 'kt')} at ${args.report_height ? typed(args.report_height, 'm') : '10 m'}; α = ${disp(result, 'exponent')}`;
  const body = [
    line('dg-grid', [x0, y + h], [x0 + wd, y + h]),
    bar(0, w, 'dg-fill dg-accent'),
    g !== null ? bar(w, g, 'dg-fill') : '',
    text('dg-label', X(0), y - 10, `Wind ${disp(result, 'wind_at_height')}`),
    g !== null && g > w ? text('dg-muted-text', X(g), y + h + 16, `Gust ${disp(result, 'gust_at_height')}`, 'end') : '',
    line('dg-muted dg-dash', [X(lim), y - 30], [X(lim), y + h + 30]),
    text(over ? 'dg-label' : 'dg-muted-text', X(lim), y - 34, `Limit ${disp(result, 'rating')}`, 'middle'),
    text('dg-muted-text', 20, 176, `At ${typed(args.flying_height, 'm')}: margin ${disp(result, 'margin_to_rating')}`),
    text('dg-muted-text', 20, 192, reported),
    text('dg-muted-text', 20, 226, 'Gusts near buildings are not modeled.'),
  ].join('');
  const title = `At ${typed(args.flying_height, 'm')} the wind is ${disp(result, 'wind_at_height')}${g !== null ? `, gusting ${disp(result, 'gust_at_height')}` : ''}, against a limit of ${disp(result, 'rating')}.`;
  return { markup: svg(body, title), desc: title };
}

/** Part 107 groundspeed against the rule's fixed 87 kt limit. */
function speedLimitGauge(args, result) {
  const speed = qty(result, 'groundspeed', SPEED);
  const limit = qty(result, 'limit', SPEED);
  if (![speed, limit].every(Number.isFinite) || limit <= 0) return null;
  const [x0, x1, y] = [30, 290, 112];
  const top = Math.max(limit * 1.2, speed * 1.08);
  const X = (v) => x0 + ((x1 - x0) * v) / top;
  const body = [
    line('dg-grid', [x0, y], [x1, y]),
    line('dg-accent', [x0, y], [X(speed), y]),
    line('dg-arc-red', [X(limit), y - 34], [X(limit), y + 34]),
    `<circle class="dg-dot-now" cx="${f1(X(speed))}" cy="${y}" r="6"/>`,
    text('dg-label', X(speed), y - 18, disp(result, 'groundspeed'), 'middle'),
    text('dg-muted-text', X(limit), y + 54, `Part 107 limit ${disp(result, 'limit')}`, 'middle'),
    text('dg-muted-text', 160, 198, `Margin ${disp(result, 'margin')}`, 'middle'),
  ].join('');
  const title = `Groundspeed ${disp(result, 'groundspeed')} against the Part 107 limit of ${disp(result, 'limit')}; margin ${disp(result, 'margin')}.`;
  return { markup: svg(body, title), desc: title };
}

/** Battery capacity with the usable share ending before the landing reserve. */
function batteryReserveGauge(args, result) {
  const energy = val(result, 'energy');
  const usable = val(result, 'usable_energy');
  if (![energy, usable].every(Number.isFinite) || energy <= 0 || usable < 0) return null;
  const [x0, x1, y] = [30, 290, 112];
  const X = (v) => x0 + ((x1 - x0) * v) / energy;
  const reserve = Math.max(0, Number(args.reserve ?? 0));
  const body = [
    `<rect class="dg-grid" x="${x0}" y="${y - 18}" width="${x1 - x0}" height="36"/>`,
    `<rect class="dg-fill" x="${x0}" y="${y - 18}" width="${f1(Math.max(0, X(usable) - x0))}" height="36"/>`,
    line('dg-muted dg-dash', [X(usable), y - 30], [X(usable), y + 30]),
    text('dg-label', X(usable), y - 40, `Usable ${disp(result, 'usable_energy')}`, usable / energy > 0.78 ? 'end' : 'middle'),
    text('dg-muted-text', x0, y + 52, '0'),
    text('dg-muted-text', x1, y + 52, `Pack ${disp(result, 'energy')}`, 'end'),
    text('dg-muted-text', 160, 198, `${reserve}% landing reserve`, 'middle'),
  ].join('');
  const title = `Battery capacity ${disp(result, 'energy')}, with ${disp(result, 'usable_energy')} usable after the discharge limit and ${reserve}% landing reserve.`;
  return { markup: svg(body, title), desc: title };
}

export const DIAGRAMS = {
  'drone.photogrammetry.gsd': gsdFootprint,
  'drone.photogrammetry.altitude-for-gsd': gsdFootprint,
  'drone.photogrammetry.trigger': triggerOverlap,
  'drone.photogrammetry.oblique-gsd': obliqueFootprint,
  'drone.ops.part107-altitude': part107Ceiling,
  'drone.power.rth-budget': rthBudget,
  'drone.ops.wind-limit': windLimit,
  'drone.ops.speed-check': speedLimitGauge,
  'drone.power.battery-energy': batteryReserveGauge,
};
