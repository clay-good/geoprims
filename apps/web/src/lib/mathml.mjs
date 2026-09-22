// Formulas as MathML, at build time (web/tool-docs, "standard docs template
// with build-time MathML"). A trace step's formula becomes real mathematics
// only when it is one: symbols, numbers, operators, parentheses, functions,
// and sub- and superscripts. Two words side by side are prose ("the sum of
// weight × arm"), and prose stays prose: those return null and print as code.

const FUNCS = new Set(['sin', 'cos', 'tan', 'asin', 'acos', 'atan', 'atan2', 'sinh', 'cosh', 'tanh', 'sqrt', 'ln', 'log', 'exp', 'abs', 'min', 'max', 'floor', 'round', 'sign', 'mod', 'sec', 'csc', 'cot']);
const SUB = { '₀': '0', '₁': '1', '₂': '2', '₃': '3', '₄': '4', 'ᵢ': 'i', 'ⱼ': 'j', '₊': '+', '₋': '−', 'ₙ': 'n', 'ₖ': 'k' };
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

function tokenize(src) {
  const s = src.replace(/²/g, '^2').replace(/³/g, '^3');
  const out = [];
  const re = /\s*(?:(\d+(?:\.\d+)?(?:e[-−]?\d+)?)|([A-Za-zΑ-Ωα-ω][A-Za-z0-9Α-Ωα-ω′']*[₀-₄ᵢⱼ₊₋ₙₖ0-9]*)|(\S))/gy;
  const end = s.trimEnd().length;
  let at = 0;
  while (at < end) {
    re.lastIndex = at;
    const m = re.exec(s);
    if (!m || !m[0]) return null;
    at = re.lastIndex;
    if (m[1]) out.push({ t: 'num', v: m[1].replace('−', '-') });
    else if (m[2]) out.push({ t: 'id', v: m[2] });
    else out.push({ t: 'op', v: m[3] });
  }
  return out;
}

function ident(v) {
  const m = /^(.*?)([₀-₄ᵢⱼ₊₋ₙₖ]+)$/.exec(v);
  const mi = (x) => `<mi>${esc(x)}</mi>`;
  if (!m) return mi(v);
  const sub = [...m[2]].map((c) => SUB[c]).join('');
  const subML = sub.replace(/([a-z])|(\d+)|([+−])/g, (_, a, d, o) => (a ? `<mi>${a}</mi>` : d ? `<mn>${d}</mn>` : `<mo>${o}</mo>`));
  return `<msub>${mi(m[1])}<mrow>${subML}</mrow></msub>`;
}

/** The MathML for a formula, or null when it is prose or will not parse. */
export function toMathML(formula) {
  // "zoom/x/y" names a path, not a division.
  if (/[A-Za-z]\w*\/[A-Za-z]\w*\/[A-Za-z]/.test(String(formula ?? ''))) return null;
  const toks = tokenize(String(formula ?? ''));
  if (!toks || !toks.length) return null;
  let i = 0;
  const peek = () => toks[i];
  const is = (v) => toks[i]?.t === 'op' && toks[i].v === v;
  const fail = () => { throw new Error('not math'); };

  function primary() {
    const k = peek();
    if (!k) fail();
    if (k.t === 'num') { i++; return `<mn>${esc(k.v)}</mn>`; }
    if (k.t === 'id') {
      i++;
      if (FUNCS.has(k.v)) {
        let head = `<mi>${k.v}</mi>`;
        if (is('^')) { i++; head = `<msup>${head}${atom()}</msup>`; }
        return `<mrow>${head}<mo>&#x2061;</mo>${unary()}</mrow>`;
      }
      return ident(k.v);
    }
    if (is('(')) { i++; const e = expr(); if (!is(')')) fail(); i++; return `<mrow><mo>(</mo>${e}<mo>)</mo></mrow>`; }
    if (is('|')) { i++; const e = expr(); if (!is('|')) fail(); i++; return `<mrow><mo>|</mo>${e}<mo>|</mo></mrow>`; }
    if (is('⌊')) { i++; const e = expr(); if (!is('⌋')) fail(); i++; return `<mrow><mo>⌊</mo>${e}<mo>⌋</mo></mrow>`; }
    fail();
  }
  // An exponent's parentheses are only grouping: a superscript shows it.
  function atom() {
    if (is('(')) { i++; const e = expr(); if (!is(')')) fail(); i++; return `<mrow>${e}</mrow>`; }
    if (is('−') || is('-')) { i++; return `<mrow><mo>−</mo>${atom()}</mrow>`; }
    return primary();
  }
  function postfix() {
    let b = primary();
    while (is('°') || is('′') || is("'")) { b = `<mrow>${b}<mo>${toks[i].v === '°' ? '°' : '′'}</mo></mrow>`; i++; }
    if (is('^')) { i++; b = `<msup>${b}${atom()}</msup>`; }
    return b;
  }
  function unary() {
    if (is('−') || is('-')) { i++; return `<mrow><mo>−</mo>${unary()}</mrow>`; }
    if (is('√')) { i++; return `<msqrt>${unary()}</msqrt>`; }
    if (is('Σ')) { i++; return `<mrow><mo>∑</mo>${unary()}</mrow>`; }
    return postfix();
  }
  function term() {
    let out = unary();
    for (;;) {
      if (is('×') || is('·') || is('*') || is('/')) {
        const op = toks[i].v === '/' ? '/' : '×';
        i++;
        out += `<mo>${op}</mo>${unary()}`;
        continue;
      }
      // Side by side: a number or a closing bracket before a symbol or a
      // bracket multiplies ("2A", "(x − x₀)(y − y₀)"); two words are prose.
      const prev = toks[i - 1];
      const k = peek();
      if (k && (is('(') || k.t === 'id' || is('√') || is('|')) && prev && (prev.t === 'num' || (prev.t === 'op' && [')', '°', '|'].includes(prev.v)))) {
        out += unary();
        continue;
      }
      if (k && k.t === 'id' && prev?.t === 'id') fail();
      return out;
    }
  }
  function expr() {
    let out = term();
    while (is('+') || is('−') || is('-') || is('±')) {
      const op = toks[i].v === '±' ? '±' : toks[i].v === '+' ? '+' : '−';
      i++;
      out += `<mo>${op}</mo>${term()}`;
    }
    return out;
  }
  try {
    let out = expr();
    while (is('=') || is('≈')) { const op = toks[i].v; i++; out += `<mo>${op}</mo>${expr()}`; }
    if (i !== toks.length) return null;
    return `<math display="block" alttext="${esc(formula)}"><mrow>${out}</mrow></math>`;
  } catch {
    return null;
  }
}
