// Paired worked examples (trust/correctness-program layer B). When a forward
// tool and its inverse give their primary example the same title, they are
// telling the reader it is one point seen twice. Nothing checked that. The
// numbers in an example's `source` line and in its derivation note are prose,
// and a vector pins what the code computes, not what the page claims about it:
// geodesy.grid-ref.itm-inverse shipped with the Spire of Dublin at an easting
// and northing 1.1 m from where the forward tool puts it, and every gate was
// green, because both tools were right and only the example was wrong.
//
// This closes that: carry the inverse's example back through the forward tool
// and it must land on the forward tool's own answer. Only numbers the tools
// produce are compared, so no input string is parsed, and a pair that means
// two different points simply gives them two different titles.

/** Metre-and-nanodegree slack; the two routes are the same math either way. */
const ABS = 1e-6;
const REL = 1e-9;

const primary = (t) => (t.examples ?? []).find((e) => e.id === t['x-primary-example']) ?? (t.examples ?? [])[0];
const val = (x) => (x && typeof x === 'object' && 'value' in x ? x.value : x);

/** Every forward tool whose inverse shares its primary example's title. */
export function titledPairs(catalog) {
  const by = new Map(catalog.tools.map((t) => [t.id, t]));
  const out = [];
  for (const forward of catalog.tools) {
    if (!forward.id.endsWith('-forward')) continue;
    const inverse = by.get(forward.id.replace(/-forward$/, '-inverse'));
    if (!inverse) continue;
    const [f, i] = [primary(forward), primary(inverse)];
    if (f && i && f.title === i.title) out.push({ forward, inverse, title: f.title });
  }
  return out;
}

/** Where a pair's two examples disagree about the point they both name. */
export async function pairProblems({ catalog, host }) {
  const problems = [];
  const run = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));
  for (const { forward, inverse, title } of titledPairs(catalog)) {
    const there = await run(forward.id, primary(forward).input);
    const back = await run(inverse.id, primary(inverse).input);
    if (!there.ok || !back.ok) {
      problems.push(`${forward.id}/${inverse.id}: the "${title}" example fails`);
      continue;
    }
    // The inverse's own settings -- a zone, an ellipsoid -- carry over; its
    // answer supplies the coordinates.
    const input = {};
    for (const k of Object.keys(forward.inputs.properties)) {
      if (k in primary(inverse).input) input[k] = primary(inverse).input[k];
      if (k in back.result) input[k] = val(back.result[k]);
    }
    const again = await run(forward.id, input);
    if (!again.ok) {
      problems.push(`${forward.id}/${inverse.id}: "${title}" read back by ${inverse.id} is not a point ${forward.id} takes (${again.error.code})`);
      continue;
    }
    for (const [k, v] of Object.entries(there.result)) {
      const [want, got] = [val(v), val(again.result[k])];
      if (typeof want !== 'number' || typeof got !== 'number') continue;
      if (Math.abs(want - got) > Math.max(ABS, Math.abs(want) * REL)) {
        problems.push(
          `${forward.id}/${inverse.id}: both call their example "${title}" but it is not one point: ${forward.id} gives ${k} ${want}, and ${inverse.id}'s example carried back gives ${got}`,
        );
      }
    }
  }
  return problems;
}
