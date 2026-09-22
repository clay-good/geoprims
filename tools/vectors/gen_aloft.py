#!/usr/bin/env python3
"""Golden vectors for aviation.wind.uv and aviation.wind.aloft-interpolate,
by hand: u = -s sin(theta), v = -s cos(theta) for a wind from theta true, and
linear interpolation of u, v, and temperature between bracketing levels."""
import json
import math
import sys
from pathlib import Path

SRC = "Meteorological u/v convention and linear interpolation worked in Python (tools/vectors/gen_aloft.py)"


def uv(d, s):
    t = math.radians(d)
    return -s * math.sin(t), -s * math.cos(t)


def back(u, v):
    s = math.hypot(u, v)
    return (math.degrees(math.atan2(-u, -v)) % 360.0 if s > 1e-9 else 0.0), s


def vec(i, inp, exp, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": "2026", "tolerance": t}


def uv_vectors():
    out = []
    for d, s in [(300, 25), (0, 10), (90, 15), (180, 40), (270, 20), (45, 12.5), (225, 33), (360, 5)]:
        u, v = uv(d, s)
        out.append(vec(len(out) + 1, {"direction": f"{d} deg", "speed": f"{s} kt"}, {"result.u.value": u, "result.v.value": v, "result.speed.value": float(s), "result.direction.value": d % 360.0}))
    for u, v in [(21.650635094610966, -12.5), (0, -10), (-15, 0), (3, 4), (-7, -24)]:
        d, s = back(u, v)
        out.append(vec(len(out) + 1, {"u": f"{u} kt", "v": f"{v} kt"}, {"result.direction.value": d, "result.speed.value": s}))
    out.append(vec(len(out) + 1, {"u": "0 kt", "v": "0 kt"}, {"result.direction.value": 0.0, "result.speed.value": 0.0}))
    out.append(vec(len(out) + 1, {"direction": "300 deg", "speed": "25 kt", "u": "3 kt"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def aloft_vectors():
    out = []
    cases = [
        ([(6000, 270, 20, 3), (9000, 300, 30, -3)], 7500),
        ([(3000, 250, 10, 15), (6000, 260, 20, 9), (9000, 280, 35, 3), (12000, 290, 45, -3)], 10500),
        ([(6000, 350, 15, None), (9000, 10, 15, None)], 7000),  # veering through north
        ([(18000, 240, 60, -21), (24000, 250, 80, -33), (30000, 250, 95, -45)], 24000),
        ([(3000, 90, 5, 20), (6000, 270, 5, 14)], 4500),  # opposite winds cancel to calm
        ([(9000, 300, 30, -3), (6000, 270, 20, 3)], 6000),  # given out of order, on a level
    ]
    for levels, h in cases:
        rows = [dict({"altitude": f"{a} ft", "direction": f"{d} deg", "speed": f"{s} kt"}, **({"temperature": f"{t} degC"} if t is not None else {})) for a, d, s, t in levels]
        lv = sorted(levels)
        k = next(i for i in range(len(lv) - 1) if h <= lv[i + 1][0])
        a, b = lv[k], lv[k + 1]
        f = (h - a[0]) / (b[0] - a[0])
        (ua, va), (ub, vb) = uv(a[1], a[2]), uv(b[1], b[2])
        u, v = ua + f * (ub - ua), va + f * (vb - va)
        d, s = back(u, v)
        exp = {"result.u.value": u, "result.v.value": v, "result.speed.value": s, "result.below.value": float(a[0]), "result.above.value": float(b[0])}
        if s > 1e-6:
            exp["result.direction.value"] = d
        if a[3] is not None and b[3] is not None:
            exp["result.temperature.value"] = a[3] + f * (b[3] - a[3])
        out.append(vec(len(out) + 1, {"levels": rows, "altitude": f"{h} ft"}, exp))
    out.append(vec(len(out) + 1, {"levels": [{"altitude": "6000 ft", "direction": "270 deg", "speed": "20 kt"}, {"altitude": "9000 ft", "direction": "300 deg", "speed": "30 kt"}], "altitude": "12000 ft"},
                   {"ok": False, "error.code": "OUT_OF_DOMAIN"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.wind.uv", uv_vectors()), ("aviation.wind.aloft-interpolate", aloft_vectors())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
