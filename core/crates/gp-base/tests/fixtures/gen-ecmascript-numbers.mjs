// Regenerate: node gen-ecmascript-numbers.mjs > ecmascript-numbers.txt (JSON.stringify is the oracle).
const out = [];
const buf = new DataView(new ArrayBuffer(8));
let seed = 0x9e3779b97f4a7c15n;
const next = () => { seed ^= seed << 13n; seed &= (1n<<64n)-1n; seed ^= seed >> 7n; seed ^= seed << 17n; seed &= (1n<<64n)-1n; return seed; };
const push = (x) => { if (!Number.isFinite(x)) return; buf.setFloat64(0, x); out.push(buf.getBigUint64(0).toString(16).padStart(16,'0') + ' ' + JSON.stringify(x)); };
for (let i = 0; i < 6000; i++) { buf.setBigUint64(0, next()); push(buf.getFloat64(0)); }
for (let e = -330; e <= 310; e++) { push(10 ** e); push(1.5 * 10 ** e); push(9.999999999999999 * 10 ** e); }
for (let i = 0; i < 2000; i++) { push(Number(next() % 100000000n) / 1000); push(Number(next() % 1000000n) * 1e-9); }
for (const x of [1e21, 1e21 - 65536, 999999999999999900000, 1e-6, 1e-7, 0.000001234, 5e-324, 2.2250738585072014e-308, Number.MAX_SAFE_INTEGER, 2**53+2, 123e-20, 0.1+0.2]) push(x);
console.log(out.join('\n'));
