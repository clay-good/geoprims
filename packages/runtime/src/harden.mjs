// Input hardening before any call reaches the core (compute-core "Input
// hardening before execution"): rejects numbers that overflow binary64,
// duplicate object keys, nesting deeper than 32, strings over 1 MB, and
// payloads over the surface's size limit. JSON.parse silently keeps the last
// duplicate key, so this scanner walks the text itself.

export const LIMITS = { depth: 32, stringBytes: 1_000_000, mcpBytes: 10_000_000, webBytes: 50_000_000 };

const err = (code, message, field) => ({ ok: false, error: { code, message, ...(field ? { field } : {}) } });
const ptr = (path) => '/' + path.map((p) => String(p).replaceAll('~', '~0').replaceAll('/', '~1')).join('/');

/**
 * Checks `text` and returns null when it is safe to pass to the core, or an
 * error envelope naming the offending JSON Pointer.
 */
export function checkJson(text, maxBytes = LIMITS.mcpBytes) {
  const bytes = new TextEncoder().encode(text).length;
  if (bytes > maxBytes) {
    return err('LIMIT_EXCEEDED', `The input is ${bytes} bytes; the limit is ${maxBytes}.`);
  }
  let i = 0;
  const path = [];
  const ws = () => {
    while (i < text.length && ' \t\n\r'.includes(text[i])) i++;
  };
  class Bad extends Error {
    constructor(env) {
      super('bad');
      this.env = env;
    }
  }
  const fail = (code, message) => {
    throw new Bad(err(code, message, path.length ? ptr(path) : undefined));
  };
  const str = () => {
    const start = i;
    i++; // opening quote
    while (i < text.length && text[i] !== '"') i += text[i] === '\\' ? 2 : 1;
    if (i >= text.length) fail('INVALID_INPUT', 'Unterminated string.');
    i++;
    const raw = text.slice(start, i);
    const value = JSON.parse(raw);
    if (new TextEncoder().encode(value).length > LIMITS.stringBytes) {
      fail('LIMIT_EXCEEDED', `A string is longer than ${LIMITS.stringBytes} bytes.`);
    }
    return value;
  };
  const value = (depth) => {
    ws();
    const c = text[i];
    if (c === '{' || c === '[') {
      if (depth >= LIMITS.depth) fail('INVALID_INPUT', `The input nests deeper than ${LIMITS.depth} levels.`);
      i++;
      ws();
      const close = c === '{' ? '}' : ']';
      const seen = new Set();
      let n = 0;
      if (text[i] === close) {
        i++;
        return;
      }
      for (;;) {
        ws();
        if (c === '{') {
          if (text[i] !== '"') fail('INVALID_INPUT', 'Expected a key.');
          const key = str();
          if (seen.has(key)) {
            path.push(key);
            fail('INVALID_INPUT', `The key "${key}" appears twice.`);
          }
          seen.add(key);
          ws();
          if (text[i++] !== ':') fail('INVALID_INPUT', 'Expected ":".');
          path.push(key);
        } else {
          path.push(n++);
        }
        value(depth + 1);
        path.pop();
        ws();
        if (text[i] === ',') {
          i++;
          continue;
        }
        if (text[i] === close) {
          i++;
          return;
        }
        fail('INVALID_INPUT', 'Malformed JSON.');
      }
    } else if (c === '"') {
      str();
    } else {
      const m = /^-?\d+(\.\d+)?([eE][+-]?\d+)?|^(true|false|null)/.exec(text.slice(i, i + 400));
      if (!m) fail('INVALID_INPUT', 'Malformed JSON.');
      if (m[3] === undefined && !Number.isFinite(Number(m[0]))) {
        fail('INVALID_INPUT', `The number ${m[0]} is too large to represent.`);
      }
      i += m[0].length;
    }
  };
  try {
    JSON.parse(text); // rejects syntax errors with a precise message first
  } catch (e) {
    return err('INVALID_INPUT', `The input is not valid JSON: ${e.message}`);
  }
  try {
    value(0);
    return null;
  } catch (e) {
    if (e instanceof Bad) return e.env;
    throw e;
  }
}
