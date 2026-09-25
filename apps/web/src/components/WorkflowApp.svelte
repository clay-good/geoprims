<script>
  // A workflow (add-job-workflows, web/workflows): the few inputs a job needs,
  // every step run in the core by the shared chain runner, each step's answer
  // with a link to its full tool, and one visual for the plan. Server-rendered
  // with the worked example, so every answer is in the HTML before any script.
  import { onMount } from 'svelte';
  import { runChain } from '../../../../packages/runtime/src/chain.mjs';
  import MapCanvas from './MapCanvas.svelte';
  import { mapsTool } from '../lib/map/layers.js';
  import { diagram } from '../lib/diagrams.js';
  import { isNumeric } from '../lib/fields.mjs';
  import { planText } from '../lib/plan.mjs';

  // `workflow` is the data file's entry; `guide` its build-time run (steps with
  // results and links); `tools` the slice of each step's tool the page needs.
  let { workflow, guide, tools } = $props();

  const schemaOf = (input) => tools[input.tool].inputs.properties[input.field];
  const isList = (schema) => schema.type === 'array';
  const columns = (schema) => Object.keys(schema.items?.properties ?? {});
  const NUMBER = /^[-+]?[\d.]+(e[-+]?\d+)?$/i;
  const FRIENDLY = { degC: '°C', degF: '°F', deg: 'degrees', m2: 'm²', km2: 'km²' };

  // List inputs edit as one row per line, columns in the tool's order, as on a tool page.
  function toText(input, v) {
    const schema = schemaOf(input);
    if (!isList(schema) || !Array.isArray(v)) return v === undefined || v === null ? '' : String(v);
    return v.map((row) => columns(schema).map((c) => row[c] ?? '').join(', ').replace(/(,\s*)+$/, '')).join('\n');
  }
  function fromText(input, text) {
    const schema = schemaOf(input);
    if (!isList(schema)) return NUMBER.test(text.trim()) ? Number(text.trim()) : text.trim();
    return text
      .split('\n')
      .filter((line) => line.trim() !== '')
      .map((line) => {
        const cells = line.split(line.includes('\t') ? '\t' : ',').map((c) => c.trim());
        return Object.fromEntries(columns(schema).map((c, i) => [c, cells[i]]).filter(([, v]) => v !== undefined && v !== '').map(([c, v]) => [c, NUMBER.test(v) ? Number(v) : v]));
      });
  }

  const shown = workflow.inputs.filter((i) => !i.advanced);
  const more = workflow.inputs.filter((i) => i.advanced);
  const exampleText = () => Object.fromEntries(workflow.inputs.map((i) => [i.name, toText(i, i.example)]));
  let values = $state(exampleText());
  let steps = $state(guide.steps);
  let isExample = $state(true);
  let stale = $state(false);
  let compute = $state.raw(null);
  let timer;
  let runId = 0;

  const failed = $derived(steps.findIndex((s) => s.status === 'failed'));
  const done = $derived(steps.filter((s) => s.status === 'ok').length);
  const visual = $derived(steps[workflow.visual.step]);
  const visualTool = tools[workflow.steps[workflow.visual.step].tool];
  const dg = $derived(visual?.status === 'ok' ? diagram(visualTool.id, visual.input, visual.result) : null);
  const onMap = mapsTool(visualTool);

  /** The step a failure belongs to, named for the reader. */
  const stepName = (i) => `step ${i + 1} (${tools[steps[i].tool].title})`;
  // The tool field an error names, when it is one of this workflow's inputs.
  const badInput = $derived.by(() => {
    if (failed < 0) return null;
    const field = steps[failed].result?.error?.field;
    const def = workflow.steps[failed].input?.[field];
    return def && typeof def === 'object' && 'input' in def ? def.input : null;
  });

  function edited() {
    isExample = false;
    stale = true;
    clearTimeout(timer);
    timer = setTimeout(run, 250);
  }

  async function run() {
    if (!compute) return;
    const id = ++runId;
    const given = Object.fromEntries(workflow.inputs.map((i) => [i.name, values[i.name] === '' ? '' : fromText(i, values[i.name])]));
    // Each step has its own request key, so a newer run supersedes an older one step by step.
    const invoke = (tool, input, i) => compute.invoke(tool, input, `wf-${i}`);
    let out;
    try {
      out = await runChain(workflow, given, invoke);
    } catch {
      return;
    }
    if (id !== runId || out.steps.some((s) => s.status !== 'waiting' && s.result === null)) return;
    // Each step's link opens its tool holding exactly the inputs it ran with.
    for (const [i, s] of out.steps.entries()) {
      const t = tools[s.tool];
      if (s.status === 'waiting') {
        s.href = t.route;
        continue;
      }
      const came = s.carried.length ? out.steps[s.carried[0].from].tool : undefined;
      const enc = await compute.encodeLink({ i: s.input, ...(came ? { c: came } : {}) });
      if (id !== runId) return;
      s.href = enc?.ok ? `${t.route}#${enc.result.fragment}` : t.route;
      s.title = t.title;
      out.steps[i] = s;
    }
    steps = out.steps;
    stale = false;
    // The permalink holds the inputs (in the fragment, which never leaves the device).
    const params = new URLSearchParams(Object.entries(values).filter(([k, v]) => v !== toText(workflow.inputs.find((i) => i.name === k), workflow.inputs.find((i) => i.name === k).example)));
    history.replaceState(null, '', params.size ? `#${params}` : location.pathname);
  }

  function reset() {
    values = exampleText();
    isExample = true;
    edited();
    isExample = true;
  }

  onMount(async () => {
    compute = await import('../lib/compute.js');
    const hash = location.hash.slice(1);
    if (hash && hash !== 'example') {
      const params = new URLSearchParams(hash);
      let any = false;
      for (const i of workflow.inputs) {
        if (params.has(i.name)) {
          values[i.name] = params.get(i.name);
          any = true;
        }
      }
      if (any) {
        isExample = false;
        run();
      }
    }
  });

  // Copy the plan: the inputs and every step's own sentence, with the link back.
  let copied = $state('');
  async function copyPlan() {
    const text = planText({
      title: guide.title ?? workflow.title,
      inputs: workflow.inputs.filter((i) => !i.advanced || values[i.name] !== toText(i, i.example)).map((i) => ({ title: schemaOf(i).title, value: values[i.name] })),
      steps: steps.map((s) => ({ ...s, title: tools[s.tool].title })),
      url: location.href,
      today: new Date().toISOString().slice(0, 10),
    });
    try {
      await navigator.clipboard.writeText(text);
      copied = 'Copied the plan.';
    } catch {
      copied = 'Copying is blocked here; select the steps and copy them instead.';
    }
    setTimeout(() => (copied = ''), 3000);
  }

  const shownValue = (v) => (Array.isArray(v) ? `${v.length} rows` : typeof v === 'object' && v !== null ? JSON.stringify(v) : String(v));
  const fieldTitle = (tool, name) => tools[tool].inputs.properties[name]?.title ?? name.replaceAll('_', ' ');
</script>

<form class="card inputs workflow-inputs" onsubmit={(e) => e.preventDefault()}>
  <div class="inputs-head">
    <h2>Your values</h2>
    {#if isExample}<span class="chip">Showing an example. Change anything.</span>{:else}<button type="button" class="quiet" onclick={reset}>Back to the example</button>{/if}
  </div>
  {#snippet field(input)}
    {@const schema = schemaOf(input)}
    <label class:invalid={badInput === input.name}>
      <span class="label-text">{schema.title}</span>
      {#if schema.enum}
        <select id={`wf-${input.name}`} bind:value={values[input.name]} onchange={edited}>
          {#each schema.enum as option}<option value={option}>{option}</option>{/each}
        </select>
      {:else if isList(schema) || input.name === 'metar' || input.name === 'text'}
        <textarea id={`wf-${input.name}`} bind:value={values[input.name]} oninput={edited} rows={isList(schema) ? 5 : 3} autocomplete="off" autocorrect="off" spellcheck="false"></textarea>
      {:else}
        <span class="entry">
          <input id={`wf-${input.name}`} type="text" inputmode={isNumeric(schema) ? 'decimal' : undefined} bind:value={values[input.name]} oninput={edited} autocomplete="off" autocorrect="off" spellcheck="false" />
        </span>
      {/if}
      {#if badInput === input.name}<span class="field-error">{steps[failed].result.error.message}</span>{/if}
      <span class="help">{schema.description ?? ''}{isList(schema) ? ` · one per line: ${columns(schema).map((c) => schema.items.properties[c].title.toLowerCase()).join(', ')}` : ''}{schema['x-unit'] && schema['x-unit'] !== '1' ? ` · plain numbers mean ${FRIENDLY[schema['x-unit']] ?? schema['x-unit']}` : ''}</span>
    </label>
  {/snippet}
  <div class="fields">
    {#each shown as input}{@render field(input)}{/each}
  </div>
  {#if more.length}
    <details class="more">
      <summary>More options</summary>
      <div class="fields">
        {#each more as input}{@render field(input)}{/each}
      </div>
    </details>
  {/if}
</form>

<section class="card answer workflow-answer" aria-live="polite" class:stale aria-label="The plan">
  {#if failed >= 0}
    <p class="sentence"><strong>Stopped at {stepName(failed)}:</strong> {steps[failed].result?.error?.message ?? 'no result'}</p>
  {:else}
    <p class="status">All {done} steps worked out from your values.</p>
  {/if}
  <ol class="journey workflow-steps">
    {#each steps as s, i}
      <li class="journey-step" class:waiting={s.status === 'waiting'} class:failed={s.status === 'failed'}>
        <p class="step-num">Step {i + 1}</p>
        <h3><a href={s.href}>{tools[s.tool].title}</a></h3>
        {#if s.status === 'ok'}
          <p class="sentence">{s.result.summary}</p>
        {:else if s.status === 'failed'}
          <p class="sentence field-error">{s.result?.error?.message ?? 'No result.'}</p>
        {:else}
          <p class="help">Waiting for step {s.waitingOn + 1}.</p>
        {/if}
        <p class="help">{s.why}</p>
      </li>
    {/each}
  </ol>
  <div class="actions">
    <button type="button" onclick={copyPlan}>Copy the plan</button>
    <span role="status" class="help">{copied}</span>
  </div>
</section>

{#if visual?.status === 'ok'}
  <!-- The step's own drawing when it has one (a runway, a height stack); else its map. -->
  {#if dg}
    <figure class="diagram card">{@html dg.markup}</figure>
  {:else if onMap && compute}
    <MapCanvas tool={visualTool} args={visual.input} result={visual.result} {compute} />
  {/if}
{/if}

<details class="card assumptions">
  <summary>Assumptions ({guide.assumptions.length})</summary>
  <p class="help">Values this workflow fixes for you. Open the step's tool to change one.</p>
  <ul>
    {#each guide.assumptions as a}
      <li>Step {a.step + 1}, {tools[a.tool].title}: {fieldTitle(a.tool, a.name)} = {shownValue(a.value)} (<a href={steps[a.step].href}>change</a>)</li>
    {/each}
  </ul>
</details>
