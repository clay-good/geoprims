<script>
  // The map canvas (web/map-canvas): the tool's inputs and result drawn over
  // the Natural Earth base layer, as a 2D map or a globe. Drag to pan or spin,
  // scroll or pinch to zoom; arrow keys, + and -, and 0 (reset) do the same.
  import { onMount } from 'svelte';
  import { buildLayers, extent } from '../lib/map/layers.js';
  import { decode, frame, inverse } from '../lib/map/projection.js';
  import { colors, draw } from '../lib/map/render.js';

  let { tool, args, result, compute } = $props();

  const kinds = new Set((tool.visualization ?? []).map((v) => v.kind));
  let mode = $state(kinds.has('line-geodesic') ? 'globe' : 'map');
  let canvas;
  let base = null;
  let layers = [];
  let view = null;
  let target = null;
  let desc = $state('');
  let readout = $state('');
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
    target = frame(mode, extent(layers), w, h);
    ease();
  }

  async function rebuild() {
    if (!result?.ok) return;
    layers = await buildLayers(tool, args, result, (input) =>
      compute.invoke('navigation.geodesic.waypoints', input, 'densify'),
    );
    const what = layers.map((l) => (l.kind === 'line' ? (l.role === 'comparison' ? 'a dashed comparison line' : 'the route line') : l.kind === 'polygon' ? 'the polygon' : `point ${l.label || 'the result'}`));
    desc = `${mode === 'globe' ? 'Globe' : 'Map'} showing ${what.join(', ') || 'the world'}. ${result.summary ?? ''}`;
    reframe();
  }

  $effect(() => {
    result;
    rebuild();
  });

  function setMode(m) {
    mode = m;
    reframe();
  }

  // Pointer: drag pans the map or spins the globe; the readout follows the pointer.
  let drag = null;
  function down(e) {
    canvas.setPointerCapture(e.pointerId);
    drag = { x: e.clientX, y: e.clientY, view: { ...view } };
    target = null;
  }
  function move(e) {
    const r = canvas.getBoundingClientRect();
    const [w, h] = size();
    const ll = view && inverse({ ...view, width: w, height: h }, e.clientX - r.left, e.clientY - r.top);
    readout = ll ? `${ll[1].toFixed(4)}°, ${ll[0].toFixed(4)}°` : '';
    if (!drag) return;
    const d = 180 / Math.PI / drag.view.scale;
    view = {
      ...drag.view,
      lon: drag.view.lon - (e.clientX - drag.x) * d,
      lat: Math.max(-85, Math.min(85, drag.view.lat + (e.clientY - drag.y) * d)),
    };
    paint();
  }
  function up() {
    drag = null;
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
      view = { ...view, lon: view.lon + moves[e.key][0], lat: Math.max(-85, Math.min(85, view.lat + moves[e.key][1])) };
      paint();
    } else if (e.key === '+' || e.key === '=') zoom(1.25);
    else if (e.key === '-') zoom(0.8);
    else if (e.key === '0') reframe();
    else return;
    e.preventDefault();
  }

  // Detail beyond Natural Earth 1:110m: say the base map is generalized.
  const generalized = $derived(view && readout !== undefined && view.scale * (Math.PI / 180) > 60);

  onMount(() => {
    reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    fetch('/basemap/ne-110m.json')
      .then((r) => r.json())
      .then((ne) => {
        base = { land: ne.land.map(decode), lakes: ne.lakes.map(decode), borders: ne.borders.map(decode) };
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
      ro.disconnect();
      mo.disconnect();
      canvas.removeEventListener('wheel', wheel);
    };
  });
</script>

<figure class="map card">
  <div class="map-bar" role="group" aria-label="View">
    <button type="button" aria-pressed={mode === 'map'} onclick={() => setMode('map')}>Map</button>
    <button type="button" aria-pressed={mode === 'globe'} onclick={() => setMode('globe')}>Globe</button>
    <button type="button" onclick={reframe} aria-label="Fit the result in view">Fit</button>
  </div>
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
  <p class="map-readout" aria-hidden="true">
    <span>{readout}</span>
    <span>{mode === 'globe' ? 'Orthographic globe' : 'Web Mercator'} · Natural Earth{generalized ? ' · Base map generalized at this zoom' : ''}</span>
  </p>
</figure>
