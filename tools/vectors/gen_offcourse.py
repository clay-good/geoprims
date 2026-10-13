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
AP3456 = "UK Ministry of Defence AP3456, Volume 9, chapter 19, paragraphs 2-4 and Figure 3"


def vec(i, inp, exp, src=SRC, tol=1e-9, version="2026"):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": version, "tolerance": t}


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
    for flown, off, rem, units, source in [
        (30, 2, 30, "NM", AP3456 + ": 2 NM off after 30 NM gives 4 degrees by the rule"),
        (30, 10, 60, "NM", AP3456 + ": a 10 NM side over 30 NM gives 20 degrees by the rule"),
        (60, 1, 120, "NM", SRC),
        (75, 2.5, 25, "NM", SRC),
        (200, 8, 100, "km", SRC),
        (12, 0.25, 48, "mi", SRC),
        (1, 0, 1, "NM", SRC),
        (500, 20, 300, "NM", SRC),
        (2.5, 0.1, 7.5, "km", SRC),
        (180, 3, 45, "NM", SRC),
        (25, 5, 75, "NM", SRC),
    ]:
        te, ca = math.degrees(math.atan(off / flown)), math.degrees(math.atan(off / rem))
        ter, car = 60 * off / flown, 60 * off / rem
        out.append(vec(len(out) + 1, {"flown": f"{flown} {units}", "off_course": f"{off} {units}", "remaining": f"{rem} {units}"},
                       {"result.track_error.value": te, "result.closing_angle.value": ca, "result.to_destination.value": te + ca,
                        "result.track_error_rule.value": ter, "result.closing_angle_rule.value": car, "result.to_destination_rule.value": ter + car}, source,
                       version="Volume 9, revised March 2011" if source.startswith(AP3456) else "2026"))
    out.append(vec(len(out) + 1, {"flown": "60 NM", "off_course": "4 NM", "remaining": "0 NM"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/remaining"},
                   "The tool's stated domain: distance remaining must be positive"))
    assert len(out) == 20
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.wind.one-in-sixty", one_in_sixty())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
