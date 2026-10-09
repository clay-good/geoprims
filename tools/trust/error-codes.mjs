// A tool's `errors` tells an agent what refusals to expect, so it must list
// every code the tool returns. Three are implicit for every tool: any field can
// be malformed (INVALID_INPUT), any quantity can carry the wrong kind of unit
// (UNIT_MISMATCH), and the runtime caps input size and call time for all of
// them (LIMIT_EXCEEDED). INTERNAL is never declared: it is a defect.
export const IMPLICIT = new Set(['INVALID_INPUT', 'UNIT_MISMATCH', 'LIMIT_EXCEEDED']);

/** Why `code` from `tool` is not declared, or null when it is. */
export function undeclared(tool, code) {
  if (IMPLICIT.has(code) || tool.errors.includes(code)) return null;
  return `${tool.id} returns ${code}, which its errors do not declare`;
}
