#!/usr/bin/env python3
"""Golden vectors for the 1-in-60 off-course tool, worked by hand: track
error atan(off / flown), closing angle atan(off / remaining), and the turn
to the destination as their sum; by the rule, 60 off / distance each."""
import json
import math
import sys
from pathlib import Path

SRC = "Flat-plane off-course geometry worked in Python (tools/vectors/gen_offcourse.py)"
SPEC = "add-aviation-suite 1-in-60 scenario"


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def one_in_sixty():
    out = []
    for flown, off, rem in [(60, 4, 60), (30, 2, 90), (100, 5, 50), (45, 0, 15), (20, 10, 20), (120, 3, 180)]:
        te, ca = math.degrees(math.atan(off / flown)), math.degrees(math.atan(off / rem))
        ter, car = 60 * off / flown, 60 * off / rem
        out.append(vec(len(out) + 1, {"flown": f"{flown} NM", "off_course": f"{off} NM", "remaining": f"{rem} NM"},
                       {"result.track_error.value": te, "result.closing_angle.value": ca, "result.to_destination.value": te + ca,
                        "result.track_error_rule.value": ter, "result.closing_angle_rule.value": car, "result.to_destination_rule.value": ter + car}))
    out[0]["source"] = SPEC + " (3.81° and 7.63° exact; 4° and 8° by the rule)"
    out.append(vec(len(out) + 1, {"flown": "0 NM", "off_course": "4 NM", "remaining": "60 NM"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(len(out) + 1, {"flown": "60 NM", "off_course": "-4 NM", "remaining": "60 NM"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.wind.one-in-sixty", one_in_sixty())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
