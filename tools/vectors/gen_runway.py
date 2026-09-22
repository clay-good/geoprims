#!/usr/bin/env python3
"""Golden vectors for the best-runway tool, worked by hand: each runway end
at 10 x its number, headwind W cos(wind - runway) and crosswind
W sin(wind - runway); runways within every limit first, then most
headwind, then least crosswind."""
import json
import math
import sys
from pathlib import Path

SRC = "Runway wind components ranked in Python (tools/vectors/gen_runway.py)"
SPEC = "add-aviation-suite ranking scenario"


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def rank(ends, wd, ws, max_x=None, max_t=None):
    rows = []
    for i, (label, hdg) in enumerate(ends):
        d = math.radians(wd - hdg)
        h, x = ws * math.cos(d), ws * math.sin(d)
        beyond = (max_x is not None and abs(x) > max_x) or (max_t is not None and -h > max_t)
        rows.append((beyond, -h, abs(x), i, label, h, abs(x)))
    rows.sort()
    return rows


def best():
    out = []
    cases = [
        ("09/27, 18/36", [("09", 90), ("27", 270), ("18", 180), ("36", 360)], 200, 12, None, None),
        ("09/27 18/36", [("09", 90), ("27", 270), ("18", 180), ("36", 360)], 300, 15, None, None),
        ("04L/22R, 04R/22L, 13/31", [("04L", 40), ("22R", 220), ("04R", 40), ("22L", 220), ("13", 130), ("31", 310)], 250, 20, None, None),
        ("16/34", [("16", 160), ("34", 340)], 240, 18, 10, None),
        ("01/19 10/28", [("01", 10), ("19", 190), ("10", 100), ("28", 280)], 50, 25, 15, None),
        ("rwy 9 27", [("09", 90), ("27", 270)], 90, 10, None, 5),
    ]
    for text, ends, wd, ws, mx, mt in cases:
        rows = rank(ends, wd, ws, mx, mt)
        inp = {"runways": text, "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"}
        if mx is not None:
            inp["max_crosswind"] = f"{mx} kt"
        if mt is not None:
            inp["max_tailwind"] = f"{mt} kt"
        exp = {"result.best": rows[0][4], "result.best_headwind.value": rows[0][5], "result.best_crosswind.value": rows[0][6]}
        for k, r in enumerate(rows):
            exp[f"result.runways.{k}.runway"] = r[4]
            exp[f"result.runways.{k}.headwind.value"] = r[5]
        if mx is not None or mt is not None:
            exp["result.beyond_limits"] = sum(1 for r in rows if r[0])
        out.append(vec(len(out) + 1, inp, exp))
    out[0]["source"] = SPEC + " (runway 18 ranks first, every runway listed)"
    # VRB: every runway's worst case is the full speed as crosswind and as tailwind, so input order stands.
    out.append(vec(len(out) + 1, {"runways": "09/27", "wind": "VRB08KT"},
                   {"result.best": "09", "result.best_headwind.value": -8.0, "result.best_crosswind.value": 8.0},
                   "add-aviation-suite variable-wind scenario (worst case 8 kt crosswind and tailwind)"))
    out.append(vec(len(out) + 1, {"runways": "27, 27", "wind_direction": "270 deg", "wind_speed": "10 kt"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(len(out) + 1, {"runways": "09/45", "wind_direction": "270 deg", "wind_speed": "10 kt"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.wind.best-runway", best())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
