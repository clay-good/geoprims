#!/usr/bin/env python3
"""Golden vectors for geodesy.parse.angle-arithmetic, by exact rational
arithmetic (Python fractions) on degrees, minutes, and seconds."""
import json
import sys
from fractions import Fraction as F
from pathlib import Path

SRC, VER = "Exact sexagesimal arithmetic with Python fractions (tools/vectors/gen_angle_arith.py)", "2026"


def secs(text):
    neg = text.startswith("-")
    d, m, s = (text.lstrip("-").split("-") + ["0", "0"])[:3]
    v = F(d) * 3600 + F(m) * 60 + F(s)
    return -v if neg else v


def dms(total):
    neg = total < 0
    units = round(abs(total) * 100)
    d, rem = divmod(units, 360000)
    m, rem = divmod(rem, 6000)
    s = F(rem, 100)
    out = f"{d}°{m:02d}'{float(s):05.2f}\""
    return ("-" + out) if neg and units else out


def main():
    cases = [
        ([("45-30-15", "add"), ("12-45-50", "add"), ("3-00-05", "subtract")], None),
        ([("359-59-59.5", "add"), ("0-0-1", "add")], "0-360"),
        ([("10-00-00", "add"), ("20-30-30", "subtract")], None),
        ([("170-15-00", "add"), ("25-50-30", "add")], "plus-minus-180"),
        ([("89-59-59.99", "add"), ("0-0-0.01", "add")], None),
        ([("-5-10-20", "add"), ("365-00-00", "add")], "0-360"),
        ([("123-45-6.78", "add"), ("98-7-54.32", "add"), ("21-52-58.9", "subtract"), ("0-0-0.1", "subtract")], None),
    ]
    out = []
    for terms, norm in cases:
        total = sum((secs(a) if op == "add" else -secs(a)) for a, op in terms)
        if norm == "0-360":
            total %= 1296000
        elif norm == "plus-minus-180":
            total = (total + 648000) % 1296000 - 648000
        inp = {"terms": [dict(angle=a, **({"operation": op} if op == "subtract" else {})) for a, op in terms]}
        if norm:
            inp["normalize"] = norm
        exp = {"result.dms": dms(total), "result.seconds.value": float(total), "result.degrees.value": float(total / 3600), "ok": True}
        tol = {"result.seconds.value": {"abs": 1e-4}, "result.degrees.value": {"abs": 1e-10}}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    out.append({"id": f"v{len(out) + 1:03d}", "input": {"terms": [{"angle": "north"}]}, "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                "source": SRC, "sourceVersion": VER, "tolerance": {}})
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geodesy.parse.angle-arithmetic.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    for v in out:
        print(v["expect"].get("result.dms"))


if __name__ == "__main__":
    main()
