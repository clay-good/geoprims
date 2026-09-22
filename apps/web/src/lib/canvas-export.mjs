// Exporting the canvas (web/map-canvas, "Export"): a PNG with the result's
// caption and every attribution the view owes in a footer strip, an SVG of a
// vector diagram, and GeoJSON of every geographic layer drawn.
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
    const properties = { role: l.role, tool: tool.id, toolVersion: tool.version, ...(l.label ? { label: l.label } : {}) };
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
