// What each copy action puts on the clipboard (web/tool-app "Result panel":
// units, copy formats, provenance). Pure, so every format can be checked
// without a clipboard: a copied value has to carry enough provenance that
// someone pasting it elsewhere can tell where it came from.

/** The copy formats the answer card offers, in the order it offers them. */
export const FORMATS = ['value', 'sentence', 'link', 'agent-call', 'json'];

/**
 * The text for one format.
 * `answer` is the displayed value, `result` the envelope, `tool` the client
 * manifest, `args` the inputs as the core would take them, `href` the page's
 * permalink.
 */
export function copyText(kind, { answer, result, tool, args, href }) {
  switch (kind) {
    case 'value':
      return answer;
    case 'sentence':
      // The reference travels with the sentence, so a pasted claim is traceable.
      return `${result.summary} (geoprims ${tool.id} ${tool.version})`;
    case 'link':
      return href;
    case 'agent-call':
      return JSON.stringify({ name: 'geoprims_run', arguments: { id: tool.id, args } }, null, 2);
    case 'json':
      return JSON.stringify(result, null, 2);
    default:
      throw new Error(`no copy format ${kind}`);
  }
}

/**
 * What the system share sheet is offered (ux/mobile-and-field, "Printing and
 * sharing from mobile"): the sentence with its reference, and the permalink.
 * The same text a paste would carry, so a share and a copy say the same thing.
 */
export const sharePayload = (parts) => ({
  title: `${parts.tool.title} · geoprims`,
  text: copyText('sentence', parts),
  url: copyText('link', parts),
});
