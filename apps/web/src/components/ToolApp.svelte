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

  let values = $state(Object.fromEntries(fields.map(([k]) => [k, toText(k, example[k])])));
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
    result?.ok ? [...result.meta.warnings].sort((a, b) => RANK[severityOf(a.code)] - RANK[severityOf(b.code)]) : [],
  );
  const secondary = $derived(result?.ok ? Object.entries(result.display ?? {}).filter(([k]) => k !== primary) : []);

  // The canvas draws geographic tools: lines, points, polygons, or a lat/lon input.
  const GEO = new Set(['line-geodesic', 'line-rhumb', 'point', 'polygon']);
  const showMap = (tool.visualization ?? []).some((v) => GEO.has(v.kind)) || ('lat' in tool.inputs.properties && 'lon' in tool.inputs.properties);
  let drawnArgs = $state(example);
  const dg = $derived(result?.ok ? diagram(tool.id, drawnArgs, result) : null);

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
    for (const [k] of fields) values[k] = toText(k, example[k]);
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
          : JSON.stringify({ tool: 'geoprims_run', arguments: { id: tool.id, args: args() } });
    await navigator.clipboard.writeText(text);
    copied = kind;
    setTimeout(() => (copied = ''), 1500);
  }

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

<section class="card answer sticky" aria-live="polite" class:stale>
  {#if result?.ok}
    {#if warnings.length}
      <ul class="warnings">
        {#each warnings as w}
          <li class={severityOf(w.code)}>{#if severityOf(w.code) === 'caution'}<span aria-hidden="true">⚠ </span><span class="sr-only">Caution: </span>{/if}{w.message}</li>
        {/each}
      </ul>
    {/if}
    <div class="value">{answerParts[0]}{#if answerParts[1]}<span class="unit"> {answerParts[1]}</span>{/if}</div>
    <p class="sentence">{result.summary}</p>
    {#if secondary.length}
      <ul class="secondary">
        {#each secondary as [k, v]}<li>{tool.outputs.properties[k]?.title ?? k}: {v}</li>{/each}
      </ul>
    {/if}
    <div class="actions">
      <button type="button" onclick={() => copy('value')}>{copied === 'value' ? 'Copied' : 'Copy value'}</button>
      <button type="button" onclick={() => copy('sentence')}>{copied === 'sentence' ? 'Copied' : 'Copy sentence'}</button>
      <button type="button" onclick={() => copy('agent')}>{copied === 'agent' ? 'Copied' : 'Copy as agent call'}</button>
      <button type="button" aria-pressed={pinned} onclick={pin}>{pinned ? 'Pinned' : 'Pin tool'}</button>
    </div>
  {:else if result}
    <p class="error">{result.error.message}</p>
    {#if result.error.hint}<p>{result.error.hint}</p>{/if}
  {/if}
</section>

{#if dg}
  <figure class="diagram card">{@html dg.markup}</figure>
{/if}

{#if showMap && compute && result?.ok}
  <MapCanvas {tool} args={drawnArgs} {result} {compute} />
{/if}

{#if linkNote}<p class="notice">{linkNote}</p>{/if}

<p class="report-line"><button type="button" class="link" onclick={openReport}>Report a problem</button> with this result.</p>
{#if reporting && ReportDialog}
  <ReportDialog {tool} args={args()} {result} onclose={() => (reporting = false)} />
{/if}

<form class="card" onsubmit={(e) => e.preventDefault()}>
  {#if isExample}<p class="chip">Example values</p>{/if}
  <div class="fields">
    {#each fields as [name, schema]}
      <label>
        {schema.title}{required.has(name) ? '' : ' (optional)'}
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
        <span class="help">{schema.description}{isList(schema) ? ` · one per line: ${columns(schema).map((c) => schema.items.properties[c].title.toLowerCase()).join(', ')}` : ''}{schema['x-unit'] && schema['x-unit'] !== '1' ? ` · a bare number is in ${schema['x-unit']}` : ''}</span>
      </label>
    {/each}
  </div>
  <div class="actions">
    <button type="button" onclick={clearAll}>Clear</button>
    <button type="button" onclick={tryExample}>Try the example</button>
  </div>
</form>
