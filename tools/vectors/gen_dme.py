#!/usr/bin/env python3
"""Golden vectors for the DME tools, worked by hand: ground distance
sqrt(DME^2 - h^2) with h in NM (1 NM = 1852 m, 1 ft = 0.3048 m); time to
station t / sin(change) from where the timing ends and 60 t / change by
the rule; a standard-rate turn radius GS / (60 pi) and the arc lead
asin(r / (R - r)), with 60 r / R by the rule."""
import json
import math
import sys
from pathlib import Path

SRC = "Right-triangle and turn geometry worked in Python (tools/vectors/gen_dme.py)"
SPEC = "add-practitioner-essentials scenarios"
FT_NM = 0.3048 / 1852


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def slant():
    out = []
    for dme, ft in [(5.0, 6000), (10, 3000), (2.5, 10000), (30, 35000), (1.2, 7000), (20, 0)]:
        h = ft * FT_NM
        g = math.sqrt(dme * dme - h * h)
        out.append(vec(len(out) + 1, {"dme": f"{dme} NM", "height": f"{ft} ft"},
                       {"result.ground_distance.value": g, "result.slant_error.value": dme - g, "result.elevation_angle.value": math.degrees(math.asin(h / dme))}))
    out[0]["source"] = SPEC + " (slant range: 4.902 NM)"
    out.append(vec(len(out) + 1, {"dme": "0.8 NM", "height": "6000 ft"}, {"ok": False, "error.code": "NO_SOLUTION"}, SPEC + " (overhead)"))
    return out


def tts():
    out = []
    for t, d, gs in [(2, 10, 120), (1.5, 5, 90), (3, 15, 150), (4, 20, None), (1, 1, 100), (2.25, 12.5, 135)]:
        exact, rule = t / math.sin(math.radians(d)), 60 * t / d
        inp = {"minutes": f"{t} min", "bearing_change": f"{d} deg"}
        exp = {"result.time.value": exact, "result.time_rule.value": rule}
        if gs:
            inp["groundspeed"] = f"{gs} kt"
            exp.update({"result.distance.value": gs * exact / 60, "result.distance_rule.value": gs * rule / 60})
        out.append(vec(len(out) + 1, inp, exp))
    out.append(vec(len(out) + 1, {"minutes": "2 min", "bearing_change": "95 deg"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def arc():
    out = []
    for big_r, gs, r in [(10, 120, None), (15, 150, None), (7, 90, None), (12, 200, None), (10, 120, 1.0), (20, 250, None)]:
        rr = r if r is not None else gs / (60 * math.pi)
        inp = {"arc": f"{big_r} NM", "groundspeed": f"{gs} kt"}
        if r is not None:
            inp["turn_radius"] = f"{r} NM"
        out.append(vec(len(out) + 1, inp, {"result.turn_radius.value": rr, "result.onto_arc_lead.value": rr,
                                           "result.lead_radials.value": math.degrees(math.asin(rr / (big_r - rr))), "result.lead_radials_rule.value": 60 * rr / big_r}))
    out.append(vec(len(out) + 1, {"arc": "1 NM", "groundspeed": "200 kt"}, {"ok": False, "error.code": "NO_SOLUTION"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.ifr.dme-slant-range", slant()), ("aviation.ifr.time-to-station", tts()), ("aviation.ifr.dme-arc-lead", arc())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
