// The example's diagrams, drawn at build time for a tool page, so the page
// shows them before (and without) loading any diagram code in the browser.
import { DIAGRAM_TOOLS, diagram } from './diagrams.js';

/** ToolApp's diagram props for a tool and its example's result. */
export function firstDrawings(tool, example, result) {
  const hasDiagram = DIAGRAM_TOOLS.includes(tool.id);
  if (!hasDiagram) return { hasDiagram };
  return {
    hasDiagram,
    firstDiagram: diagram(tool.id, example, result, '', { turn: 0, tilt: 90 }),
    firstInline: tool['x-diagram-inline'] ? diagram(tool.id, example, result, 'inline') : null,
  };
}
