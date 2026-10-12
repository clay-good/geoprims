<script>
  // Local raster mode: files stay behind opaque worker handles. The shared
  // runtime streams Rust-produced windows into a bounded preview and summary.
  import { onDestroy } from 'svelte';
  import { previewHistogram, previewPixels, previewReadout, rasterPreview } from '../lib/raster-preview.mjs';

  let { tool, compute, getInputs } = $props();
  const roles = tool['x-raster'].bands.map((name) => ({ name, title: tool.inputs.properties[name].title }));

  let sources = $state([]);
  let assignments = $state({});
  let loading = $state(false);
  let running = $state(false);
  let problem = $state('');
  let completed = $state(0);
  let total = $state(0);
  let outcome = $state.raw(null);
  let cursor = $state('');
  let resampling = $state({ method: 'nearest', target: 'left' });
  let controller = null;

  const selected = (role) => sources[assignments[role]?.source];
  const ready = $derived(
    sources.length > 0 && roles.every(({ name }) => {
      const source = selected(name);
      const band = assignments[name]?.band;
      return source && Number.isInteger(band) && band >= 0 && band < source.metadata.bands;
    }),
  );
  const canResample = $derived(
    roles.length === 2
      && outcome?.error?.code === 'GRID_MISMATCH'
      && outcome.error.details?.reason !== 'CRS'
      && outcome.error.details?.left?.transform
      && outcome.error.details?.right?.transform,
  );

  async function remove(handles = sources) {
    await Promise.all(handles.map((source) => compute.removeFile(tool.id, source.handle).catch(() => {})));
  }

  async function choose(list) {
    const files = [...(list ?? [])];
    if (!files.length) return;
    controller?.abort();
    problem = '';
    outcome = null;
    cursor = '';
    if (files.length > roles.length) {
      problem = `Choose 1 multiband file or at most ${roles.length} separate band files.`;
      return;
    }
    loading = true;
    await remove();
    sources = [];
    const opened = [];
    try {
      for (const file of files) {
        const handle = `raster-${crypto.randomUUID()}`;
        const supplied = await compute.openFile(tool.id, handle, file);
        if (!supplied?.ok) throw new Error(supplied?.error?.message ?? 'The file could not be opened.');
        const inspected = await compute.inspectFile(tool.id, handle);
        if (!inspected?.ok) throw new Error(inspected?.error?.message ?? 'The file is not a supported GeoTIFF.');
        opened.push({ file, handle, metadata: inspected.metadata });
      }
      sources = opened;
      assignments = Object.fromEntries(roles.map(({ name }, i) => {
        const source = files.length === 1 ? 0 : Math.min(i, files.length - 1);
        const band = files.length === 1 ? Math.min(i, opened[0].metadata.bands - 1) : 0;
        return [name, { source, band }];
      }));
    } catch (error) {
      await remove(opened);
      problem = error.message;
    } finally {
      loading = false;
    }
  }

  function setSource(role, value) {
    assignments[role] = { source: Number(value), band: 0 };
    outcome = null;
  }

  function setBand(role, value) {
    assignments[role] = { ...assignments[role], band: Number(value) };
    outcome = null;
  }

  async function start(resample = null) {
    if (!ready) return;
    const bandSources = roles.map((role) => ({
      handle: selected(role.name).handle,
      band: assignments[role.name].band,
    }));
    const target = resample?.target === 'right' ? selected(roles[1].name) : selected(roles[0].name);
    const preview = rasterPreview(target.metadata.width, target.metadata.height);
    controller = new AbortController();
    running = true;
    completed = 0;
    total = 0;
    problem = '';
    outcome = null;
    try {
      const out = await compute.processFileScene({
        tool: tool.id,
        sources: bandSources,
        windowWidth: 512,
        windowHeight: 256,
        maxSamples: 4 * 1024 * 1024,
        parameters: getInputs(),
        resampling: resample,
        signal: controller.signal,
        accept: (window) => preview.add(window),
        onProgress: (progress) => {
          completed = progress.completed;
          total = progress.total;
        },
      });
      if (out) {
        const sampled = preview.value();
        outcome = out.ok ? {
          ...out,
          resampling: resample,
          preview: sampled,
          histogram: previewHistogram(sampled, out.summary.min, out.summary.max),
        } : out;
      }
    } catch (error) {
      problem = error.message;
    } finally {
      running = false;
    }
  }

  function paint(node, value) {
    const draw = ({ preview, min, max }) => {
      node.width = preview.width;
      node.height = preview.height;
      node.getContext('2d').putImageData(new ImageData(previewPixels(preview, min, max), preview.width, preview.height), 0, 0);
    };
    draw(value);
    return { update: draw };
  }

  const number = (value) => value?.toLocaleString('en-US', { maximumSignificantDigits: 7 }) ?? 'no valid pixels';
  const grid = (value) => `${value.width.toLocaleString('en-US')} × ${value.height.toLocaleString('en-US')} px`;

  function readPixel(event) {
    if (!outcome?.ok) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const x = Math.min(outcome.preview.width - 1, Math.floor(((event.clientX - rect.left) / rect.width) * outcome.preview.width));
    const y = Math.min(outcome.preview.height - 1, Math.floor(((event.clientY - rect.top) / rect.height) * outcome.preview.height));
    const read = previewReadout(outcome.preview, outcome.metadata, x, y);
    const where = read.x === undefined
      ? `column ${read.column + 1}, row ${read.row + 1}`
      : `${number(read.x)}, ${number(read.y)}${read.epsg ? ` (EPSG:${read.epsg})` : ''}`;
    cursor = `${read.value === null ? 'No-data' : number(read.value)} at ${where}`;
  }

  onDestroy(() => {
    controller?.abort();
    remove();
  });
</script>

<div class="raster-panel">
  <p class="help">Choose 1 multiband GeoTIFF or 1 file per band. Processing stays on this device; file names, bytes, and private handles are not put in links or reports.</p>
  <label class="import-button">
    <input type="file" class="sr-only" accept=".tif,.tiff,image/tiff" multiple onchange={(event) => choose(event.currentTarget.files)} />
    Choose GeoTIFF files
  </label>
  {#if loading}<p class="import-note" role="status">Reading raster metadata…</p>{/if}
  {#if problem}<p class="import-note bad" role="status">{problem}</p>{/if}

  {#if sources.length}
    <ul class="raster-files">
      {#each sources as source}
        <li><strong>{source.file.name}</strong>: {source.metadata.width.toLocaleString('en-US')} × {source.metadata.height.toLocaleString('en-US')} px, {source.metadata.bands} {source.metadata.bands === 1 ? 'band' : 'bands'}{source.metadata.epsg ? `, EPSG:${source.metadata.epsg}` : ''}</li>
      {/each}
    </ul>
    <div class="table-scroll" tabindex="0" role="region" aria-label="Raster band mapping">
      <table class="raster-map">
        <thead><tr><th scope="col">Input band</th><th scope="col">File</th><th scope="col">Band number</th></tr></thead>
        <tbody>
          {#each roles as role}
            <tr>
              <th scope="row">{role.title}</th>
              <td>
                <select aria-label={`File for ${role.title}`} value={assignments[role.name]?.source} onchange={(event) => setSource(role.name, event.currentTarget.value)}>
                  {#each sources as source, i}<option value={i}>{source.file.name}</option>{/each}
                </select>
              </td>
              <td>
                <select aria-label={`Band number for ${role.title}`} value={assignments[role.name]?.band} onchange={(event) => setBand(role.name, event.currentTarget.value)}>
                  {#each Array(selected(role.name)?.metadata.bands ?? 0) as _, i}<option value={i}>{i + 1}</option>{/each}
                </select>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="actions">
      {#if running}
        <button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>
      {:else}
        <button type="button" class="primary" disabled={!ready} onclick={() => start()}>Process raster</button>
      {/if}
    </div>
    {#if running || total}
      <progress class="batch-progress" max={total || 1} value={completed} aria-label="Raster windows done">{completed} of {total}</progress>
    {/if}
  {/if}

  {#if outcome && !outcome.ok}
    <p class="import-note bad" role="status">{outcome.error.message}</p>
    {#if outcome.error.code === 'GRID_MISMATCH'}
      {#if roles.length === 2}
        <dl class="raster-grids">
          <div><dt>{roles[0].title}</dt><dd>{grid(outcome.error.details.left)}</dd></div>
          <div><dt>{roles[1].title}</dt><dd>{grid(outcome.error.details.right)}</dd></div>
        </dl>
      {/if}
      {#if canResample}
        <fieldset class="raster-resample">
          <legend>Align the grids locally</legend>
          <label>Method
            <select value={resampling.method} onchange={(event) => (resampling.method = event.currentTarget.value)}>
              <option value="nearest">Nearest neighbor</option>
              <option value="bilinear">Bilinear</option>
            </select>
          </label>
          <label>Output grid
            <select value={resampling.target} onchange={(event) => (resampling.target = event.currentTarget.value)}>
              <option value="left">{selected(roles[0].name).file.name} — {grid(outcome.error.details.left)}</option>
              <option value="right">{selected(roles[1].name).file.name} — {grid(outcome.error.details.right)}</option>
            </select>
          </label>
          <button type="button" class="primary" onclick={() => start({ ...resampling })}>Resample and process</button>
        </fieldset>
      {:else}
        <p class="help">{roles.length > 2 ? 'All selected grids must match for this index.' : 'These grids cannot be aligned locally because their coordinate systems differ or georeferencing is missing.'}</p>
      {/if}
    {/if}
  {:else if outcome?.ok}
    {#if outcome.resampling}<p class="import-note" role="status">Processed with {outcome.resampling.method === 'nearest' ? 'nearest-neighbor' : 'bilinear'} resampling on the {outcome.resampling.target === 'left' ? roles[0].title : roles[1].title} grid.</p>{/if}
    <figure class="raster-preview">
      <canvas
        use:paint={{ preview: outcome.preview, min: outcome.summary.min, max: outcome.summary.max }}
        onpointermove={readPixel}
        onpointerleave={() => (cursor = '')}
        role="img"
        aria-label={`${tool['x-raster'].output} preview from ${number(outcome.summary.min)} to ${number(outcome.summary.max)}`}
      ></canvas>
      <figcaption>
        <span>{number(outcome.summary.min)}</span>
        <span>{number(outcome.summary.min === null ? null : outcome.summary.min < 0 && outcome.summary.max > 0 ? 0 : (outcome.summary.min + outcome.summary.max) / 2)}</span>
        <span>{number(outcome.summary.max)}</span>
      </figcaption>
    </figure>
    <figure class="raster-histogram">
      <figcaption>Distribution of {outcome.histogram.sampledPixels.toLocaleString('en-US')} preview pixels</figcaption>
      <div class="histogram-bars" role="img" aria-label={`Histogram from ${number(outcome.summary.min)} to ${number(outcome.summary.max)} in ${outcome.histogram.counts.length} equal-width bins`}>
        {#each outcome.histogram.counts as count}<span style={`--bar: ${outcome.histogram.maxCount ? count / outcome.histogram.maxCount : 0}`} aria-hidden="true"></span>{/each}
      </div>
      <p class="sr-only">Preview histogram values</p>
      <dl class="sr-only">
        {#each outcome.histogram.counts as count, i}
          <dt>{number(outcome.histogram.edges[i])} to {number(outcome.histogram.edges[i + 1])}</dt><dd>{count.toLocaleString('en-US')} sampled pixels</dd>
        {/each}
      </dl>
    </figure>
    <dl class="raster-summary">
      <div><dt>Mean</dt><dd>{number(outcome.summary.mean)}</dd></div>
      <div><dt>Valid pixels</dt><dd>{outcome.summary.validPixels.toLocaleString('en-US')}</dd></div>
      <div><dt>No-data pixels</dt><dd>{outcome.summary.noDataPixels.toLocaleString('en-US')}</dd></div>
    </dl>
    <p class="cursor-readout" aria-live="polite">{cursor || 'Point at the preview to read a pixel value and coordinate.'}</p>
    <p class="help">The preview and histogram sample at most 512 × 512 pixels. Statistics use every valid output pixel. GeoTIFF download is not connected yet.</p>
  {/if}
</div>
