<script>
  // Equipment profiles on a tool that takes them (web/equipment-profiles):
  // save this tool's equipment inputs of one kind under a name, load them
  // back, and move a profile between browsers as a JSON file. Everything
  // stays in this browser.
  import { blank, examplesOf, fieldsFor, fileName, fromFile, KINDS, MAX_BYTES, MAX_NAME, MAX_PER_KIND, ofKind, put, remove, saveTool, toFile, valuesFor } from '../lib/profiles.mjs';
  import { saveBlob } from '../lib/canvas-export.mjs';

  let { tool, kind, getInputs, setInputs, onLoaded = () => {} } = $props();

  /** What a name of this kind looks like, for the empty field. */
  const PLACEHOLDER = {
    aircraft: 'Like N12345 C172S',
    drone: 'Like Mavic 3E',
    camera: 'Like Zenmuse P1',
    lidar: 'Like Zenmuse L2',
    'gnss-receiver': 'Like rover R12i',
    radio: 'Like 2.4 GHz control link',
  };
  const noun = KINDS[kind].title.toLowerCase();

  // Saved profiles, with the shipped examples (read-only) beneath any of the same name.
  const listed = () => ({ ...examplesOf(kind), ...ofKind(kind) });
  let profiles = $state(listed());
  // The reader's own profiles come first; an example only when there are none.
  let chosen = $state(Object.keys(ofKind(kind))[0] ?? Object.keys(profiles)[0] ?? '');
  let name = $state(chosen);
  let status = $state('');
  const names = $derived(Object.keys(profiles).sort((a, b) => a.localeCompare(b)));
  const kept = fieldsFor(kind, tool.id).map((f) => tool.inputs.properties[f]?.title ?? f).join(', ');

  function refresh(select) {
    profiles = listed();
    chosen = select;
    name = select;
  }

  function save() {
    const n = name.trim();
    if (n === '') return (status = 'Give the profile a name first.');
    if (profiles[n]?.example) return (status = `${n} is an example profile and cannot be changed; save under another name.`);
    const next = { ...saveTool(profiles[n] ?? blank(kind, n), tool.id, getInputs()), savedOn: new Date().toISOString().slice(0, 10) };
    const how = put(next);
    if (how === 'full') return (status = `This browser already keeps ${MAX_PER_KIND} ${noun} profiles; delete one first.`);
    refresh(n);
    status = how === 'saved' ? `Saved this tool's ${noun} data to ${n}.` : `Saved to ${n} for this page only: this browser is not keeping site data.`;
  }

  function load() {
    const p = profiles[chosen];
    if (!p) return;
    const v = valuesFor(p, tool.id);
    if (Object.keys(v).length === 0) return (status = `${chosen} has nothing saved for this tool yet.`);
    setInputs(v);
    onLoaded(chosen, v);
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
    if (file.size > MAX_BYTES) return (status = 'This file is too large to be an equipment profile.');
    const r = fromFile(await file.text());
    if (!r.ok) return (status = r.message);
    if (r.profile.kind !== kind) return (status = `This is a ${KINDS[r.profile.kind].title.toLowerCase()} profile; open a tool that takes ${KINDS[r.profile.kind].title.toLowerCase()} data to import it.`);
    const replaced = Object.hasOwn(profiles, r.profile.name);
    if (put(r.profile) === 'full') return (status = `This browser already keeps ${MAX_PER_KIND} ${noun} profiles; delete one first.`);
    refresh(r.profile.name);
    const left = r.dropped.length ? ` Left out what a ${noun} profile does not keep: ${r.dropped.join(', ')}.` : '';
    status = `${replaced ? 'Replaced' : 'Added'} ${r.profile.name}. Choose Load to fill this tool from it.${left}`;
  }

  function drop() {
    if (profiles[chosen]?.example) return (status = `${chosen} is an example profile; it cannot be deleted.`);
    if (!chosen || !confirm(`Delete the profile ${chosen} from this browser?`)) return;
    const gone = chosen;
    remove(kind, gone);
    refresh(Object.keys(ofKind(kind))[0] ?? Object.keys(listed())[0] ?? '');
    status = `Deleted ${gone}.`;
  }
</script>

<div class="profiles">
  <p class="help">A {noun} profile keeps this tool's {noun} data ({kept}) in this browser. Nothing is uploaded; export a profile to move it to another device.</p>
  {#if names.length}
    <label>
      <span>Saved profiles</span>
      <select bind:value={chosen} onchange={() => (name = chosen)}>
        {#each names as n (n)}<option value={n}>{n}{profiles[n].example ? ' (Example)' : profiles[n].savedOn ? ` (saved ${profiles[n].savedOn})` : ''}</option>{/each}
      </select>
    </label>
    <div class="row">
      <button type="button" class="quiet" onclick={load}>Load</button>
      <button type="button" class="quiet" onclick={exportFile}>Export</button>
      {#if !profiles[chosen]?.example}<button type="button" class="quiet" onclick={drop}>Delete</button>{/if}
    </div>
  {/if}
  <label>
    <span>Profile name</span>
    <input type="text" bind:value={name} maxlength={MAX_NAME} placeholder={PLACEHOLDER[kind]} autocomplete="off" />
  </label>
  <div class="row">
    <button type="button" class="quiet" onclick={save}>Save this tool's {noun} data</button>
    <label class="import-button">
      <input type="file" class="sr-only" accept=".json,application/json" onchange={importFile} />
      Import a profile
    </label>
  </div>
  <p class="help" role="status" aria-live="polite">{status}</p>
</div>

<style>
  .profiles { display: grid; gap: 0.5rem; }
  .row { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  label:not(.import-button) { display: grid; gap: 0.25rem; }
</style>
