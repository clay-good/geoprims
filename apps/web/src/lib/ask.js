// One way of answering "which tool, with which values?" for the home search
// and the palette (discovery/natural-language-prefill). The core search ranks
// the tools and fills the top one's inputs from the question; values that look
// like coordinates or codes are detected and offered too. The same question
// always gives the same first result, so Enter always goes to the same place.
import { detect, search } from './compute.js';

const route = (id) => '/' + id.split('.').join('/') + '/';

/**
 * Results for a question, best first: the top tool opened with the values the
 * question gave, then anything detected in it, then the ranked tools.
 * Each result has a title, a summary, and an href. Returns null when a newer
 * question has already replaced this one.
 */
export async function ask(query, { limit = 8 } = {}) {
  const q = String(query ?? '').trim();
  if (!q) return { results: [], filled: false };
  const [out, det] = await Promise.all([
    search({ query: q, limit, includeExperimental: true }),
    // Values contain digits, "+", or "/"; plain words only search.
    /[\d+/]/.test(q) ? detect(q) : null,
  ]);
  if (!out) return null;
  const found = det?.ok ? det.result.found : [];
  const detected = found.flatMap((f) =>
    f.actions.map((a, k) => ({
      kind: 'detected',
      title: a.title,
      summary: `Opens with ${f.value}`,
      href: a.href,
      head: k === 0 ? `Detected: ${f.label}.` : null,
      headDetail: f.summary,
    })),
  );
  const tools = (out.ok ? out.result.results : []).map((t) => ({ kind: 'tool', ...t, href: route(t.id) }));
  const filled = (tools[0]?.open ?? []).map((a) => ({ kind: 'filled', tool: tools[0].title, id: tools[0].id, ...a }));
  return { results: [...filled, ...detected, ...tools], filled: filled.length > 0, found };
}
