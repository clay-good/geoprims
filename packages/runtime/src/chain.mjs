// The chain runner (add-job-workflows, "Shared chain runner"): one workflow,
// its steps run in order through `invoke(id, input)`. The site build, the
// workflow page, and the MCP server all call this, so the three give the same
// step results for the same inputs. It holds no math: it only carries values
// from the reader's inputs and from earlier steps into later steps.
//
// A step input is a literal (fixed by the workflow: an assumption), or
//   {"input": "name"}             the workflow input of that name, or
//   {"from": [step, "output"]}    an earlier step's output, with "columns" to
//                                 carry only the list columns the next tool takes.

/** 12 significant digits: drops binary noise (72.96000000000001), far below any tool's precision. */
const tidy = (n) => (typeof n === 'number' && Number.isFinite(n) ? Number(n.toPrecision(12)) : n);

/** An output as a tool input: a quantity as "value unit"; anything else as it is. */
export function asInput(v) {
  if (v && typeof v === 'object' && !Array.isArray(v) && 'value' in v) return v.unit && v.unit !== '1' ? `${tidy(v.value)} ${v.unit}` : tidy(v.value);
  return tidy(v);
}

const isRef = (v, key) => v && typeof v === 'object' && !Array.isArray(v) && key in v;

/** The workflow inputs' example values: the prefill. */
export const examples = (workflow) => Object.fromEntries((workflow.inputs ?? []).map((i) => [i.name, i.example]));

/**
 * Runs a workflow with `values` (workflow input name → value; missing ones take
 * the example). Returns { ok, steps, failed }: each step with its tool, the
 * inputs it ran with, its result, and status "ok", "failed", or "waiting". The
 * chain stops at the first failed step; later steps wait, and nothing runs on
 * a guessed value. Wiring mistakes (a missing output, a forward reference)
 * throw, because they are bugs in the workflow, not in the reader's input.
 */
export async function runChain(workflow, values, invoke) {
  const given = { ...examples(workflow), ...(values ?? {}) };
  const steps = [];
  let failed = -1;
  for (const [i, step] of workflow.steps.entries()) {
    if (failed >= 0) {
      steps.push({ tool: step.tool, why: step.why, status: 'waiting', waitingOn: failed, input: null, result: null, carried: [] });
      continue;
    }
    const input = {};
    const carried = [];
    for (const [name, v] of Object.entries(step.input ?? {})) {
      if (isRef(v, 'from') && Array.isArray(v.from)) {
        const [at, key] = v.from;
        if (!(at < i)) throw new Error(`${workflow.slug} step ${i + 1}: "${name}" takes from step ${at + 1}, which is not earlier.`);
        const out = steps[at].result.result[key];
        if (out === undefined) throw new Error(`${workflow.slug} step ${i + 1}: step ${at + 1} (${steps[at].tool}) has no output "${key}".`);
        // A list carries only the columns the next tool takes, and no empty cells.
        input[name] = v.columns && Array.isArray(out)
          ? out.map((row) => Object.fromEntries(v.columns.filter((c) => row[c] !== undefined && row[c] !== null && row[c] !== '').map((c) => [c, asInput(row[c])])))
          : asInput(out);
        carried.push({ name, from: at });
      } else if (isRef(v, 'input')) {
        if (!(workflow.inputs ?? []).some((x) => x.name === v.input)) throw new Error(`${workflow.slug} step ${i + 1}: "${name}" names no workflow input "${v.input}".`);
        const value = given[v.input];
        // A blank optional input is left out, so the tool applies its own default.
        if (value === undefined || value === null || value === '') continue;
        input[name] = value;
      } else input[name] = v;
    }
    // The step's index rides along, so a caller can key its requests per step.
    const result = await invoke(step.tool, input, i);
    const ok = !!result?.ok;
    if (!ok) failed = i;
    steps.push({ tool: step.tool, why: step.why, status: ok ? 'ok' : 'failed', input, result, carried });
  }
  return { ok: failed < 0, steps, failed: failed < 0 ? null : failed };
}

/**
 * The inputs a workflow fixes rather than asks for: each literal step input,
 * as { step, tool, name, value }. The page lists them under "Assumptions".
 */
export function assumptions(workflow) {
  const out = [];
  for (const [i, step] of workflow.steps.entries()) {
    for (const [name, v] of Object.entries(step.input ?? {})) {
      if (isRef(v, 'from') || isRef(v, 'input')) continue;
      out.push({ step: i, tool: step.tool, name, value: v });
    }
  }
  return out;
}
