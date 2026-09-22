<script>
  // The map canvas (web/map-canvas): the tool's inputs and result drawn over
  // the Natural Earth base layer, as a 2D map or a globe. Drag to pan or spin,
  // scroll or pinch to zoom; arrow keys, + and -, and 0 (reset) do the same.
  import { onMount } from 'svelte';
  import { buildLayers, extent } from '../lib/map/layers.js';
  import { decode, forward, frame, inverse, PROJECTION_NAMES } from '../lib/map/projection.js';
  import { colors, draw } from '../lib/map/render.js';
  import { magneticNorth, readoutText } from '../lib/map/readout.js';
  import { clickTarget, dragDegrees, handleAt, handlesOf } from '../lib/map/handles.js';
  import { coordFormat } from '../lib/prefs.js';
  import { say } from '../lib/keys.js';
  import { attributionLines, caption, layersGeoJson, loadRegistry, pngWithFooter, saveBlob } from '../lib/canvas-export.mjs';

  let { tool, args, result, compute, onmove } = $props();

  const kinds = new Set((tool.visualization ?? []).map((v) => v.kind));
  let mode = $state(kinds.has('line-geodesic') ? 'globe' : 'map');
  // The 2D projection the Map button shows: Web Mercator, equirectangular, or polar.
  let projection = $state('map');
  let canvas;
  let base = null;
  let layers = [];
  let view = null;
  let target = null;
  // The canvas's text alternative: the view, what it shows, and the answer.
  let shown = $state('');
  const desc = $derived(shown ? `${mode === 'globe' ? 'Globe' : `${PROJECTION_NAMES[mode]} map`} showing ${shown}. ${result?.summary ?? ''}` : '');
  let readout = $state('');
  let scaleBar = $state({ px: 0, label: '' });
  let legend = $state([]);
  const R_EARTH = 6371008.8;
  // Sets an element's width through the CSSOM (the CSP allows no style attributes).
  function width(node, px) {
    const set = (v) => (node.style.inlineSize = `${Math.round(v)}px`);
    set(px);
    return { update: set };
  }
  // Mercator's scale grows with latitude; the other views are true along meridians.
  const metersPerPixel = (v) => (v.mode === 'map' ? R_EARTH * Math.cos((v.lat * Math.PI) / 180) : R_EARTH) / v.scale;
  /** A round distance (1, 2, or 5 × 10^n meters) about 100 px long at the view's center. */
  function measure(v) {
    const perPx = metersPerPixel(v);
    const target = perPx * 100;
    const p10 = 10 ** Math.floor(Math.log10(target));
    const nice = [5, 2, 1].map((k) => k * p10).find((d) => d <= target) ?? p10;
    const label = nice >= 1000 ? `${(nice / 1000).toLocaleString('en-US')} km` : `${nice.toLocaleString('en-US')} m`;
    return { px: nice / perPx, label };
  }
  let reduced = false;

  const size = () => {
    const r = canvas.getBoundingClientRect();
    return [Math.max(1, r.width), Math.max(1, r.height)];
  };

  function paint() {
    if (!canvas || !view) return;
    const dpr = globalThis.devicePixelRatio || 1;
    const [w, h] = size();
    if (canvas.width !== Math.round(w * dpr)) canvas.width = Math.round(w * dpr);
    if (canvas.height !== Math.round(h * dpr)) canvas.height = Math.round(h * dpr);
    const g = canvas.getContext('2d');
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw(g, { ...view, width: w, height: h }, base, layers, colors(canvas));
    scaleBar = measure(view);
  }

  // Eases the camera to `target` in at most 500 ms (instantly with reduced motion).
  function ease() {
    if (!target) return;
    if (reduced || !view || view.mode !== target.mode) {
      view = target;
      target = null;
      return paint();
    }
    const from = { ...view };
    const to = target;
    const dl = ((to.lon - from.lon + 540) % 360) - 180;
    const t0 = performance.now();
    const step = (now) => {
      if (target !== to) return;
      const k = Math.min(1, (now - t0) / 500);
      const e = 1 - (1 - k) ** 3;
      view = { ...to, lon: from.lon + dl * e, lat: from.lat + (to.lat - from.lat) * e, scale: from.scale * (to.scale / from.scale) ** e };
      paint();
      if (k < 1) requestAnimationFrame(step);
      else target = null;
    };
    requestAnimationFrame(step);
  }

  function reframe() {
    const [w, h] = size();
    target = frame(mode, extent(layers), w, h, { tight: layers.some((l) => l.kind === 'polygon') });
    ease();
  }

  // H3 cells for cell-set layers: outlines and grid rings, one batch each, from the core.
  const batch = async (id, inputs) => {
    const raw = await compute.invokeBatch(id, JSON.stringify(inputs));
    return Array.isArray(raw) ? raw : [];
  };
  const cellSource = {
    boundaries: async (ids) =>
      (await batch('indexing.h3.cell-info', ids.map((cell) => ({ cell })))).map((r) => (r?.ok ? r.result.boundary.map((p) => [p.lon, p.lat]) : [])),
    rings: async (origin, k) =>
      (await batch('indexing.h3.grid-ring', Array.from({ length: k + 1 }, (_, d) => ({ cell: origin, k: d })))).map((r) => (r?.ok ? r.result.cells.map((c) => c.cell) : [])),
  };

  async function rebuild() {
    if (!result?.ok) return;
    layers = await buildLayers(tool, args, result, (input) =>
      compute.invoke('navigation.geodesic.waypoints', input, 'densify'),
      cellSource,
    );
    const cellCount = layers.filter((l) => l.cell).length;
    const what = layers.filter((l) => !l.cell).map((l) => (l.kind === 'line' ? (l.role === 'comparison' ? 'a dashed comparison line' : 'the route line') : l.kind === 'polygon' ? 'the polygon' : l.label ? `point ${l.label}` : `the ${l.role === 'result' ? 'result' : 'input'} point`));
    if (cellCount) what.push(`${cellCount} H3 ${cellCount === 1 ? 'cell' : 'cells'}${layers.some((l) => l.cell && l.role === 'result') ? ', the origin highlighted and the rest fading with grid distance' : ''}`);
    shown = what.join(', ') || 'the world';
    // What the lines mean: the result path, and the other kind of line for comparison.
    const rhumb = kinds.has('line-rhumb');
    const named = (r) => (r ? 'Rhumb line: constant heading' : 'Geodesic: the shortest path');
    legend = [
      layers.some((l) => l.kind === 'line' && l.role === 'result') && { cls: 'solid', text: named(rhumb) },
      layers.some((l) => l.kind === 'line' && l.role === 'comparison') && { cls: 'dashed', text: `${named(!rhumb)}, for comparison` },
      layers.some((l) => l.kind === 'polygon' && !l.cell) && { cls: 'area', text: 'The area' },
      layers.some((l) => l.cell) && { cls: 'area', text: layers.some((l) => l.cell && l.role === 'result') ? 'H3 cells: the origin strongest, fading with grid distance' : 'H3 cells' },
    ].filter(Boolean);
    // A result that lands mid-drag was computed for an earlier position:
    // keep the point under the pointer until the drag ends.
    const held = drag?.handle && drag.at && layers.find((l) => l.field === drag.handle.field);
    if (held) held.points = [drag.at];
    // After the reader has moved a point on the map, keep their view.
    if (keepView) paint();
    else reframe();
  }

  $effect(() => {
    result;
    rebuild();
  });

  function setMode(m) {
    mode = m;
    keepView = false;
    reframe();
  }

  // Pointer: drag pans the map or spins the globe; the readout follows the pointer.
  // A press on an input point drags it (the form follows); a press elsewhere
  // pans; a click without moving sets a single-point tool's point.
  let drag = null;
  let keepView = false;
  const canDrag = !!onmove;
  const local = (e) => {
    const r = canvas.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  };
  function place(field, ll) {
    const d = dragDegrees(ll[1], ll[0], metersPerPixel(view));
    onmove(field, d.lat, d.lon);
  }
  function down(e) {
    canvas.setPointerCapture(e.pointerId);
    const [w, h] = size();
    const [x, y] = local(e);
    const handle = canDrag && view ? handleAt(handlesOf(layers), { ...view, width: w, height: h }, x, y) : null;
    // Keep where on the point the press landed, so the point does not jump.
    const grab = handle ? forward({ ...view, width: w, height: h }, handle.lon, handle.lat) : null;
    drag = { x: e.clientX, y: e.clientY, view: { ...view }, handle, moved: false, dx: grab ? grab[0] - x : 0, dy: grab ? grab[1] - y : 0 };
    target = null;
  }
  function move(e) {
    const r = canvas.getBoundingClientRect();
    const [w, h] = size();
    const ll = view && inverse({ ...view, width: w, height: h }, e.clientX - r.left, e.clientY - r.top);
    showPoint(ll);
    if (canDrag && !drag && view) {
      const [x, y] = local(e);
      canvas.style.cursor = handleAt(handlesOf(layers), { ...view, width: w, height: h }, x, y) ? 'move' : '';
    }
    if (!drag) return;
    if (drag.view.mode === 'polar' && !drag.handle) {
      // Turn the map about the pole, keeping the grabbed meridian under the pointer.
      const hs = drag.view.lat >= 0 ? 1 : -1;
      const bearing = (cx, cy) => Math.atan2(cx - r.left - w / 2, hs * (cy - r.top - h / 2));
      view = { ...drag.view, lon: drag.view.lon - ((bearing(e.clientX, e.clientY) - bearing(drag.x, drag.y)) * 180) / Math.PI };
      if (Math.hypot(e.clientX - drag.x, e.clientY - drag.y) > 3) drag.moved = true;
      paint();
      return;
    }
    if (Math.hypot(e.clientX - drag.x, e.clientY - drag.y) > 3) drag.moved = true;
    if (drag.handle) {
      const [x, y] = local(e);
      const ll = inverse({ ...view, width: w, height: h }, x + drag.dx, y + drag.dy);
      if (!ll || !drag.moved) return;
      keepView = true;
      // Move the point now; the recomputed result redraws the rest.
      const layer = layers.find((l) => l.field === drag.handle.field);
      drag.at = [ll[0], ll[1]];
      if (layer) layer.points = [drag.at];
      paint();
      place(drag.handle.field, ll);
      return;
    }
    const d = 180 / Math.PI / drag.view.scale;
    view = {
      ...drag.view,
      lon: drag.view.lon - (e.clientX - drag.x) * d,
      lat: Math.max(-85, Math.min(85, drag.view.lat + (e.clientY - drag.y) * d)),
    };
    paint();
  }
  // The readout in the chosen format. Grid formats ask the core; one request
  // is in flight at a time and only the newest pointer position is kept.
  let fmt = 'dd';
  let asking = false;
  let queued;
  async function showPoint(ll) {
    if (!ll) return (readout = '');
    if (asking) return (queued = ll);
    asking = true;
    const text = await readoutText(ll[1], ll[0], fmt, {
      invoke: compute?.invokeLatest && ((id, input) => compute.invokeLatest(id, input, 'readout')),
      metersPerPixel: metersPerPixel(view),
    });
    asking = false;
    if (text !== null) readout = text;
    if (queued) {
      const next = queued;
      queued = undefined;
      showPoint(next);
    }
  }
  function up(e) {
    const click = drag && !drag.moved && !drag.handle && e?.type === 'pointerup';
    drag = null;
    const field = canDrag && click && clickTarget(tool);
    if (!field) return;
    const [w, h] = size();
    const [x, y] = local(e);
    const ll = inverse({ ...view, width: w, height: h }, x, y);
    if (!ll) return;
    keepView = true;
    place(field, ll);
  }
  function zoom(f) {
    target = null;
    const [w, h] = size();
    view = { ...view, scale: Math.max(Math.min(w, h) / 7, Math.min(view.scale * f, w * 5000)) };
    paint();
  }
  function wheel(e) {
    e.preventDefault();
    zoom(e.deltaY < 0 ? 1.15 : 1 / 1.15);
  }
  function key(e) {
    const step = 60 / view.scale * (180 / Math.PI);
    const moves = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] };
    if (moves[e.key]) {
      view = view.mode === 'polar'
        ? { ...view, lon: view.lon + (moves[e.key][0] ? Math.sign(moves[e.key][0]) * 10 : 0) }
        : { ...view, lon: view.lon + moves[e.key][0], lat: Math.max(-85, Math.min(85, view.lat + moves[e.key][1])) };
      paint();
    } else if (e.key === '+' || e.key === '=') zoom(1.25);
    else if (e.key === '-') zoom(0.8);
    else if (e.key === '0') reframe();
    else return;
    e.preventDefault();
  }

  // Export (web/map-canvas, "Export"): the view as drawn, with the caption and
  // every attribution it owes in a footer strip, or its layers as GeoJSON.
  async function exportPng() {
    paint();
    const lines = attributionLines(result, await loadRegistry());
    saveBlob(await pngWithFooter(canvas, caption(tool, result), lines), `${tool.id}.png`);
  }
  function exportGeoJson() {
    const text = `${JSON.stringify(layersGeoJson(layers, tool), null, 2)}\n`;
    saveBlob(new Blob([text], { type: 'application/geo+json' }), `${tool.id}.geojson`);
  }

  // Detail beyond Natural Earth 1:110m: say the base map is generalized.
  const generalized = $derived(view && readout !== undefined && view.scale * (Math.PI / 180) > 60);

  // North is up at the center of every view here: the Mercator map and the
  // globe are both drawn north-up. Magnetic north shows when the tool's
  // result carries a declination.
  const magnetic = $derived(magneticNorth(result));
  function turn(node, deg) {
    const set = (d) => (node.style.rotate = `${d}deg`);
    set(deg);
    return { update: set };
  }

  // `c` switches between the flat map and the globe.
  function onShortcut(e) {
    if (e.detail !== 'canvas') return;
    e.preventDefault();
    setMode(mode === 'globe' ? projection : 'globe');
    say(mode === 'globe' ? 'Globe' : PROJECTION_NAMES[mode]);
  }

  onMount(() => {
    addEventListener('gp-shortcut', onShortcut);
    fmt = coordFormat();
    const onPrefs = () => (fmt = coordFormat());
    addEventListener('gp-prefs', onPrefs);
    reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    fetch('/basemap/ne-110m.json')
      .then((r) => r.json())
      .then((ne) => {
        base = {
          land: ne.land.map(decode),
          lakes: ne.lakes.map(decode),
          borders: ne.borders.map(decode),
          states: (ne.states ?? []).map(decode),
          places: (ne.places ?? []).map(([name, lon, lat, minZoom]) => ({ name, lon: lon / 100, lat: lat / 100, minZoom })),
        };
        paint();
      })
      .catch(() => {});
    const ro = new ResizeObserver(() => paint());
    ro.observe(canvas);
    // Repaint when the display mode changes the design tokens.
    const mo = new MutationObserver(() => paint());
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'style'] });
    canvas.addEventListener('wheel', wheel, { passive: false });
    return () => {
      removeEventListener('gp-shortcut', onShortcut);
      removeEventListener('gp-prefs', onPrefs);
      ro.disconnect();
      mo.disconnect();
      canvas.removeEventListener('wheel', wheel);
    };
  });
</script>

<figure class="map card">
  <div class="map-bar">
    <div class="segmented" role="group" aria-label="View">
      <button type="button" aria-pressed={mode !== 'globe'} onclick={() => setMode(projection)}>Map</button>
      <button type="button" aria-pressed={mode === 'globe'} onclick={() => setMode('globe')}>Globe</button>
    </div>
    {#if mode !== 'globe'}
      <label class="map-projection"><span class="sr-only">Projection</span>
        <select value={projection} onchange={(e) => { projection = e.currentTarget.value; setMode(projection); }}>
          <option value="map">Web Mercator</option>
          <option value="equirect">Equirectangular</option>
          <option value="polar">Polar</option>
        </select>
      </label>
    {/if}
    <div class="map-tools" role="group" aria-label="Zoom">
      <button type="button" onclick={() => zoom(1 / 1.5)} aria-label="Zoom out">−</button>
      <button type="button" onclick={() => zoom(1.5)} aria-label="Zoom in">+</button>
      <button type="button" onclick={() => { keepView = false; reframe(); }} aria-label="Fit the result in view">Fit</button>
    </div>
    <div class="map-tools" role="group" aria-label="Export">
      <button type="button" onclick={exportPng} aria-label="Download the view as PNG, with attribution">PNG</button>
      <button type="button" onclick={exportGeoJson} aria-label="Download the layers as GeoJSON">GeoJSON</button>
    </div>
  </div>
  <div class="map-stage">
    <canvas
      bind:this={canvas}
      role="img"
      aria-label={desc || 'Map'}
      tabindex="0"
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onkeydown={key}
    ></canvas>
    <div class="map-north" aria-hidden="true">
      <span class="north-true"><svg viewBox="0 0 16 24" width="16" height="24"><path d="M8 1 L14 21 L8 17 L2 21 Z" /></svg>N</span>
      {#if magnetic !== null}
        <span class="north-mag" title="Magnetic north"><svg use:turn={magnetic} viewBox="0 0 16 24" width="16" height="24"><path d="M8 1 L8 23 M4 6 L8 1 L12 6" /></svg>MN</span>
      {/if}
    </div>
  </div>
  {#if legend.length}
    <ul class="map-legend">
      {#each legend as l}<li><span class={`swatch ${l.cls}`} aria-hidden="true"></span>{l.text}</li>{/each}
    </ul>
  {/if}
  <p class="map-readout" aria-hidden="true">
    <span class="scale">{#if scaleBar.px > 0}<span class="scale-bar" use:width={scaleBar.px}></span>{scaleBar.label}{/if}</span>
    <span>{readout || (canDrag && clickTarget(tool) ? 'Click to set the point, or drag it' : canDrag && tool.inputs.properties.lat1 ? 'Drag A or B to move them' : mode === 'globe' ? 'Drag to turn the globe' : 'Drag to pan, scroll to zoom')}</span>
    <span>{PROJECTION_NAMES[mode]} · Natural Earth{generalized ? ' (generalized at this zoom)' : ''}</span>
  </p>
</figure>
