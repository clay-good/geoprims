<script>
  // Aircraft profiles on the tools that take aircraft data: save this tool's
  // aircraft inputs under a name, load them back, and move a profile between
  // browsers as a JSON file. Everything stays in this browser.
  import { all, blank, fileName, fromFile, MAX_BYTES, put, remove, saveTool, toFile, valuesFor, PROFILE_FIELDS } from '../lib/aircraft.mjs';
  import { saveBlob } from '../lib/canvas-export.mjs';

  let { tool, getInputs, setInputs } = $props();

  let profiles = $state(all());
  let chosen = $state(Object.keys(profiles)[0] ?? '');
  let name = $state(chosen);
  let status = $state('');
  const names = $derived(Object.keys(profiles).sort((a, b) => a.localeCompare(b)));
  const kept = PROFILE_FIELDS[tool.id].map((f) => tool.inputs.properties[f]?.title ?? f).join(', ');

  function refresh(select) {
    profiles = all();
    chosen = select;
    name = select;
  }

  function save() {
    const n = name.trim();
    if (n === '') return (status = 'Give the profile a name first.');
    const next = saveTool(profiles[n] ?? blank(n), tool.id, getInputs());
    const lasting = put(next);
    refresh(n);
    status = lasting ? `Saved this tool's aircraft data to ${n}.` : `Saved to ${n} for this page only: this browser is not keeping site data.`;
  }

  function load() {
    const p = profiles[chosen];
    if (!p) return;
    const v = valuesFor(p, tool.id);
    if (Object.keys(v).length === 0) return (status = `${chosen} has nothing saved for this tool yet.`);
    setInputs(v);
    status = `Loaded ${chosen}.`;
  }

  function exportFile() {
    const p = profiles[chosen];
    if (!p) return;
    saveBlob(new Blob([toFile(p)], { type: 'application/json' }), fileName(p));
    status = `Downloaded ${fileName(p)}.`;
  }

  async function importFile(e) {
    const file = e.currentTarget.files?.[0];
    e.currentTarget.value = '';
    if (!file) return;
    if (file.size > MAX_BYTES) return (status = 'This file is too large to be an aircraft profile.');
    const r = fromFile(await file.text());
    if (!r.ok) return (status = r.message);
    const replaced = Object.hasOwn(profiles, r.profile.name);
    put(r.profile);
    refresh(r.profile.name);
    status = `${replaced ? 'Replaced' : 'Added'} ${r.profile.name}. Choose Load to fill this tool from it.`;
  }

  function drop() {
    if (!chosen || !confirm(`Delete the profile ${chosen} from this browser?`)) return;
    const gone = chosen;
    remove(gone);
    refresh(Object.keys(all())[0] ?? '');
    status = `Deleted ${gone}.`;
  }
</script>

<div class="aircraft">
  <p class="help">A profile keeps this tool's aircraft data ({kept}) in this browser. Nothing is uploaded; export a profile to move it to another device.</p>
  {#if names.length}
    <label>
      <span>Saved profiles</span>
      <select bind:value={chosen} onchange={() => (name = chosen)}>
        {#each names as n (n)}<option value={n}>{n}</option>{/each}
      </select>
    </label>
    <div class="row">
      <button type="button" class="quiet" onclick={load}>Load</button>
      <button type="button" class="quiet" onclick={exportFile}>Export</button>
      <button type="button" class="quiet" onclick={drop}>Delete</button>
    </div>
  {/if}
  <label>
    <span>Profile name</span>
    <input type="text" bind:value={name} maxlength="60" placeholder="Like N12345 C172S" autocomplete="off" />
  </label>
  <div class="row">
    <button type="button" class="quiet" onclick={save}>Save this tool's aircraft data</button>
    <label class="import-button">
      <input type="file" class="sr-only" accept=".json,application/json" onchange={importFile} />
      Import a profile
    </label>
  </div>
  <p class="help" role="status" aria-live="polite">{status}</p>
</div>

<style>
  .aircraft { display: grid; gap: 0.5rem; }
  .row { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  label:not(.import-button) { display: grid; gap: 0.25rem; }
</style>
