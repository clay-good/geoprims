// The time tools' diagrams. See diagrams.js for what a diagram is.

import { R, deg, disp, dot, f1, line, path, svg, text } from './kit.js';

/** Sun path: the day's track across the sky, with the part above the threshold marked. */
function sunPath(args, result) {
  const path = (result.result.path ?? []).map((p) => [p.azimuth.value, p.elevation.value, p.time]);
  const threshold = deg(args.threshold) ?? 30;
  if (path.length < 10) return null;
  // A sky plot: north up, azimuth around, the horizon at the rim and the
  // zenith at the center, so the path is read the way the sky is.
  const [cx, cy, sky] = [150, 114, 94];
  const at = (az, el) => {
    const rr = sky * (1 - Math.max(el, 0) / 90);
    return [cx + rr * Math.sin(az * R), cy - rr * Math.cos(az * R)];
  };
  const up = path.filter((p) => p[1] >= 0);
  const seg = (a, b) => line(Math.min(a[1], b[1]) >= threshold ? 'dg-accent' : 'dg-muted', at(a[0], a[1]), at(b[0], b[1]));
  const top = path.reduce((a, b) => (b[1] > a[1] ? b : a));
  const ring = (el, cls) => `<circle class="${cls}" cx="${cx}" cy="${cy}" r="${f1(sky * (1 - el / 90))}" fill="none"/>`;
  const body = [
    ring(0, 'dg-grid'),
    ring(threshold, 'dg-grid dg-dash'),
    ...['N', 'E', 'S', 'W'].map((c, i) => text('dg-muted-text', ...at(i * 90, -6).map((q, j) => q + (j ? 4 : 0)), c, 'middle')),
    ...up.slice(1).map((p, i) => seg(up[i], p)),
    dot(...at(top[0], top[1]), 'dg-dot-now'),
    text('dg-label', ...at(top[0], top[1]).map((q, j) => q + (j ? -10 : 0)), `${top[2]} · ${Math.round(top[1])}\u00b0`, 'middle'),
    text('dg-muted-text', 12, 234, `Above ${Math.round(threshold)}\u00b0 ${result.result.window} · ${disp(result, 'duration')}`),
  ].join('');
  const title = `Sun path for the day: highest ${disp(result, 'max_elevation')} at ${top[2]}, above ${Math.round(threshold)}\u00b0 ${result.result.window} (${disp(result, 'duration')}). North is up, the rim is the horizon and the center is overhead.`;
  return { markup: svg(body, title), desc: title };
}

/**
 * The moon's night: both paths on one sky plot, the moon in the accent style
 * and the sun behind it, with rise and set marked where each crosses the
 * horizon. North up, azimuth around, the horizon at the rim and the zenith at
 * the centre, so it reads the way the sky does.
 */
function moonSky(args, result) {
  const rows = result.result.path ?? [];
  if (rows.length < 20) return null;
  const of = (name) => rows
    .filter((p) => p.body === name)
    .map((p) => [p.azimuth.value, p.elevation.value, p.time]);
  const [moon, sun] = [of('moon'), of('sun')];
  if (moon.length < 10) return null;
  const [cx, cy, sky] = [150, 114, 94];
  const at = (az, el) => {
    const rr = sky * (1 - Math.max(el, 0) / 90);
    return [cx + rr * Math.sin(az * R), cy - rr * Math.cos(az * R)];
  };
  // Only the part above the horizon is on the plot; a body below it is nowhere
  // in the sky, and squashing it onto the rim would be a lie.
  const arc = (track, cls) => track
    .slice(1)
    .map((p, i) => (Math.min(track[i][1], p[1]) >= 0 ? line(cls, at(track[i][0], track[i][1]), at(p[0], p[1])) : ''))
    .join('');
  // Where a track crosses the horizon, which is the rise or the set.
  const crossings = (track) => track.slice(1).flatMap((p, i) => {
    const q = track[i];
    if (q[1] >= 0 === p[1] >= 0) return [];
    const f = q[1] / (q[1] - p[1]);
    return [[q[0] + (p[0] - q[0]) * f, p[1] > q[1] ? 'rise' : 'set', p[2]]];
  });
  const ring = (el, cls) => `<circle class="${cls}" cx="${cx}" cy="${cy}" r="${f1(sky * (1 - el / 90))}" fill="none"/>`;
  const top = moon.reduce((a, b) => (b[1] > a[1] ? b : a));
  const lit = Math.round((result.result.illuminated ?? 0) * 100);
  const body = [
    ring(0, 'dg-grid'),
    ring(30, 'dg-grid dg-dash'),
    ...['N', 'E', 'S', 'W'].map((c, i) => text('dg-muted-text', ...at(i * 90, -6).map((q, j) => q + (j ? 4 : 0)), c, 'middle')),
    arc(sun, 'dg-muted dg-dash'),
    arc(moon, 'dg-accent'),
    ...crossings(moon).map(([az, kind, t]) => {
      const [x, y] = at(az, 0);
      return dot(x, y) + text('dg-label', x, y + (y < cy ? -8 : 16), `${kind} ${t}`, 'middle');
    }),
    top[1] >= 0 ? dot(...at(top[0], top[1]), 'dg-dot-now') : '',
    top[1] >= 0 ? text('dg-label', ...at(top[0], top[1]).map((q, j) => q + (j ? -10 : 0)), `${Math.round(top[1])}\u00b0`, 'middle') : '',
    // One line, clear of the compass letter at the bottom of the rim.
    text('dg-muted-text', 12, 236, `Moon solid, sun dashed \u00b7 ${lit}% lit${result.result.state === 'normal' ? '' : ` \u00b7 ${result.display?.state ?? ''}`}`),
  ].join('');
  const title = `The moon's path across the sky for the local date, with the sun's behind it: ${
    top[1] >= 0 ? `highest ${Math.round(top[1])} degrees at ${top[2]} UTC` : 'never above the horizon'
  }, ${lit}% of the disc lit. North is up, the rim is the horizon and the centre is overhead.`;
  return { markup: svg(body, title), desc: title };
}

export const DIAGRAMS = {
  'time.sun.mapping-window': sunPath,
  'time.moon.rise-set': moonSky,
};
