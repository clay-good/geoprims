#!/usr/bin/env python3
"""Golden vectors for the GNSS field tools, worked by hand: precision = (a +
b 1e-6 d) x the confidence factor (1, or 2.448 horizontal and 1.96 vertical
at 95%); OPUS rapid-static below 2 hours and static from 2 to 48 hours with
the RINEX 2 name ssssddd0.yyo; antenna height sqrt(s^2 - r^2) + offset."""
import datetime as dt
import json
import math
import sys
from pathlib import Path

SRC = "Worked by hand in Python (tools/vectors/gen_gnss.py)"
SPEC = "add-practitioner-essentials GNSS scenarios"


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def rtk():
    out = [vec(1, {"h_constant": "8 mm", "h_ppm": 1, "baseline": "15 km"}, {"result.horizontal.value": 23.0}, SPEC + " (8 mm + 1 ppm at 15 km: 23 mm)")]
    out.append(vec(2, {"h_constant": "10 mm", "h_ppm": 1, "baseline": "8 km", "v_constant": "20 mm", "v_ppm": 1, "confidence": "95"},
                   {"result.horizontal.value": 18 * 2.448, "result.vertical.value": 28 * 1.96}))
    out.append(vec(3, {"h_constant": "8 mm", "h_ppm": 0.5, "baseline": "30 km", "target": "20 mm"},
                   {"result.horizontal.value": 23.0, "result.target_status": "Beyond your 20 mm target"}))
    out.append(vec(4, {"h_constant": "5 mm", "h_ppm": 0.5, "baseline": "2 km", "target": "10 mm", "confidence": "95"},
                   {"result.horizontal.value": 6 * 2.448, "result.target_status": "Beyond your 10 mm target"}))
    out.append(vec(5, {"h_constant": "8 mm", "h_ppm": 1, "baseline": "15 km", "v_constant": "15 mm"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def opus():
    out = []
    for mins, date, st in [(60, "2026-09-22", "base"), (240, "2024-12-31", "p123"), (15, "2026-01-01", "ab12"), (119, None, None)]:
        inp = {"session": f"{mins} min"}
        exp = {"result.processing": "rapid static" if mins < 120 else "static", "result.software": "RSGPS rapid-static software" if mins < 120 else "PAGES static software"}
        if date:
            inp.update({"date": date, "station": st.upper() if st == "ab12" else st})
            doy = dt.date.fromisoformat(date).timetuple().tm_yday
            exp.update({"result.day_of_year": doy, "result.rinex": f"{st.lower()}{doy:03d}0.{int(date[2:4]):02d}o"})
        out.append(vec(len(out) + 1, inp, exp))
    out[0]["source"] = SPEC + " (a one-hour session gets rapid-static processing)"
    out.append(vec(len(out) + 1, {"session": "10 min"}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}))
    out.append(vec(len(out) + 1, {"session": "60 min", "date": "2026-02-30", "station": "base"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def antenna():
    out = []
    for s, r, off in [(1.8, 0.1, 0.0), (1.523, 0.1692, 0.0), (2.0, 0.15, -0.035), (1.2, 0.0835, 0.0)]:
        v = math.sqrt(s * s - r * r) + off
        inp = {"slant": f"{s} m", "radius": f"{r} m", "offset": f"{off} m"}
        out.append(vec(len(out) + 1, inp, {"result.vertical.value": v, "result.difference.value": s - v}))
    out[0]["source"] = SPEC + " (1.800 m slant, 0.100 m radius: about 1.7972 m)"
    out.append(vec(len(out) + 1, {"slant": "0.1 m", "radius": "0.15 m"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("survey.gnss.rtk-budget", rtk()), ("survey.gnss.opus-plan", opus()), ("survey.gnss.antenna-height", antenna())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
