#!/usr/bin/env python3
"""Golden vectors for the last two wind-triangle forms, worked by hand as
east/north vectors: air = ground - wind and ground = air + wind, where the
wind vector points where the wind blows to (its direction + 180)."""
import json
import math
import sys
from pathlib import Path

SRC = "Wind-triangle vectors worked in Python (tools/vectors/gen_triangle.py)"
SPEC = "add-aviation-suite heading-and-groundspeed scenario, solved the other way"


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def en(az, s):
    a = math.radians(az)
    return s * math.sin(a), s * math.cos(a)


def polar(e, n):
    return math.degrees(math.atan2(e, n)) % 360.0, math.hypot(e, n)


def wrap(a):
    return (a + 180.0) % 360.0 - 180.0


def tas_from_gs():
    out = []
    for crs, gs, wd, ws in [(90, 108.7, 30, 20), (360, 150, 270, 30), (180, 95, 180, 25), (45, 120, 225, 15), (270, 60, 90, 0), (315, 200, 20, 45)]:
        ge, gn = en(crs, gs)
        we, wn = en(wd + 180, ws)
        h, tas = polar(ge - we, gn - wn)
        out.append(vec(len(out) + 1, {"course": f"{crs} deg", "groundspeed": f"{gs} kt", "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"},
                       {"result.tas.value": tas, "result.heading.value": h, "result.wind_correction_angle.value": wrap(h - crs)}))
    out[0]["source"] = SPEC + " (about 120 kt on heading 081.7°)"
    out.append(vec(len(out) + 1, {"course": "90 deg", "groundspeed": "0 kt", "wind_direction": "30 deg", "wind_speed": "20 kt"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(len(out) + 1, {"course": "90 deg", "groundspeed": "100 kt", "wind": "VRB05KT"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def course_from_heading():
    out = []
    for hdg, tas, wd, ws in [(81.7, 120, 30, 20), (360, 150, 270, 30), (180, 95, 180, 25), (45, 120, 225, 15), (270, 60, 90, 0), (10, 40, 120, 55)]:
        ae, an = en(hdg, tas)
        we, wn = en(wd + 180, ws)
        c, gs = polar(ae + we, an + wn)
        out.append(vec(len(out) + 1, {"heading": f"{hdg} deg", "tas": f"{tas} kt", "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"},
                       {"result.course.value": c, "result.groundspeed.value": gs, "result.drift_angle.value": wrap(c - hdg)}))
    out[0]["source"] = SPEC + " (course about 090° at 108.7 kt)"
    out.append(vec(len(out) + 1, {"heading": "90 deg", "tas": "30 kt", "wind_direction": "90 deg", "wind_speed": "30 kt"}, {"ok": False, "error.code": "NO_SOLUTION"}))
    out.append(vec(len(out) + 1, {"heading": "90 deg", "tas": "100 kt", "wind_direction": "30 deg", "wind_speed": "20 kt", "wind_reference": "magnetic"}, {"ok": False, "error.code": "INVALID_INPUT"}, "add-aviation-suite mixed-reference scenario"))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.wind.tas-from-groundspeed", tas_from_gs()), ("aviation.wind.course-from-heading", course_from_heading())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
