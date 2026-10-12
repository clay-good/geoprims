// Exporting the canvas (web/map-canvas, "Export"): a PNG with the result's
// caption and every attribution the view owes in a footer strip, an SVG of a
// vector diagram or 2D map, and GeoJSON of every geographic layer drawn.
//
// The GeoJSON and the attribution are pure, so they are checked without a
// browser; composing the PNG needs a canvas and lives in `pngWithFooter`.

/** The base map's attribution: Natural Earth is public domain, and says so. */
export const BASEMAP_ATTRIBUTION = 'Base map: Natural Earth (public domain)';

/**
 * Every attribution a view owes: the base map, then each data asset the
 * result was computed from (a DEM, a geoid, a magnetic model), from the asset
 * registry, in the order the result lists them.
 */
export function attributionLines(result, registry, { basemap = true } = {}) {
  const lines = basemap ? [BASEMAP_ATTRIBUTION] : [];
  for (const a of result?.meta?.assets ?? []) {
    const entry = (registry?.assets ?? []).find((r) => r.id === a.id);
    if (entry?.attribution && !lines.includes(entry.attribution)) lines.push(entry.attribution);
  }
  return lines;
}

/** The caption under an exported view: the sentence, then which tool made it. */
export const caption = (tool, result) =>
  [result?.summary, `geoprims ${tool.id} ${tool.version}`].filter(Boolean).join(' — ');

/**
 * Every geographic layer as a GeoJSON feature, [lon, lat] per RFC 7946:
 * lines as LineStrings, points as Points (with their label), polygons with
 * each ring closed. The comparison line keeps its role, so a reader can tell
 * the geodesic from the dashed rhumb line.
 */
export function layersGeoJson(layers, tool) {
  const features = [];
  for (const l of layers ?? []) {
    const properties = { role: l.role, tool: tool.id, toolVersion: tool.version, ...(l.label ? { label: l.label } : {}), ...(l.path ? { path: l.path } : {}) };
    if (l.kind === 'line' && l.points?.length > 1) {
      features.push({ type: 'Feature', geometry: { type: 'LineString', coordinates: l.points }, properties });
    } else if (l.kind === 'point') {
      for (const p of l.points ?? []) features.push({ type: 'Feature', geometry: { type: 'Point', coordinates: p }, properties });
    } else if (l.kind === 'polygon' && l.rings?.length) {
      const rings = l.rings.map((r) => (r.length && (r[0][0] !== r.at(-1)[0] || r[0][1] !== r.at(-1)[1]) ? [...r, r[0]] : r));
      features.push({ type: 'Feature', geometry: { type: 'Polygon', coordinates: rings }, properties });
    }
  }
  return { type: 'FeatureCollection', features };
}

/** Breaks `text` at spaces into lines that `fits` accepts (a long word stays whole). */
export function wrap(text, fits) {
  const lines = [];
  let line = '';
  for (const word of String(text).split(/\s+/).filter(Boolean)) {
    const next = line ? `${line} ${word}` : word;
    if (line && !fits(next)) {
      lines.push(line);
      line = word;
    } else line = next;
  }
  if (line) lines.push(line);
  return lines;
}

/**
 * An SVG diagram as a file that stands alone: the page's diagram rules written
 * into a <style> (the page's stylesheet does not travel with it), a
 * background, and the caption and attribution in a footer strip.
 */
export function svgWithFooter(markup, captionText, lines, { style = '', background = '#ffffff', ink = '#4f5763' } = {}) {
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
  // About 0.55 em per character in a sans face at 10 units.
  const note = [captionText, ...lines].filter(Boolean).flatMap((t) => wrap(t, (l) => l.length * 5.5 <= Number(/viewBox="0 0 (\d+)/.exec(markup)?.[1] ?? 320) - 16));
  const open = markup.match(/^<svg\b[^>]*>/)?.[0];
  const viewBox = open && /viewBox="0 0 (\d+(?:\.\d+)?) (\d+(?:\.\d+)?)"/.exec(open);
  if (!viewBox) return markup;
  const [w, h] = [Number(viewBox[1]), Number(viewBox[2])];
  const extra = 14 * note.length + 10;
  const grown = open.replace(viewBox[0], `viewBox="0 0 ${w} ${h + extra}"`);
  const head = `${style ? `<style>${style}</style>` : ''}<rect x="0" y="0" width="${w}" height="${h + extra}" fill="${background}"/>`;
  const text = note.map((t, i) => `<text x="8" y="${h + 16 + i * 14}" font-family="sans-serif" font-size="10" fill="${ink}">${esc(t)}</text>`).join('');
  return markup.replace(open, grown + head).replace(/<\/svg>\s*$/, `${text}</svg>`);
}

const RAD = Math.PI / 180;
const MAX_MERCATOR_LAT = 85.0511287798;
const mercY = (lat) => Math.log(Math.tan(Math.PI / 4 + (Math.max(-MAX_MERCATOR_LAT, Math.min(MAX_MERCATOR_LAT, lat)) * RAD) / 2));
const xml = (value) => String(value).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[c]);
const num = (value) => Number(value.toFixed(2));

function unwrapped(points) {
  const out = [];
  for (const [lon, lat] of points) {
    if (!out.length) out.push([lon, lat]);
    else {
      const previous = out.at(-1)[0];
      out.push([previous + ((((lon - previous + 180) % 360) + 360) % 360) - 180, lat]);
    }
  }
  return out;
}

/** An SVG path for geographic points in a 2D view, with antimeridian copies. */
function mapPath(view, points, closed = false) {
  if (!points?.length) return '';
  const paths = [];
  if (view.mode === 'map' || view.mode === 'equirect') {
    const continuous = unwrapped(points);
    const y0 = view.mode === 'map' ? mercY(view.lat) : view.lat * RAD;
    for (const off of [-360, 0, 360]) {
      const projected = continuous.map(([lon, lat]) => [
        view.width / 2 + view.scale * (lon + off - view.lon) * RAD,
        view.height / 2 - view.scale * ((view.mode === 'map' ? mercY(lat) : lat * RAD) - y0),
      ]);
      let minX = Infinity; let minY = Infinity; let maxX = -Infinity; let maxY = -Infinity;
      for (const [x, y] of projected) {
        minX = Math.min(minX, x); maxX = Math.max(maxX, x);
        minY = Math.min(minY, y); maxY = Math.max(maxY, y);
      }
      if (maxX < -2 || minX > view.width + 2 || maxY < -2 || minY > view.height + 2) continue;
      const kept = [];
      for (const point of projected) {
        if (!kept.length || Math.hypot(point[0] - kept.at(-1)[0], point[1] - kept.at(-1)[1]) >= 0.75) kept.push(point);
      }
      if (kept.at(-1) !== projected.at(-1)) kept.push(projected.at(-1));
      if (kept.length) paths.push(`M${kept.map(([x, y]) => `${num(x)} ${num(y)}`).join('L')}${closed ? 'Z' : ''}`);
    }
    return paths.join('');
  }
  if (view.mode !== 'polar') return '';
  const h = view.lat >= 0 ? 1 : -1;
  const projected = points.map(([lon, lat]) => {
    const colat = 90 - h * lat;
    if (colat > 150) return null;
    const rho = colat * RAD * view.scale;
    const dl = (lon - view.lon) * RAD;
    return [view.width / 2 + rho * Math.sin(dl), view.height / 2 + h * rho * Math.cos(dl)];
  });
  if (closed && projected.some((point) => !point)) return '';
  let d = '';
  let open = false;
  for (const point of projected) {
    if (!point) {
      open = false;
      continue;
    }
    d += `${open ? 'L' : 'M'}${num(point[0])} ${num(point[1])}`;
    open = true;
  }
  return d + (closed && d ? 'Z' : '');
}

function gridLines(step, reach) {
  const lines = [];
  for (let lon = -180; lon < 180; lon += step) lines.push(Array.from({ length: reach + 1 }, (_, i) => [lon, -reach + i * 2]));
  const top = reach > 80 ? Math.floor(80 / step) * step : 60;
  for (let lat = -top; lat <= top; lat += step) lines.push(Array.from({ length: 181 }, (_, i) => [-180 + i * 2, lat]));
  return lines;
}

/** A standalone vector export of the current 2D map; globe export stays PNG. */
export function mapSvg(view, base, layers, colors, captionText, attribution) {
  if (!view || view.mode === 'globe') throw new Error('SVG export is available for 2D map views');
  const width = Math.max(1, Math.round(view.width));
  const height = Math.max(1, Math.round(view.height));
  const path = (points, closed, attrs) => {
    const d = mapPath({ ...view, width, height }, points, closed);
    return d ? `<path d="${d}" ${attrs}/>` : '';
  };
  const many = (sets, closed, attrs) => (sets ?? []).map((points) => path(points, closed, attrs)).join('');
  const out = [`<svg viewBox="0 0 ${width} ${height}" xmlns="http://www.w3.org/2000/svg">`, `<defs><clipPath id="map-clip"><rect width="${width}" height="${height}"/></clipPath></defs>`, `<g clip-path="url(#map-clip)" stroke-linejoin="round" stroke-linecap="round">`, `<rect width="${width}" height="${height}" fill="${xml(colors.bg)}"/>`];
  out.push(many(gridLines(view.scale > width ? 10 : 30, view.mode === 'polar' ? 88 : 80), false, `fill="none" stroke="${xml(colors.graticule)}" stroke-width="1" data-layer="graticule"`));
  if (base) {
    out.push(many(base.land, true, `fill="${xml(colors.land)}" fill-rule="evenodd" stroke="${xml(colors.line)}" stroke-width="0.75" data-layer="land"`));
    out.push(many(base.lakes, true, `fill="${xml(colors.bg)}" data-layer="lake"`));
    out.push(many(base.states, false, `fill="none" stroke="${xml(colors.line)}" stroke-opacity="0.7" stroke-width="0.7" stroke-dasharray="4 3" data-layer="state"`));
    out.push(many(base.borders, false, `fill="none" stroke="${xml(colors.line)}" stroke-opacity="0.65" stroke-width="0.8" data-layer="border"`));
  }
  const order = { comparison: 0, input: 1, detail: 1, result: 2 };
  for (const layer of [...(layers ?? [])].sort((a, b) => (order[a.role] ?? 0) - (order[b.role] ?? 0))) {
    const stroke = layer.role === 'result' ? colors.accent : colors.muted;
    if (layer.kind === 'polygon') {
      const d = (layer.rings ?? []).map((ring) => mapPath({ ...view, width, height }, ring, true)).join('');
      if (!d) continue;
      if (layer.grid) {
        out.push(`<path d="${d}" fill="none" stroke="${xml(colors.muted)}" stroke-opacity="0.65" stroke-width="1" data-layer="grid"/>`);
        continue;
      }
      const opacity = layer.cell ? (layer.role === 'result' ? 0.28 : 0.1 * (layer.weight ?? 0.8)) : layer.role === 'coverage' ? 0.07 : 0.14;
      const dash = layer.compacted ? ' stroke-dasharray="5 3"' : '';
      out.push(`<path d="${d}" fill="${xml(stroke)}" fill-opacity="${opacity}" fill-rule="evenodd" stroke="${xml(stroke)}" stroke-width="${layer.role === 'result' ? 2 : 1.5}"${dash} data-layer="polygon"/>`);
    } else if (layer.kind === 'line') {
      const dash = layer.path === 'great-circle' ? '2 4' : layer.role === 'comparison' ? '6 5' : '';
      out.push(path(layer.points, false, `fill="none" stroke="${xml(colors.surface)}" stroke-width="${layer.role === 'result' ? 8 : 4}" data-layer="line-casing"`));
      out.push(path(layer.points, false, `fill="none" stroke="${xml(stroke)}" stroke-width="${layer.role === 'result' ? 3 : 1.5}"${dash ? ` stroke-dasharray="${dash}"` : ''} data-layer="line"`));
    } else if (layer.kind === 'point') {
      for (const [lon, lat] of layer.points ?? []) {
        const d = mapPath({ ...view, width, height }, [[lon, lat]], false);
        const match = /^M(-?[\d.]+) (-?[\d.]+)/.exec(d);
        if (!match) continue;
        const [x, y] = [match[1], match[2]];
        const radius = layer.role === 'result' ? 7 : layer.role === 'detail' ? 2.5 : 5.5;
        out.push(`<circle cx="${x}" cy="${y}" r="${radius}" fill="${xml(stroke)}" data-layer="point"/>`);
        if (layer.label) out.push(`<text x="${num(Number(x) + radius + 5)}" y="${num(Number(y) - radius - 5)}" fill="${xml(colors.text)}" font-family="sans-serif" font-size="12" font-weight="600">${xml(layer.label)}</text>`);
      }
    }
  }
  out.push('</g></svg>');
  return svgWithFooter(out.join(''), captionText, attribution, { background: colors.bg, ink: colors.text });
}

/**
 * The page's rules for selectors starting with `prefix`, with every custom
 * property resolved to the value it has now, so a file carries the look.
 */
export function inlineStyles(prefix) {
  const root = getComputedStyle(document.documentElement);
  const out = [];
  for (const sheet of document.styleSheets) {
    let rules;
    try {
      rules = sheet.cssRules;
    } catch {
      continue;
    }
    for (const r of rules) {
      if (r.selectorText?.split(',').some((sel) => sel.trim().startsWith(prefix))) {
        out.push(r.cssText.replace(/var\((--[\w-]+)\)/g, (_, name) => root.getPropertyValue(name).trim() || 'currentColor'));
      }
    }
  }
  return out.join('');
}

/**
 * A PNG of `source` with a footer strip holding the caption and every
 * attribution line. The strip is drawn in the paper palette whatever the
 * screen shows, so the export reads the same everywhere it is pasted.
 */
export async function pngWithFooter(source, captionText, lines) {
  const scale = source.width / Math.max(1, source.getBoundingClientRect?.().width || source.width);
  const lineHeight = Math.round(16 * scale);
  const pad = Math.round(10 * scale);
  const out = document.createElement('canvas');
  const g = out.getContext('2d');
  const font = `${Math.round(12 * scale)}px sans-serif`;
  g.font = font;
  const fits = (l) => g.measureText(l).width <= source.width - pad * 2;
  // The caption is ink; attribution lines are muted. Both wrap to the width.
  const note = [captionText, ...lines].filter(Boolean).flatMap((t, i) => wrap(t, fits).map((l) => [l, i === 0]));
  out.width = source.width;
  out.height = source.height + pad * 2 + lineHeight * note.length;
  g.fillStyle = '#ffffff';
  g.fillRect(0, 0, out.width, out.height);
  g.drawImage(source, 0, 0);
  g.fillStyle = '#e1e4dd';
  g.fillRect(0, source.height, out.width, Math.max(1, Math.round(scale)));
  g.font = font; // resizing the canvas reset it
  g.textBaseline = 'top';
  note.forEach(([t, isCaption], i) => {
    g.fillStyle = isCaption ? '#0c1116' : '#4f5763';
    g.fillText(t, pad, source.height + pad + i * lineHeight);
  });
  return new Promise((resolve) => out.toBlob(resolve, 'image/png'));
}

/** Hands the reader a file; nothing is uploaded. */
export function saveBlob(blob, name) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

let registryPromise = null;
/** The asset registry, fetched once and only when an export asks for it. */
export const loadRegistry = () =>
  (registryPromise ??= fetch('/assets/registry.json').then((r) => r.json(), () => ({ assets: [] })).catch(() => ({ assets: [] })));
