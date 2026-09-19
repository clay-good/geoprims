<script>
  // The interactive tool: schema-driven form, live answer card, permalinks.
  // Server-rendered with the worked example, so the answer is in the HTML.
  import { onMount } from 'svelte';
  import { isPinned, recordUse, togglePin, toolOptions } from '../lib/prefs.js';
  import MapCanvas from './MapCanvas.svelte';
  import { diagram } from '../lib/diagrams.js';

  let { tool, example, initial } = $props();

  const fields = Object.entries(tool.inputs.properties).filter(([k]) => k !== 'options');
  const required = new Set(tool.inputs.required);
  // Everyday inputs first; the rest (ellipsoid, rarely changed options) under "More options".
  const isCore = ([k, schema]) => schema['x-core'] || required.has(k);
  const coreFields = fields.filter(isCore);
  const moreFields = fields.filter((f) => !isCore(f));
  const outputOrder = Object.keys(tool.outputs.properties);

  // List inputs (traverse courses, polygon corners) edit as one row per line,
  // columns in schema order, separated by commas or tabs (a spreadsheet paste).
  const columns = (schema) => Object.keys(schema.items?.properties ?? {});
  const isList = (schema) => schema.type === 'array';
  const NUMBER = /^[-+]?[\d.]+(e[-+]?\d+)?$/i;
  function toText(k, v) {
    if (v === undefined) return '';
    const schema = tool.inputs.properties[k];
    if (!isList(schema) || !Array.isArray(v)) return String(v);
    return v.map((row) => columns(schema).map((c) => row[c] ?? '').join(', ')).join('\n');
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
  let copied = $state('');
  let linkNote = $state('');
  let compute = $state.raw(null);
  let timer;
  // The report dialog is imported on first click, so nothing loads before then.
  let ReportDialog = $state(null);
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
  const secondary = $derived(result?.ok ? Object.entries(result.display ?? {}).filter(([k]) => k !== primary) : []);
  const FACTS = 6;
  let allFacts = $state(false);
  const facts = $derived(allFacts ? secondary : secondary.slice(0, FACTS));

  // The canvas draws geographic tools: lines, points, polygons, or a lat/lon input.
  const GEO = new Set(['line-geodesic', 'line-rhumb', 'point', 'polygon']);
  const showMap = (tool.visualization ?? []).some((v) => GEO.has(v.kind)) || ('lat' in tool.inputs.properties && 'lon' in tool.inputs.properties);
  let drawnArgs = $state(example);
  const dg = $derived(result?.ok ? diagram(tool.id, drawnArgs, result) : null);

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
    const a = args();
    const out = await compute.invoke(tool.id, a);
    if (!out) return; // superseded by a newer edit
    result = out;
    drawnArgs = a;
    stale = false;
    if (settingsOnly) return;
    const enc = await compute.encodeLink({ i: a });
    if (enc?.ok) history.replaceState(null, '', `#${enc.result.fragment}`);
  }

  function edited() {
    isExample = false;
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
    history.replaceState(null, '', '#example');
    run();
  }

  async function copy(kind) {
    const text =
      kind === 'value'
        ? answer
        : kind === 'sentence'
          ? `${result.summary} (geoprims ${tool.id} ${tool.version})`
          : kind === 'link'
            ? location.href
            : JSON.stringify({ tool: 'geoprims_run', arguments: { id: tool.id, args: args() } });
    await navigator.clipboard.writeText(text);
    copied = kind;
    setTimeout(() => (copied = ''), 1500);
  }

  // On phones, a slim bar keeps the answer in view once the big number scrolls away.
  let answerCard = $state(null);
  let answerHidden = $state(false);
  function checkAnswer() {
    answerHidden = !!answerCard && answerCard.getBoundingClientRect().bottom < 0;
  }
  onMount(() => {
    addEventListener('scroll', checkAnswer, { passive: true });
    return () => removeEventListener('scroll', checkAnswer);
  });

  let pinned = $state(false);
  function pin() {
    togglePin(tool);
    pinned = isPinned(tool.id);
  }

  onMount(async () => {
    pinned = isPinned(tool.id);
    recordUse(tool);
    addEventListener('gp-prefs', () => compute && run(true));
    if (typeof WebAssembly !== 'object') {
      const { NO_WASM } = await import('../lib/messages.js');
      result = { ok: false, error: { code: 'UNSUPPORTED', message: NO_WASM } };
    }
    compute = await import('../lib/compute.js');
    const hash = location.hash.slice(1);
    if (hash && hash !== 'example') {
      const d = await compute.decodeLink(hash);
      if (d.ok && d.result.kind === 'state') {
        for (const [k] of fields) values[k] = toText(k, d.result.state.i?.[k]);
        isExample = false;
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
  {#if result?.ok}
    <div class="value" bind:this={answerCard}>{answerParts[0]}{#if answerParts[1]}<span class="unit"> {answerParts[1]}</span>{/if}</div>
    <p class="sentence">{result.summary}</p>
    {#if warnings.length}
      <ul class="warnings">
        {#each warnings as w}
          <li class={severityOf(w.code)}>{#if severityOf(w.code) === 'caution'}<span aria-hidden="true">⚠ </span><span class="sr-only">Caution: </span>{/if}{w.message}</li>
        {/each}
      </ul>
    {/if}
    {#if secondary.length}
      <dl class="facts">
        {#each facts as [k, v]}<div><dt>{tool.outputs.properties[k]?.title ?? k}</dt><dd class:words={!/\d/.test(v)}>{v}</dd></div>{/each}
      </dl>
      {#if secondary.length > FACTS}
        <button type="button" class="quiet more" onclick={() => (allFacts = !allFacts)}>{allFacts ? 'Show fewer' : `Show all ${secondary.length} results`}</button>
      {/if}
    {/if}
    <div class="actions">
      <button type="button" class="quiet" onclick={() => copy('value')}>{copied === 'value' ? 'Copied ✓' : 'Copy'}</button>
      <button type="button" class="quiet" onclick={() => copy('sentence')}>{copied === 'sentence' ? 'Copied ✓' : 'Copy sentence'}</button>
      <button type="button" class="quiet" onclick={() => copy('link')}>{copied === 'link' ? 'Link copied ✓' : 'Share link'}</button>
      <button type="button" class="quiet star" aria-pressed={pinned} aria-label={pinned ? 'Unpin tool' : 'Pin tool'} title={pinned ? 'Pinned to your home page' : 'Pin to your home page'} onclick={pin}>{pinned ? '★' : '☆'}</button>
    </div>
  {:else if result}
    <p class="error">{result.error.message}</p>
    {#if result.error.hint}<p class="hint">{result.error.hint}</p>{/if}
  {/if}
  <p class="report-line">{#if linkNote}<span class="notice">{linkNote} </span>{/if}Something look off? <button type="button" class="link" onclick={openReport}>Report a problem</button></p>
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
    <label>
      <span class="label-text">{schema.title}{#if !required.has(name)}<span class="optional"> optional</span>{/if}</span>
      {#if schema.enum}
        <select bind:value={values[name]} onchange={edited}>
          {#if !required.has(name)}<option value="">—</option>{/if}
          {#each schema.enum as option}<option value={option}>{option}</option>{/each}
        </select>
      {:else if isList(schema)}
        <textarea bind:value={values[name]} oninput={edited} rows="6" autocomplete="off" spellcheck="false"></textarea>
      {:else}
        <input bind:value={values[name]} oninput={edited} autocomplete="off" spellcheck="false" />
      {/if}
      <span class="help">{readable(schema.description ?? '')}{isList(schema) ? ` · one per line: ${columns(schema).map((c) => schema.items.properties[c].title.toLowerCase()).join(', ')}` : ''}{schema['x-unit'] && schema['x-unit'] !== '1' ? ` · plain numbers mean ${friendly(schema['x-unit'])}` : ''}</span>
    </label>
  {/snippet}
  <div class="fields">
    {#each coreFields as f}{@render field(f)}{/each}
  </div>
  {#if moreFields.length}
    <details class="more-options" open={moreFields.some(([k]) => values[k] !== '')}>
      <summary>More options <span class="count">{moreFields.length}</span></summary>
      <div class="fields">
        {#each moreFields as f}{@render field(f)}{/each}
      </div>
    </details>
  {/if}
  <div class="actions">
    <button type="button" class="quiet" onclick={tryExample}>Use the example</button>
    <button type="button" class="quiet" onclick={clearAll}>Clear</button>
  </div>
</form>
</div>

{#if dg}
  <figure class="diagram card">
    {@html dg.markup}
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
  <MapCanvas {tool} args={drawnArgs} {result} {compute} />
{/if}
{#if reporting && ReportDialog}
  <ReportDialog {tool} args={args()} {result} onclose={() => (reporting = false)} />
{/if}
