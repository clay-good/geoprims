<script>
  // The interactive tool: schema-driven form, live answer card, permalinks.
  // Server-rendered with the worked example, so the answer is in the HTML.
  import { onMount } from 'svelte';
  import { isPinned, numberFormat, PROFILES, profile, recordUse, setProfile, togglePin, toolOptions } from '../lib/prefs.js';
  import MapCanvas from './MapCanvas.svelte';
  import { diagram } from '../lib/diagrams.js';
  import { attributionLines, caption, inlineStyles, loadRegistry, saveBlob, svgWithFooter } from '../lib/canvas-export.mjs';
  import { copyText, sharePayload } from '../lib/copy.js';
  import { cellText, rowTables } from '../lib/rows.js';

  import { isNumeric, isSigned, flipped, stepLabel, stepped, stepsOf } from '../lib/fields.mjs';
  import { keyboardInset, trackKeyboard } from '../lib/keyboard.mjs';
  import { assetMessage } from '../lib/messages.js';
  import { cameFrom, chainHref, chainState, chainTargets } from '../lib/chain.mjs';
  import { degrees, pairOf } from '../lib/coordinate.mjs';
  import { checkSize } from '../lib/import.mjs';
  import { csvProjected, csvRows, importReport, rowsFor, rowsText } from '../lib/import-rows.mjs';
  import { CSV_CRS, SPCS_UNITS } from '../lib/crs.mjs';
  import { exportText, fileName, FORMATS as EXPORTS, pointsOf } from '../lib/export.mjs';
  // `embedded` is the home page's featured copy: it leaves the page URL alone,
  // stays out of the recent list, and links out to the tool's own page instead.
  let { tool, example, initial, embedded = false } = $props();

  const fields = Object.entries(tool.inputs.properties).filter(([k]) => k !== 'options');
  const required = new Set(tool.inputs.required);
  // Everyday inputs first; the rest (ellipsoid, rarely changed options) under "More options".
  const isCore = ([k, schema]) => schema['x-core'] || required.has(k);
  const coreFields = fields.filter(isCore);
  const moreFields = fields.filter((f) => !isCore(f));
  const outputOrder = Object.keys(tool.outputs.properties);

  // The numeric input contract (ux/mobile-and-field): never type="number",
  // which fights unit-carrying text like "29.92 inHg"; a decimal keypad on
  // anything numeric; and a ± toggle where a value can be negative.
  function flipSign(name) {
    values[name] = flipped(values[name]);
    edited();
  }
  /** A Field-mode step button: moves this field by `delta`, unit and all. */
  function stepField(name, delta) {
    values[name] = stepped(values[name], delta, numberFormat());
    edited();
  }

  // List inputs (traverse courses, polygon corners) edit as one row per line,
  // columns in schema order, separated by commas or tabs (a spreadsheet paste).
  const columns = (schema) => Object.keys(schema.items?.properties ?? {});
  const lastField = fields.at(-1)?.[0];
  const isList = (schema) => schema.type === 'array';
  const NUMBER = /^[-+]?[\d.]+(e[-+]?\d+)?$/i;
  function toText(k, v) {
    if (v === undefined) return '';
    const schema = tool.inputs.properties[k];
    if (!isList(schema) || !Array.isArray(v)) return String(v);
    // Trailing empty cells (an optional ring, a name left out) are not written,
    // so a row reads "37, -109.05", not "37, -109.05,".
    return v.map((row) => columns(schema).map((c) => row[c] ?? '').join(', ').replace(/(,\s*)+$/, '')).join('\n');
  }
  function fromList(schema, text) {
    return text
      .split('\n')
      .filter((line) => line.trim() !== '')
      .map((line) => {
        const cells = line.split(line.includes('\t') ? '\t' : ',').map((c) => c.trim());
        return Object.fromEntries(
          columns(schema)
            .map((c, i) => [c, cells[i]])
            .filter(([, v]) => v !== undefined && v !== '')
            .map(([c, v]) => [c, NUMBER.test(v) ? Number(v) : v]),
        );
      });
  }

  // Unit symbols people read (the core accepts all of these spellings).
  const FRIENDLY = { degC: '°C', degF: '°F', deg: 'degrees', arcsec: 'arc-seconds', arcmin: 'arc-minutes', m2: 'm²', km2: 'km²', ft2: 'ft²', ppm: 'parts per million' };
  const friendly = (u) => FRIENDLY[u] ?? u;
  /** Example text as a person would type it: "30 degC" becomes "30 °C". */
  const readable = (text) => text.replace(/(\d)\s*deg([CF])\b/g, '$1 °$2');
  const exampleText = (k) => readable(toText(k, example[k]));

  let values = $state(Object.fromEntries(fields.map(([k]) => [k, exampleText(k)])));
  let result = $state(initial);
  let isExample = $state(true);
  let stale = $state(false);
  let elapsedMs = $state(null);
  let copied = $state('');
  let linkNote = $state('');
  // Embedded: the permalink rides on the "open the full tool" link instead of the URL.
  let fragment = $state('#example');
  const toolHref = $derived(tool.route + fragment);
  let compute = $state.raw(null);
  let timer;
  // The report dialog is imported on first click, so nothing loads before then.
  // A missing or damaged data file reads as the dataset's name and what to
  // do, not as an asset id and a file name (web/offline-pwa).
  let assetRegistry = $state(null);
  const assetError = $derived(
    result && !result.ok && /^ASSET_/.test(result.error.code) && assetRegistry
      ? assetMessage(result.error, assetRegistry, globalThis.navigator?.onLine !== false)
      : null,
  );
  $effect(() => {
    if (result && !result.ok && /^ASSET_/.test(result.error.code) && !assetRegistry) {
      fetch('/assets/registry.json').then((r) => r.json()).then((r) => (assetRegistry = r), () => (assetRegistry = { assets: [] }));
    }
  });
  // The diagram as a file, with the caption and any data attribution written in.
  async function exportSvg() {
    const lines = attributionLines(result, await loadRegistry(), { basemap: false });
    const css = getComputedStyle(document.documentElement);
    const markup = svgWithFooter(dg.markup, caption(tool, result), lines, {
      style: inlineStyles('.dg'),
      background: css.getPropertyValue('--surface').trim() || '#ffffff',
      ink: css.getPropertyValue('--muted').trim() || '#4f5763',
    });
    saveBlob(new Blob([markup], { type: 'image/svg+xml' }), `${tool.id}.svg`);
  }
  let ReportDialog = $state(null);
  // Batch mode loads only when a reader opens it.
  let BatchPanel = $state(null);
  const batchable = Object.entries(tool.inputs.properties).some(([n, s]) => n !== 'options' && s.type !== 'array');
  const loadBatch = async () => (BatchPanel ??= (await import('./BatchPanel.svelte')).default);
  let reporting = $state(false);

  async function openReport() {
    ReportDialog ??= (await import('./ReportDialog.svelte')).default;
    reporting = true;
  }

  // Display text comes from the core (display precision, grouping, unit labels).
  // The primary result is the first declared output present (optional outputs may be absent).
  const primary = $derived(result?.ok ? outputOrder.find((k) => result.display?.[k] !== undefined) : undefined);
  const answer = $derived(result?.ok && primary ? result.display[primary] : '');
  // Atlas: a big number with a small, muted unit ("7,932" and "ft").
  const answerParts = $derived(/^(.*\d)\s+(\D.*)$/.exec(answer)?.slice(1) ?? [answer, '']);
  // Cautions first, then accuracy notes, then info (codes registry severities).
  const RANK = { caution: 0, accuracy: 1, info: 2 };
  const severityOf = (code) => tool.severity[code] ?? 'info';
  const warnings = $derived(
    result?.ok
      ? result.meta.warnings.filter((w) => w.code !== 'EXPERIMENTAL_TOOL').sort((a, b) => RANK[severityOf(a.code)] - RANK[severityOf(b.code)])
      : [],
  );
  // Status outputs ("Beyond your 15 kt crosswind limit") are a judgment, not a
  // value: they get their own line with an icon and the threshold's source,
  // and stay out of the list of facts.
  const STATUS_MARK = { Within: '✓', Near: '!', Beyond: '✕', Meets: '✓', Does: '✕' };
  const statusOf = (t) => tool.statusSources?.[t];
  const statuses = $derived(
    result?.ok
      ? Object.entries(result.result ?? {})
          .filter(([k, v]) => statusOf(k) && typeof v === 'string')
          .map(([k, phrase]) => ({ key: k, phrase, mark: STATUS_MARK[phrase.split(' ')[0]] ?? '•', source: statusOf(k) }))
      : [],
  );
  const secondary = $derived(
    result?.ok ? Object.entries(result.display ?? {}).filter(([k]) => k !== primary && !statusOf(k)) : [],
  );
  // A list of rows is part of the answer, not a footnote: a decoder's groups, a
  // forecast's periods, a lookup's matches. The rule lives in lib/rows.js.
  const tables = $derived(rowTables(result));
  const columnTitle = (key, column) =>
    tool.outputs.properties[key]?.items?.properties?.[column]?.title ?? column;
  // An input error names its field as a JSON pointer ("/points/2/lat" is the points field): mark it, and let the answer card jump to it.
  const errorField = $derived(result && !result.ok ? result.error.field?.split('/')[1] : undefined);
  const badField = $derived(errorField && errorField in tool.inputs.properties ? errorField : undefined);
  function goToField() {
    const el = document.getElementById(`field-${badField}`);
    el?.closest('details')?.setAttribute('open', '');
    el?.focus();
  }

  // The unit profile switch: shown when a result has units a profile changes (angles never change).
  const hasUnits = Object.values(tool.outputs.properties).some((o) => o['x-quantity'] && o['x-quantity'] !== 'angle');
  let unitProfile = $state('');
  const FACTS = 6;
  let allFacts = $state(false);
  const facts = $derived(allFacts ? secondary : secondary.slice(0, FACTS));

  // The canvas draws geographic tools: lines, points, polygons, or a lat/lon input.
  const GEO = new Set(['line-geodesic', 'line-rhumb', 'point', 'polygon']);
  const showMap = (tool.visualization ?? []).some((v) => GEO.has(v.kind)) || ('lat' in tool.inputs.properties && 'lon' in tool.inputs.properties);
  let drawnArgs = $state(example);
  const dg = $derived(result?.ok ? diagram(tool.id, drawnArgs, result) : null);
  // A tool whose meaning is a picture shows a compact one directly under the
  // answer, as well as the full canvas in its place below (contracts/page-chrome,
  // "Fixed tool-page anatomy"). The full one carries the description; this copy
  // is the same picture, so it is decoration.
  const inlineDg = $derived(tool['x-diagram-inline'] && result?.ok ? diagram(tool.id, drawnArgs, result, 'inline') : null);

  // Scene playback (map-canvas "Animated scenes"): the playhead is the tool's
  // timeline input, so every frame is a core result and the permalink keeps it.
  const tl = tool.timeline;
  const seconds = (q) => (q ? q.value * ({ s: 1, min: 60, h: 3600 }[q.unit] ?? 1) : 0);
  const sceneEnd = $derived(tl && result?.ok ? seconds(result.result[tl.end]) : 0);
  const keyMoment = $derived(tl && result?.ok ? seconds(result.result[tl.key]) : 0);
  let playing = $state(false);
  let speed = $state(1);
  let loop = $state(false);
  const playhead = $derived.by(() => {
    const v = tl ? String(values[tl.input] ?? '').trim() : '';
    const n = Number.parseFloat(v);
    return v === '' || !Number.isFinite(n) ? keyMoment : n;
  });
  function setPlayhead(t) {
    values[tl.input] = `${Math.round(t * 10) / 10} s`;
    isExample = false;
    run();
  }
  function togglePlay() {
    if (playing) return (playing = false);
    playing = true;
    let t = playhead >= sceneEnd - 1e-9 ? 0 : playhead;
    let last = performance.now();
    const step = (now) => {
      if (!playing) return;
      // The whole scene takes about 8 seconds at 1×.
      t += ((now - last) / 1000) * speed * (sceneEnd / 8);
      last = now;
      if (t >= sceneEnd) {
        if (loop) t = 0;
        else {
          t = sceneEnd;
          playing = false;
        }
      }
      setPlayhead(t);
      if (playing) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  function args() {
    const a = {};
    for (const [k, schema] of fields) {
      const v = values[k].trim();
      if (v === '') continue;
      a[k] = isList(schema) ? fromList(schema, v) : schema.type === 'number' && NUMBER.test(v) ? Number(v) : v;
    }
    // The unit profile and number format from settings (and in the permalink).
    const o = toolOptions();
    if (o && tool.inputs.properties.options) a.options = o;
    return a;
  }

  /** Recomputes; `settingsOnly` re-runs for new settings without touching the URL. */
  async function run(settingsOnly = false) {
    stale = true;
    elapsedMs = null;
    const a = args();
    const out = await compute.invoke(tool.id, a, 'invoke', (elapsed) => { elapsedMs = elapsed; });
    if (!out) return; // superseded by a newer edit
    result = out;
    drawnArgs = a;
    stale = false;
    elapsedMs = null;
    if (settingsOnly) return;
    const enc = await compute.encodeLink({ i: a });
    if (!enc?.ok) return;
    if (embedded) fragment = `#${enc.result.fragment}`;
    else history.replaceState(null, '', `#${enc.result.fragment}`);
  }

  function edited() {
    isExample = false;
    stale = true;
    elapsedMs = null;
    compute.cancel('invoke');
    clearTimeout(timer);
    timer = setTimeout(run, 150);
  }

  function clearAll() {
    for (const [k] of fields) values[k] = '';
    edited();
  }

  function tryExample() {
    for (const [k] of fields) values[k] = exampleText(k);
    isExample = true;
    if (embedded) fragment = '#example';
    else history.replaceState(null, '', '#example');
    run();
  }

  let from = $state(null);
  let copiedRow = $state('');
  let sendOpen = $state('');
  /** Destination → link, built when a row's "Send to" is opened. */
  let sendLinks = $state({});

  /**
   * The tools that can take this value, with the link that opens each of them
   * with the value already in it (web/app-shell, "Tool chaining").
   */
  // A tool with a latitude and a longitude takes a coordinate in any notation
  // the catalog can read (web/app-shell, "Schema-driven input forms").
  const pair = pairOf(tool);
  let pasted = $state('');
  let read = $state(null);
  let reading = false;

  async function readPasted() {
    const text = pasted;
    if (!compute || reading) return;
    reading = true;
    try {
      read = await compute.readCoordinate(text);
    } finally {
      reading = false;
    }
    if (read?.ok) applyRead(read.lat, read.lon);
  }

  /** Puts a read coordinate into the tool's own latitude and longitude. */
  function applyRead(lat, lon) {
    values[pair.lat] = degrees(lat);
    values[pair.lon] = degrees(lon);
    edited();
  }

  /** A point moved or set on the map: its fields follow, and the tool recomputes. */
  function moveFromMap(field, lat, lon) {
    values[field.lat] = lat;
    values[field.lon] = lon;
    if (pair && field.lat === pair.lat) [read, pasted] = [null, ''];
    edited();
  }

  /** The other reading, when the text carried no hemisphere or label. */
  function swapRead() {
    if (!read?.ambiguous) return;
    const { lat, lon } = read.ambiguous;
    read = { ...read, lat, lon, ambiguous: { lat: lon, lon: lat }, message: `Read as lat, lon: ${degrees(lat)}°, ${degrees(lon)}°` };
    applyRead(lat, lon);
  }

  // Downloading a result (web/io-formats). The geographic formats are offered
  // only when the result actually holds a point.
  let exportOpen = $state(false);
  const exportable = $derived(
    result?.ok ? EXPORTS.filter((f) => !f.geographic || pointsOf(tool, result).length > 0) : [],
  );

  /** Writes one format to a file the browser saves. */
  function download(format) {
    const text = exportText(format.id, {
      tool,
      args: args(),
      result,
      display: result.display ?? {},
      today: new Date().toISOString(),
    });
    const url = URL.createObjectURL(new Blob([text], { type: `${format.type};charset=utf-8` }));
    const a = document.createElement('a');
    a.href = url;
    a.download = fileName(tool, format.id);
    a.click();
    URL.revokeObjectURL(url);
    exportOpen = false;
  }

  // Bringing a file into a list of points (web/io-formats): a KML boundary, a
  // GPX track, a GeoJSON polygon, a CSV of coordinates.
  let importNote = $state({});
  const takesPoints = (schema) => isList(schema) && ['lat', 'lon'].every((c) => c in (schema.items?.properties ?? {}));

  async function importInto(name, file) {
    if (!file || !compute) return;
    const schema = tool.inputs.properties[name];
    const tooBig = checkSize(file.size);
    if (tooBig) {
      importNote = { ...importNote, [name]: { ok: false, text: tooBig } };
      return;
    }
    const parsed = await compute.readFileBytes(file.name, new Uint8Array(await file.arrayBuffer()));
    // A CSV says nothing about which column is which, so the reader chooses,
    // starting from what the headers suggest.
    if ((parsed.format === 'csv' || parsed.format === 'tsv') && parsed.ok) {
      const suggested = parsed.columns ?? { lat: -1, lon: -1 };
      const swapped = parsed.swapped === true;
      importNote = {
        ...importNote,
        [name]: {
          ok: true,
          csv: { fileName: file.name, parsed, lat: swapped ? suggested.lon : suggested.lat, lon: swapped ? suggested.lat : suggested.lon, swapped, crs: 'wgs84', zone: '', hemisphere: 'N', unit: 'legal' },
          text: swapped
            ? `The column named latitude holds values beyond ±90, so the columns look swapped. Check them below.`
            : `Which columns are latitude and longitude?`,
        },
      };
      return;
    }
    const filled = rowsFor(schema, parsed);
    importNote = { ...importNote, [name]: { ok: filled.ok, text: importReport(file.name, parsed.format, parsed, filled) } };
    if (filled.ok) {
      values[name] = rowsText(schema, filled.rows);
      edited();
    }
  }

  /** A projected CSV's columns start from headers like "easting" and "northing". */
  function pickCrs(name, crs) {
    const c = importNote[name].csv;
    c.crs = crs;
    if (crs !== 'wgs84') {
      const find = (re) => c.parsed.headers.findIndex((h) => re.test(h.trim()));
      const e = find(/^(e|east|easting|x)$/i);
      const n = find(/^(n|north|northing|y)$/i);
      if (e >= 0 && n >= 0) [c.lat, c.lon] = [n, e];
    }
    importNote = { ...importNote };
  }

  /** Fills a list from a CSV once its columns, and the system they are in, are chosen. */
  async function useColumns(name) {
    const note = importNote[name];
    const schema = tool.inputs.properties[name];
    const c = note.csv;
    let filled;
    if (c.crs === 'wgs84') filled = csvRows(schema, c.parsed, Number(c.lat), Number(c.lon));
    else if (!String(c.zone).trim()) filled = { ok: false, message: c.crs === 'utm' ? 'Enter the UTM zone, 1 to 60.' : 'Enter the State Plane zone, like 3702 or Pennsylvania South.' };
    else {
      const crs = c.crs === 'utm' ? { kind: 'utm', zone: Number(c.zone), hemisphere: c.hemisphere } : { kind: 'spcs', zone: String(c.zone).trim(), unit: c.unit };
      if (crs.kind === 'utm' && !(Number.isInteger(crs.zone) && crs.zone >= 1 && crs.zone <= 60)) filled = { ok: false, message: 'A UTM zone is a whole number from 1 to 60.' };
      else filled = await csvProjected(schema, c.parsed, Number(c.lon), Number(c.lat), crs, compute.invokeBatch);
    }
    const reported = { ...c.parsed, converted: filled.converted, datum: filled.datum };
    importNote = { ...importNote, [name]: { ...note, ok: filled.ok, text: importReport(c.fileName, c.parsed.format, reported, filled), csv: filled.ok ? null : c } };
    if (filled.ok) {
      values[name] = rowsText(schema, filled.rows);
      edited();
    }
  }

  /** The catalog, fetched once and only when a reader asks to send a value. */
  let tools = null;
  const allTools = async () => (tools ??= (await (await fetch('/catalog/v1.json')).json()).tools);

  async function openSend(name, text) {
    sendOpen = sendOpen === name ? '' : name;
    if (!sendOpen || sendLinks[name] || !compute) return;
    const targets = chainTargets(await allTools(), tool, name);
    const links = [];
    for (const target of targets) {
      const enc = await compute.encodeLink(chainState(target, text, tool.id));
      if (enc?.ok) links.push({ ...target, href: chainHref(target, enc.result.fragment) });
    }
    sendLinks = { ...sendLinks, [name]: links };
  }
  async function copyRow(k, v) {
    await navigator.clipboard.writeText(v);
    copiedRow = k;
    setTimeout(() => (copiedRow = ''), 1500);
  }

  /**
   * The share sheet where there is one, the clipboard where there is not.
   * Either way the same sentence, reference, and link.
   */
  async function share() {
    const href = embedded ? new URL(toolHref, location.origin).href : location.href;
    if (navigator.share) {
      try {
        await navigator.share(sharePayload({ answer, result, tool, args: args(), href }));
        return;
      } catch {
        // A dismissed sheet is not a failure; fall through to the clipboard.
      }
    }
    await copy('link');
  }

  async function copy(kind) {
    const href = embedded ? new URL(toolHref, location.origin).href : location.href;
    // The day the reader copied it, which belongs to the copy, not the result.
    const today = new Date().toISOString().slice(0, 10);
    const text = copyText(kind, { answer, result, tool, args: args(), href, today });
    await navigator.clipboard.writeText(text);
    copied = kind;
    setTimeout(() => (copied = ''), 1500);
  }

  // On phones, a slim bar keeps the answer in view once the big number scrolls away.
  let answerCard = $state(null);
  let answerHidden = $state(false);
  function checkAnswer() {
    if (!answerCard) return;
    const box = answerCard.getBoundingClientRect();
    // What the keyboard covers is not on screen, however tall the window says
    // it is, so the card counts as hidden behind it too.
    const bottom = innerHeight - keyboardInset(visualViewport, innerHeight);
    answerHidden = box.bottom < 0 || box.top > bottom;
  }
  onMount(() => {
    addEventListener('scroll', checkAnswer, { passive: true });
    // The bar rides above the on-screen keyboard, which covers a fixed
    // element pinned to the bottom of the page.
    const untrack = trackKeyboard(document.documentElement, window, checkAnswer);
    return () => {
      removeEventListener('scroll', checkAnswer);
      untrack();
    };
  });

  let pinned = $state(false);
  function pin() {
    togglePin(tool);
    pinned = isPinned(tool.id);
  }

  onMount(async () => {
    if (!embedded) {
      pinned = isPinned(tool.id);
      recordUse(tool);
    }
    unitProfile = profile();
    addEventListener('gp-prefs', () => {
      unitProfile = profile();
      if (compute) run(true);
    });
    if (typeof WebAssembly !== 'object') {
      const { NO_WASM } = await import('../lib/messages.js');
      result = { ok: false, error: { code: 'UNSUPPORTED', message: NO_WASM } };
    }
    compute = await import('../lib/compute.js');
    if (embedded) {
      if (toolOptions()) run(true);
      return;
    }
    const hash = location.hash.slice(1);
    if (hash && hash !== 'example') {
      const d = await compute.decodeLink(hash);
      if (d.ok && d.result.kind === 'state') {
        for (const [k] of fields) values[k] = toText(k, d.result.state.i?.[k]);
        isExample = false;
        // A "send to" link says which tool the value came from.
        const id = d.result.state.c;
        if (id) from = cameFrom(d.result.state, await allTools());
        run();
      } else if (!d.ok) {
        linkNote = d.error.code === 'UNSUPPORTED' ? d.error.message : 'This link could not be read, so the example is shown.';
      }
    }
    // The page was rendered in each tool's own units; show the user's settings.
    if (!hash || hash === 'example') {
      if (toolOptions()) run(true);
    }
  });
</script>

<div class="tool-grid">
<section class="card answer" aria-live="polite" class:stale aria-label="Answer">
  {#if stale && elapsedMs !== null}<p class="status">Calculating for {(elapsedMs / 1000).toFixed(1)} s…</p>{/if}
  {#if from}
    <p class="came-from">From <a href={from.href}>{from.title}</a></p>
  {/if}
  {#if result?.ok}
    <div class="value" bind:this={answerCard}>{answerParts[0]}{#if answerParts[1]}<span class="unit"> {answerParts[1]}</span>{/if}</div>
    <p class="sentence">{result.summary}</p>
    {#if inlineDg}<div class="inline-diagram" aria-hidden="true">{@html inlineDg.markup}</div>{/if}
    {#if result.comparison}<p class="comparison">{result.comparison}</p>{/if}
    {#each statuses as st}
      <p class="status"><span class="mark" aria-hidden="true">{st.mark}</span> <strong>{st.phrase}</strong> <span class="against">Against {#if st.source.url}<a href={st.source.url}>{st.source.label}</a>{:else}{st.source.label}{/if}.</span></p>
    {/each}
    {#if warnings.length}
      <ul class="warnings">
        {#each warnings as w}
          <li class={severityOf(w.code)}>{#if severityOf(w.code) === 'caution'}<span aria-hidden="true">⚠ </span><span class="sr-only">Caution: </span>{/if}{w.message}</li>
        {/each}
      </ul>
    {/if}
    {#each tables as t}
      <div class="table-scroll rows">
        <table>
        <caption>{tool.outputs.properties[t.key]?.title ?? t.key}</caption>
        <thead><tr>{#each t.columns as c}<th scope="col">{columnTitle(t.key, c)}</th>{/each}</tr></thead>
        <tbody>
          {#each t.rows as row}<tr>{#each t.columns as c}<td>{cellText(row[c], tool.outputs.properties[t.key]?.items?.properties?.[c])}</td>{/each}</tr>{/each}
        </tbody>
        </table>
      </div>
    {/each}
    {#if secondary.length}
      <dl class="facts">
        {#each facts as [k, v]}
          <div>
            <dt>{tool.outputs.properties[k]?.title ?? k}</dt>
            <dd class:words={!/\d/.test(v)}>
              <button type="button" class="copy-row" title="Copy this value" onclick={() => copyRow(k, v)}>{v}{#if copiedRow === k}<span class="copied" role="status"> Copied ✓</span>{/if}</button>
              <button type="button" class="send-open" aria-expanded={sendOpen === k} aria-label={`Send ${tool.outputs.properties[k]?.title ?? k} to another tool`} title="Send this value to another tool" onclick={() => openSend(k, v)}>→</button>
            </dd>
            {#if sendOpen === k}
              <div class="send-to" role="group" aria-label="Send this value to">
                {#if sendLinks[k]?.length}
                  <ul>
                    {#each sendLinks[k] as target}
                      <li><a href={target.href}>{target.title} <span class="send-field">as {target.fieldTitle.toLowerCase()}</span></a></li>
                    {/each}
                  </ul>
                {:else}
                  <p class="help">{sendLinks[k] ? 'No other tool takes this kind of value.' : 'Looking…'}</p>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </dl>
      {#if secondary.length > FACTS}
        <button type="button" class="quiet more" onclick={() => (allFacts = !allFacts)}>{allFacts ? 'Show fewer' : `Show all ${secondary.length} results`}</button>
      {/if}
    {/if}
    <div class="actions">
      <button type="button" class="primary" onclick={() => copy('value')}>{copied === 'value' ? 'Copied ✓' : 'Copy'}</button>
      <button type="button" class="quiet" onclick={() => copy('reference')} title="The answer with its inputs, method, sources, versions, and the notice">{copied === 'reference' ? 'Copied ✓' : 'Copy with reference'}</button>
      <button type="button" class="quiet" onclick={share}>{copied === 'link' ? 'Link copied ✓' : 'Share'}</button>
      <button type="button" class="quiet" onclick={() => (exportOpen = !exportOpen)} aria-expanded={exportOpen}>Download</button>
      {#if hasUnits}
        <label class="units-switch"><span class="sr-only">Units</span>
          <select bind:value={unitProfile} onchange={() => setProfile(unitProfile)} title="Units for every tool (also in Settings)">
            {#each PROFILES as [id, label]}<option value={id}>{id ? label : 'Units: each tool’s own'}</option>{/each}
          </select>
        </label>
      {/if}
      {#if !embedded}<button type="button" class="quiet star" aria-pressed={pinned} aria-label={pinned ? 'Unpin tool' : 'Pin tool'} title={pinned ? 'Pinned to your home page' : 'Pin to your home page'} onclick={pin}>{pinned ? '★' : '☆'}</button>{/if}
    </div>
    {#if exportOpen}
      <div class="export-menu" role="group" aria-label="Download this result as">
        {#each exportable as format}
          <button type="button" class="quiet" onclick={() => download(format)}>{format.label}</button>
        {/each}
      </div>
    {/if}
  {:else if result}
    {@const plain = assetError}
    <p class="error">{plain?.message ?? result.error.message}</p>
    {#if plain?.hint ?? result.error.hint}<p class="hint">{plain?.hint ?? result.error.hint}</p>{/if}
    <div class="actions">
      {#if badField}<button type="button" class="quiet" onclick={goToField}>Go to {tool.inputs.properties[badField].title.toLowerCase()}</button>{/if}
      <button type="button" class="quiet" onclick={tryExample}>Try the example</button>
    </div>
  {/if}
  {#if embedded}
    <p class="report-line"><a class="open-tool" href={toolHref}>Open the full tool, with your values →</a></p>
  {:else}
  <p class="report-line">{#if linkNote}<span class="notice">{linkNote} </span>{/if}<span>Something look off?</span> <button type="button" class="report-button" onclick={openReport}>Report a problem</button></p>
  {/if}
</section>

{#if result?.ok && answerHidden}
  <button type="button" class="answer-bar" onclick={() => answerCard?.scrollIntoView({ behavior: 'smooth', block: 'start' })} aria-label={`Answer: ${answer}. Show the full answer.`}>
    <span class="answer-bar-value">{answerParts[0]}{#if answerParts[1]}<span class="unit"> {answerParts[1]}</span>{/if}</span>
    <span class="answer-bar-go" aria-hidden="true">↑</span>
  </button>
{/if}

<form class="card inputs" onsubmit={(e) => e.preventDefault()}>
  <div class="inputs-head">
    <h2>Your values</h2>
    {#if isExample}<span class="chip">Showing an example. Change anything.</span>{/if}
  </div>
  {#snippet field([name, schema])}
    <label class:invalid={badField === name}>
      <span class="label-text">{schema.title}{#if !required.has(name)}<span class="optional"> optional</span>{/if}</span>
      {#if schema.enum}
        <select id={`field-${name}`} aria-invalid={badField === name} aria-describedby={badField === name ? 'field-error' : undefined} bind:value={values[name]} onchange={edited}>
          {#if !required.has(name)}<option value="">—</option>{/if}
          {#each schema.enum as option}<option value={option}>{option}</option>{/each}
        </select>
      {:else if isList(schema)}
        <textarea
          id={`field-${name}`}
          aria-invalid={badField === name}
          aria-describedby={badField === name ? 'field-error' : undefined}
          bind:value={values[name]}
          oninput={edited}
          rows="6"
          autocomplete="off"
          autocorrect="off"
          spellcheck="false"
          ondragover={(e) => takesPoints(schema) && e.preventDefault()}
          ondrop={(e) => {
            if (!takesPoints(schema) || !e.dataTransfer?.files?.length) return;
            e.preventDefault();
            importInto(name, e.dataTransfer.files[0]);
          }}
        ></textarea>
      {:else}
        <span class="entry">
          <input
            id={`field-${name}`}
            type="text"
            inputmode={isNumeric(schema) ? 'decimal' : undefined}
            enterkeyhint={name === lastField ? 'done' : 'next'}
            aria-invalid={badField === name}
            aria-describedby={badField === name ? 'field-error' : undefined}
            bind:value={values[name]}
            oninput={edited}
            autocomplete="off"
            autocorrect="off"
            spellcheck="false"
          />
          {#if isSigned(name, schema)}
            <button type="button" class="sign" onclick={() => flipSign(name)} aria-label={`Change the sign of ${schema.title}`} title="Plus or minus">±</button>
          {/if}
        </span>
      {/if}
      {#if stepsOf(schema).length}
        <!-- Shown only in Field mode, where a gloved thumb beats a keypad. -->
        <span class="steps">
          {#each stepsOf(schema) as delta}
            <button type="button" onclick={() => stepField(name, delta)} aria-label={`${delta < 0 ? 'Lower' : 'Raise'} ${schema.title} by ${Math.abs(delta)}`}>{stepLabel(delta)}</button>
          {/each}
        </span>
      {/if}
      {#if badField === name}<span class="field-error" id="field-error">{result.error.message}</span>{/if}
      <span class="help">{readable(schema.description ?? '')}{isList(schema) ? ` · one per line: ${columns(schema).map((c) => schema.items.properties[c].title.toLowerCase()).join(', ')}` : ''}{schema['x-unit'] && schema['x-unit'] !== '1' ? ` · plain numbers mean ${friendly(schema['x-unit'])}` : ''}</span>
    </label>
    {#if takesPoints(schema)}
      <div class="import-block">
          <span class="import-row">
            <label class="import-button">
              <input type="file" class="sr-only" accept=".geojson,.json,.kml,.kmz,.gpx,.csv,.tsv,.wkt,.wkb,.txt" onchange={(e) => importInto(name, e.currentTarget.files?.[0])} />
              Import a file
            </label>
            <span class="import-hint">KML, KMZ, GPX, GeoJSON, CSV, WKT, or WKB — or drop it on the box. Read on this device.</span>
          </span>
          {#if importNote[name]}<span class="import-note" class:bad={!importNote[name].ok} role="status">{importNote[name].text}</span>{/if}
          {#if importNote[name]?.csv}
            <span class="csv-map">
              <label class="csv-pick csv-crs">Coordinates are
                <select value={importNote[name].csv.crs} onchange={(e) => pickCrs(name, e.currentTarget.value)}>
                  {#each CSV_CRS as c}<option value={c.id}>{c.label}</option>{/each}
                </select>
              </label>
              {#if importNote[name].csv.crs === 'utm'}
                <label class="csv-pick">Zone
                  <input type="text" inputmode="numeric" bind:value={importNote[name].csv.zone} placeholder="17" autocomplete="off" />
                </label>
                <label class="csv-pick">Hemisphere
                  <select bind:value={importNote[name].csv.hemisphere}><option value="N">North</option><option value="S">South</option></select>
                </label>
              {:else if importNote[name].csv.crs === 'spcs'}
                <label class="csv-pick">Zone
                  <input type="text" bind:value={importNote[name].csv.zone} placeholder="3702" autocomplete="off" />
                </label>
                <label class="csv-pick">Unit
                  <select bind:value={importNote[name].csv.unit}>{#each SPCS_UNITS as [id, label]}<option value={id}>{label}</option>{/each}</select>
                </label>
              {/if}
              <label class="csv-pick">{importNote[name].csv.crs === 'wgs84' ? 'Latitude' : 'Northing'}
                <select bind:value={importNote[name].csv.lat}>
                  {#each importNote[name].csv.parsed.headers as h, i}<option value={i}>{h || `column ${i + 1}`}</option>{/each}
                </select>
              </label>
              <label class="csv-pick">{importNote[name].csv.crs === 'wgs84' ? 'Longitude' : 'Easting'}
                <select bind:value={importNote[name].csv.lon}>
                  {#each importNote[name].csv.parsed.headers as h, i}<option value={i}>{h || `column ${i + 1}`}</option>{/each}
                </select>
              </label>
              <button type="button" class="quiet" onclick={() => { const c = importNote[name].csv; [c.lat, c.lon] = [c.lon, c.lat]; importNote = { ...importNote }; }}>Swap</button>
              <button type="button" class="primary" onclick={() => useColumns(name)}>Use these columns</button>
            </span>
          {/if}
      </div>
    {/if}
  {/snippet}
  {#if pair}
    <div class="coordinate-field">
      <label>
        <span class="label-text">Paste a coordinate<span class="optional"> any notation</span></span>
        <input
          type="text"
          inputmode="text"
          autocomplete="off"
          autocorrect="off"
          spellcheck="false"
          placeholder="40°26'46&quot;N 79°58'56&quot;W, 9q8yyk8yuv, 849VCWC8+R9"
          bind:value={pasted}
          oninput={readPasted}
        />
      </label>
      {#if read}
        <p class="read" role="status" class:bad={!read.ok}>
          {read.message}
          {#if read.ambiguous}
            <button type="button" class="quiet" onclick={swapRead}>Swap to {degrees(read.ambiguous.lat)}°, {degrees(read.ambiguous.lon)}°</button>
          {/if}
        </p>
      {/if}
    </div>
  {/if}
  <div class="fields">
    {#each coreFields as f}{@render field(f)}{/each}
  </div>
  {#if moreFields.length}
    <details class="more-options" open={moreFields.some(([k]) => values[k] !== '' || k === badField)}>
      <summary>More options <span class="count">{moreFields.length}</span></summary>
      <div class="fields">
        {#each moreFields as f}{@render field(f)}{/each}
      </div>
    </details>
  {/if}
  <div class="actions">
    <button type="button" class="quiet" onclick={tryExample}>Try the example</button>
    <button type="button" class="quiet" onclick={clearAll}>Clear</button>
  </div>
  {#if !embedded && batchable}
    <details class="batch" ontoggle={(e) => e.currentTarget.open && loadBatch()}>
      <summary>Run many at once from a CSV</summary>
      {#if BatchPanel && compute}<BatchPanel {tool} {compute} />{:else}<p class="help">Loading…</p>{/if}
    </details>
  {/if}
</form>
</div>

{#if dg}
  <figure class="diagram card">
    {@html dg.markup}
    <div class="diagram-actions">
      <button type="button" class="quiet" onclick={exportSvg}>Download SVG</button>
    </div>
    {#if tl && sceneEnd > 0}
      <div class="timeline" role="group" aria-label="Scene playback">
        <button type="button" aria-pressed={playing} onclick={togglePlay}>{playing ? 'Pause' : 'Play'}</button>
        <input
          type="range"
          min="0"
          max={sceneEnd}
          step={sceneEnd / 200}
          value={playhead}
          aria-label="Scene time"
          aria-valuetext={`${Math.round(playhead)} seconds of ${Math.round(sceneEnd)}`}
          oninput={(e) => {
            playing = false;
            setPlayhead(Number(e.currentTarget.value));
          }}
        />
        <select aria-label="Speed" bind:value={speed}>
          <option value={0.5}>0.5×</option>
          <option value={1}>1×</option>
          <option value={2}>2×</option>
          <option value={4}>4×</option>
        </select>
        <label><input type="checkbox" bind:checked={loop} /> Loop</label>
      </div>
    {/if}
  </figure>
{/if}

{#if showMap && compute && result?.ok}
  <MapCanvas {tool} args={drawnArgs} {result} {compute} onmove={moveFromMap} />
{/if}
{#if reporting && ReportDialog}
  <ReportDialog {tool} args={args()} {result} onclose={() => (reporting = false)} />
{/if}
