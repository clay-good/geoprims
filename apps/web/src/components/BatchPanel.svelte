<script>
  // Batch mode (web/io-formats, "Batch mode over CSV"): a CSV of cases run
  // through this tool in the compute worker, with progress, cancel, the
  // errors listed by row, and the results joined to the original columns.
  // Loaded only when a reader opens the panel.
  import { batchErrors, batchInputs, joinedCsv, MAX_ROWS, rowArgs, runBatch, suggestMapping } from '../lib/batch.mjs';
  import { checkSize } from '../lib/import.mjs';

  let { tool, compute } = $props();
  const inputs = batchInputs(tool);

  let file = $state(null);
  let parsed = $state(null);
  let mapping = $state({});
  let units = $state({});
  let problem = $state('');
  let running = $state(false);
  let done = $state(0);
  let total = $state(0);
  let outcome = $state(null);
  let controller = null;

  const missing = $derived(inputs.filter((i) => i.required && !(mapping[i.name] >= 0)).map((i) => i.title));

  async function choose(f) {
    outcome = null;
    problem = '';
    if (!f) return;
    const tooBig = checkSize(f.size);
    if (tooBig) return (problem = tooBig);
    const out = await compute.readFile(f.name, await f.text());
    if (!out.ok || !out.headers) return (problem = out.message ?? 'Batch mode reads a CSV or TSV with a header row.');
    if (out.rows.length > MAX_ROWS) return (problem = `That file has ${out.rows.length.toLocaleString('en-US')} rows; a batch holds at most ${MAX_ROWS.toLocaleString('en-US')}.`);
    file = f;
    parsed = out;
    mapping = suggestMapping(tool, out.headers);
    units = Object.fromEntries(inputs.filter((i) => i.unit).map((i) => [i.name, i.unit]));
  }

  async function start() {
    controller = new AbortController();
    running = true;
    done = 0;
    total = parsed.rows.length;
    outcome = null;
    try {
      const args = rowArgs(tool, parsed.rows, mapping, units);
      const { results, cancelled } = await runBatch(tool.id, args, {
        invokeBatch: compute.invokeBatch,
        signal: controller.signal,
        onProgress: (d) => (done = d),
      });
      outcome = { results, cancelled, errors: batchErrors(results) };
    } catch (e) {
      problem = e.message;
    } finally {
      running = false;
    }
  }

  function download() {
    const rows = parsed.rows.slice(0, outcome.results.length);
    const text = joinedCsv(tool, parsed.headers, rows, outcome.results);
    const url = URL.createObjectURL(new Blob([text], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${tool.id}.batch.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="batch-panel">
  <p class="help">One row per case, with a header row. Each column feeds an input; plain numbers take the unit you choose. Everything runs on this device.</p>
  <label class="import-button">
    <input type="file" class="sr-only" accept=".csv,.tsv,.txt" onchange={(e) => choose(e.currentTarget.files?.[0])} />
    Choose a CSV
  </label>
  {#if problem}<p class="import-note bad" role="status">{problem}</p>{/if}

  {#if parsed}
    <p class="import-note" role="status">{file.name}: {parsed.rows.length.toLocaleString('en-US')} rows, {parsed.headers.length} columns.</p>
    <table class="batch-map">
      <thead><tr><th scope="col">Input</th><th scope="col">Column</th><th scope="col">Unit for plain numbers</th></tr></thead>
      <tbody>
        {#each inputs as input}
          <tr>
            <th scope="row">{input.title}{#if input.required}<span class="optional"> required</span>{/if}</th>
            <td>
              <select bind:value={mapping[input.name]} aria-label={`Column for ${input.title}`}>
                <option value={-1}>—</option>
                {#each parsed.headers as h, i}<option value={i}>{h || `column ${i + 1}`}</option>{/each}
              </select>
            </td>
            <td>{#if input.unit}<input type="text" class="batch-unit" bind:value={units[input.name]} aria-label={`Unit for ${input.title}`} autocomplete="off" autocorrect="off" spellcheck="false" />{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if missing.length}<p class="import-note bad">Choose a column for {missing.join(', ')}.</p>{/if}
    <div class="actions">
      {#if running}
        <button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>
      {:else}
        <button type="button" class="primary" disabled={missing.length > 0} onclick={start}>Run {parsed.rows.length.toLocaleString('en-US')} rows</button>
      {/if}
    </div>
    {#if running || outcome}
      <progress class="batch-progress" max={total} value={done} aria-label="Rows done">{done} of {total}</progress>
    {/if}
    {#if outcome}
      <p class="import-note" class:bad={outcome.errors.length > 0} role="status">
        {outcome.cancelled ? 'Cancelled after' : 'Done:'} {outcome.results.length.toLocaleString('en-US')} rows, {(outcome.results.length - outcome.errors.length).toLocaleString('en-US')} worked{outcome.errors.length ? `, ${outcome.errors.length.toLocaleString('en-US')} did not` : ''}.
      </p>
      {#if outcome.errors.length}
        <ul class="batch-errors">
          {#each outcome.errors.slice(0, 20) as e}<li>Line {e.line}: {e.message}</li>{/each}
          {#if outcome.errors.length > 20}<li>… and {outcome.errors.length - 20} more, all in the download.</li>{/if}
        </ul>
      {/if}
      <div class="actions"><button type="button" class="quiet" onclick={download}>Download results (CSV)</button></div>
    {/if}
  {/if}
</div>
