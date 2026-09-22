// Learning guides (web/tool-docs, "Learning guides"): the practitioner
// journeys in data/journeys.json, each a chain of tools. An input is a value,
// or {"from": [step, "output"]} to take an earlier step's answer. The chain is
// run through the core when the site is built, so every step's link opens its
// tool already holding the previous steps' real outputs.

/** 12 significant digits: drops binary noise (72.96000000000001), far below any tool's precision. */
const tidy = (n) => (typeof n === 'number' && Number.isFinite(n) ? Number(n.toPrecision(12)) : n);

/** An output as a tool input: a quantity as "value unit"; anything else as it is. */
export function asInput(v) {
  if (v && typeof v === 'object' && !Array.isArray(v) && 'value' in v) return v.unit && v.unit !== '1' ? `${tidy(v.value)} ${v.unit}` : tidy(v.value);
  return tidy(v);
}

/**
 * Runs a journey: each step's inputs resolved from literals and earlier
 * results, then run through `invoke(id, input)`. Returns the steps with their
 * inputs and results, or throws naming the step that failed, so a broken
 * chain stops the build rather than publishing a guide that does not work.
 */
export async function runJourney(journey, invoke) {
  const done = [];
  for (const [i, step] of journey.steps.entries()) {
    const input = {};
    for (const [name, v] of Object.entries(step.input ?? {})) {
      if (v && typeof v === 'object' && !Array.isArray(v) && Array.isArray(v.from)) {
        const [at, key] = v.from;
        if (!(at < i)) throw new Error(`${journey.slug} step ${i + 1}: "${name}" takes from step ${at + 1}, which is not earlier.`);
        const out = done[at].result.result[key];
        if (out === undefined) throw new Error(`${journey.slug} step ${i + 1}: step ${at + 1} (${done[at].tool}) has no output "${key}".`);
        // A list carries only the columns the next tool takes, and no empty cells.
        input[name] = v.columns && Array.isArray(out)
          ? out.map((row) => Object.fromEntries(v.columns.filter((c) => row[c] !== undefined && row[c] !== null && row[c] !== '').map((c) => [c, asInput(row[c])])))
          : asInput(out);
      } else input[name] = v;
    }
    const result = await invoke(step.tool, input);
    if (!result?.ok) throw new Error(`${journey.slug} step ${i + 1} (${step.tool}) failed: ${result?.error?.message ?? 'no result'}`);
    done.push({ ...step, input, result, carried: Object.entries(step.input ?? {}).filter(([, v]) => v?.from).map(([name, v]) => ({ name, from: v.from[0] })) });
  }
  return done;
}
