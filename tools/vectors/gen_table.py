#!/usr/bin/env python3
"""Golden vectors for aviation.loading.table-interpolate, by hand:
multilinear interpolation over the cells that bracket the query, then
corrections in order."""
import itertools
import json
import sys
from pathlib import Path

SRC = "Multilinear interpolation worked in Python (tools/vectors/gen_table.py)"
SPEC = "add-aviation-suite scenarios"


def interp(axes, table, q):
    brackets = []
    for ax, x in zip(axes, q):
        i = next(i for i in range(len(ax) - 1) if x <= ax[i + 1])
        brackets.append((ax[i], ax[i + 1], (x - ax[i]) / (ax[i + 1] - ax[i])))
    total = 0.0
    for corner in itertools.product([0, 1], repeat=len(axes)):
        w, key = 1.0, []
        for (lo, hi, t), up in zip(brackets, corner):
            key.append(hi if up else lo)
            w *= t if up else 1 - t
        total += w * table[tuple(key)]
    return total


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def rows(table):
    out = []
    for key, v in table.items():
        r = dict(zip("abc", key))
        r["value"] = v
        out.append(r)
    return out


def main():
    out = []
    alt, tmp = [0, 2000, 4000], [0, 10, 20, 30]
    grid = {(0, 0): 1000, (0, 10): 1070, (0, 20): 1145, (0, 30): 1225, (2000, 0): 1150, (2000, 10): 1235, (2000, 20): 1325, (2000, 30): 1420,
            (4000, 0): 1335, (4000, 10): 1435, (4000, 20): 1540, (4000, 30): 1655}
    # The spec scenario: bilinear at 3,000 ft and 25 °C, then corrections.
    tv = interp([alt, tmp], grid, (3000, 25))
    out.append(vec(len(out) + 1, {"table": rows(grid), "at_a": 3000, "at_b": 25}, {"result.table_value": tv, "result.value": tv}, SPEC))
    corr = [{"label": "10 kt headwind", "kind": "percent", "amount": -10}, {"label": "dry grass", "kind": "multiply", "amount": 1.15}, {"label": "obstacle", "kind": "add", "amount": 50}]
    out.append(vec(len(out) + 1, {"table": rows(grid), "at_a": 3000, "at_b": 25, "corrections": corr}, {"result.table_value": tv, "result.value": tv * 0.9 * 1.15 + 50}))
    for q in [(0, 0), (4000, 30), (1000, 5), (2000, 15), (3500, 0)]:
        out.append(vec(len(out) + 1, {"table": rows(grid), "at_a": q[0], "at_b": q[1]}, {"result.table_value": interp([alt, tmp], grid, q)}))
    # One variable.
    one = {(1800,): 850, (2100,): 960, (2400,): 1095}
    out.append(vec(len(out) + 1, {"table": rows(one), "at_a": 2250}, {"result.table_value": interp([[1800, 2100, 2400]], one, (2250,))}))
    # Three variables: altitude × temperature × weight.
    three = {(a, t, w): 900 + 0.12 * a + 6 * t + 0.4 * (w - 2000) + 0.00003 * a * t for a in (0, 4000) for t in (0, 30) for w in (2000, 2400)}
    q = (2500, 12, 2300)
    out.append(vec(len(out) + 1, {"table": rows(three), "at_a": q[0], "at_b": q[1], "at_c": q[2]}, {"result.table_value": interp([[0, 4000], [0, 30], [2000, 2400]], three, q)}))
    # Refusals: beyond the table (the spec scenario), a gap in the grid, a missing lookup value.
    out.append(vec(len(out) + 1, {"table": rows(grid), "at_a": 5000, "at_b": 25, "names": "pressure altitude, temperature"}, {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": "/at_a"}, SPEC))
    gap = rows(grid)[:-1]
    out.append(vec(len(out) + 1, {"table": gap, "at_a": 1000, "at_b": 5}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(len(out) + 1, {"table": rows(grid), "at_a": 1000}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/at_b"}))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "aviation.loading.table-interpolate.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
