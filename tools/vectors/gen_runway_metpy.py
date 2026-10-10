#!/usr/bin/env python3
"""Golden vectors for aviation.wind.best-runway with the wind resolved by
MetPy, Unidata's meteorology library, which the core does not use.

MetPy's wind_components gives the wind's eastward and northward parts. A
runway end's headwind is the part of that vector against the direction of
landing, and its crosswind the part across it: two dot products with the
runway's heading (10 x its number). The tool works from the angle between
wind and runway instead. Runways are then ranked by the tool's stated rule:
within every limit first, then most headwind, then least crosswind.

No case has two runway ends within 0.01 kt of each other on the ranking keys,
and none has parallel runways, which tie.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_runway_metpy.py   (with metpy installed)
"""
import json
import math
from pathlib import Path

import metpy
from metpy.calc import wind_components
from metpy.units import units as u

SRC = f"MetPy {metpy.__version__} (wind_components), resolved along and across each runway heading and ranked by the stated rule (tools/vectors/gen_runway_metpy.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.wind.best-runway.jsonl"
TOL = {"abs": 1e-9}

# Lines from other generators are kept byte for byte.
kept = [line for line in OUT.read_text().splitlines() if "gen_runway_metpy.py" not in json.loads(line)["source"]]
rows = []
# runway numbers (each pair is one strip), wind from deg, kt, max crosswind kt, max tailwind kt
CASES = [([(9, 27), (18, 36)], 200, 12, None, None), ([(4, 22), (13, 31)], 250, 20, None, None), ([(1, 19), (10, 28)], 50, 25, 15, None),
         ([(16, 34)], 240, 18, 10, None), ([(7, 25), (12, 30), (17, 35)], 330, 14, None, None), ([(3, 21), (8, 26), (15, 33)], 105, 22, 12, None),
         ([(2, 20), (11, 29)], 275, 31, 20, 5), ([(6, 24)], 62, 9, None, None), ([(5, 23), (14, 32)], 187, 16, None, 3),
         ([(9, 27), (1, 19), (13, 31)], 355, 28, 18, None), ([(18, 36), (6, 24)], 143, 7, None, None), ([(10, 28), (15, 33)], 222, 40, 25, 10),
         ([(8, 26), (17, 35), (3, 21)], 18, 11, 8, None), ([(12, 30), (4, 22)], 296, 19, None, 0)]
for strips, wd, ws, max_x, max_t in CASES:
    ue, vn = (float(c.m) for c in wind_components(ws * u.knot, wd * u.deg))
    ends = []
    for a, b in strips:
        for n in (a, b):
            h = math.radians(10 * n)
            head = -(ue * math.sin(h) + vn * math.cos(h))
            cross = abs(ue * math.cos(h) - vn * math.sin(h))
            beyond = (max_x is not None and cross > max_x) or (max_t is not None and -head > max_t)
            ends.append((beyond, -head, cross, f"{n:02d}", head))
    ends.sort()
    for p, q in zip(ends, ends[1:]):
        assert p[0] != q[0] or abs(p[1] - q[1]) > 0.01 or abs(p[2] - q[2]) > 0.01, (strips, wd)
    for e in ends:
        assert max_x is None or abs(e[2] - max_x) > 0.01
        assert max_t is None or abs(-e[4] - max_t) > 0.01
    inp = {"runways": ", ".join(f"{a:02d}/{b:02d}" for a, b in strips), "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"}
    if max_x is not None:
        inp["max_crosswind"] = f"{max_x} kt"
    if max_t is not None:
        inp["max_tailwind"] = f"{max_t} kt"
    exp = {"ok": True, "result.best": ends[0][3], "result.best_headwind.value": ends[0][4], "result.best_crosswind.value": ends[0][2]}
    for k, e in enumerate(ends):
        exp[f"result.runways.{k}.runway"] = e[3]
        exp[f"result.runways.{k}.headwind.value"] = e[4]
    if max_x is not None or max_t is not None:
        exp["result.beyond_limits"] = sum(1 for e in ends if e[0])
    rows.append({"id": f"v{len(kept) + len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": f"MetPy {metpy.__version__}",
                 "tolerance": {k: TOL if isinstance(v, float) else {"abs": 0} for k, v in exp.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}})
OUT.write_text("".join(line + "\n" for line in kept + [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]))
print(f"{OUT.name}: {len(kept) + len(rows)} vectors")
