<script>
  import { deedPlotState, EDITABLE_CALL_FIELDS, reviewRows, sourceParts } from '../lib/deed-confirm.mjs';

  let { text = '', rows = [], schema = {} } = $props();
  let sourceRows = $state(rows);
  let editable = $state(reviewRows(rows));
  let selected = $state(0);
  let status = $state('');
  const fields = schema.items?.properties ?? {};
  const parts = $derived(sourceParts(text, editable));

  $effect(() => {
    if (rows === sourceRows) return;
    sourceRows = rows;
    editable = reviewRows(rows);
    selected = 0;
    status = '';
  });

  function edit(row, field, value) {
    editable[row] = { ...editable[row], [field]: value };
    editable = [...editable];
    status = '';
  }

  async function confirm() {
    const state = deedPlotState(editable);
    if (state.i.calls.length < 2) {
      status = 'Enter at least two plottable calls before continuing.';
      return;
    }
    status = 'Preparing the confirmed plot…';
    const compute = await import('../lib/compute.js');
    const encoded = await compute.encodeLink(state);
    if (!encoded?.ok) {
      status = 'The confirmed calls could not be put in a link.';
      return;
    }
    location.assign(`/survey/land/deed-plot/#${encoded.result.fragment}`);
  }
</script>

<div class="deed-confirm" role="group" aria-labelledby="deed-confirm-title">
  <h2 id="deed-confirm-title">Confirm the parsed calls</h2>
  <p class="help">Select a source phrase to find its row. Edit any reading that does not match the deed. Calls without geometry stay out of the plot, leaving the figure open.</p>
  <div class="deed-source" aria-label="Original deed with one highlighted phrase per parsed call">
    {#each parts as part}
      {#if part.call === null}
        {part.text}
      {:else}
        <mark class:active={selected === part.call} role="button" tabindex="0" onclick={() => (selected = part.call)} onkeydown={(event) => (event.key === 'Enter' || event.key === ' ') && (selected = part.call)}>{part.text}</mark>
      {/if}
    {/each}
  </div>
  <div class="table-scroll rows deed-calls">
    <table>
      <caption>Parsed calls — editable until you confirm</caption>
      <thead><tr><th scope="col">Call</th>{#each EDITABLE_CALL_FIELDS as field}<th scope="col">{fields[field]?.title ?? field}</th>{/each}<th scope="col">Review note</th></tr></thead>
      <tbody>
        {#each editable as row, i}
          <tr class:selected={selected === i} onclick={() => (selected = i)}>
            <th scope="row">{i + 1}</th>
            {#each EDITABLE_CALL_FIELDS as field}
              <td><input type="text" value={row[field] ?? ''} aria-label={`Call ${i + 1} ${fields[field]?.title ?? field}`} oninput={(event) => edit(i, field, event.currentTarget.value)} /></td>
            {/each}
            <td>{row.flags || row.monument || 'Check against the highlighted text.'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="actions">
    <button type="button" class="primary" onclick={confirm}>Confirm calls and open deed plot</button>
    {#if status}<span class="help" role="status">{status}</span>{/if}
  </div>
</div>
