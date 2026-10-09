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

export const DIAGRAMS = {
  'time.sun.mapping-window': sunPath,
};
