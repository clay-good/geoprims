// "Copy the plan" (add-job-workflows, exports): a workflow as plain text for a
// briefing, a message, or a notebook. Every sentence is a step's own core
// summary; nothing is recomputed or reworded here.

/**
 * The plan as text: title, the reader's inputs by their field titles, each
 * step's answer (or why it stopped), and the link that reopens it.
 */
export function planText({ title, inputs, steps, url, today }) {
  const lines = [`${title} (geoprims, ${today})`, ''];
  for (const i of inputs) lines.push(`${i.title}: ${String(i.value).replace(/\n/g, '; ')}`);
  lines.push('');
  steps.forEach((s, n) => {
    const said = s.status === 'ok' ? s.result.summary : s.status === 'failed' ? `Stopped: ${s.result?.error?.message ?? 'no result'}` : `Waiting for step ${s.waitingOn + 1}.`;
    lines.push(`${n + 1}. ${s.title}: ${said}`);
  });
  lines.push('', 'Planning and education aid. Not for primary navigation.', url);
  return `${lines.join('\n')}\n`;
}
