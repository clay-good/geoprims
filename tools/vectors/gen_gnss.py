#!/usr/bin/env python3
"""Golden vectors for the GNSS field tools, worked by hand: precision = (a +
b 1e-6 d) x the confidence factor (1, or 2.448 horizontal and 1.96 vertical
at 95%); OPUS rapid-static below 2 hours and static from 2 to 48 hours with
the RINEX 2 name ssssdddh.yyo; antenna height sqrt(s^2 - r^2) + offset."""
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
    tersus = "Tersus GNSS, How does Tersus handle long-range single baseline RTK positioning?, 2023-04-06"
    out.append(vec(6, {"h_constant": "8 mm", "h_ppm": 1, "baseline": "40 km", "v_constant": "15 mm", "v_ppm": 1},
                   {"result.horizontal.value": 48.0, "result.vertical.value": 55.0}, tersus + " (published Oscar receiver worked example)"))
    out[-1]["sourceVersion"] = "2023-04-06"
    cases = [
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "0 km"}, {"result.horizontal.value": 8.0}),
        ({"h_constant": "8 mm", "h_ppm": 0.1, "baseline": "10000 km"}, {"result.horizontal.value": 1008.0}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "10000.001 km"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/baseline"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "-1 m"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/baseline"}),
        ({"h_constant": "0 mm", "h_ppm": 0, "baseline": "0 km"}, {"result.horizontal.value": 0.0}),
        ({"h_constant": "-0.1 mm", "h_ppm": 1, "baseline": "1 km"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/h_constant"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "40 km", "target": "60 mm"}, {"result.horizontal.value": 48.0, "result.target_status": "Within your 60 mm target"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "40 km", "target": "48 mm"}, {"result.horizontal.value": 48.0, "result.target_status": "Near your 48 mm target"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "40 km", "target": "40 mm"}, {"result.horizontal.value": 48.0, "result.target_status": "Beyond your 40 mm target"}),
        ({"h_constant": "0.8 cm", "h_ppm": 1, "baseline": "40000 m", "v_constant": "1.5 cm", "v_ppm": 1}, {"result.horizontal.value": 48.0, "result.vertical.value": 55.0}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "1 mi"}, {"result.horizontal.value": 9.609344}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "40 km", "v_constant": "15 mm", "v_ppm": 1, "confidence": "95"}, {"result.horizontal.value": 117.504, "result.vertical.value": 107.8}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "40 km", "v_constant": "15 mm", "v_ppm": 1, "confidence": "1sigma"}, {"result.horizontal.value": 48.0, "result.vertical.value": 55.0}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "1 km", "v_ppm": 1}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/v_constant"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "1 km", "v_constant": "-1 mm", "v_ppm": 1}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/v_constant"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "1 km", "target": "-1 mm"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/target"}),
        ({"h_constant": "0 mm", "h_ppm": 1000, "baseline": "1 km"}, {"result.horizontal.value": 1000.0}),
        ({"h_constant": "8 mm", "h_ppm": 1000.001, "baseline": "1 km"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/h_ppm"}),
        ({"h_constant": "8 mm", "h_ppm": 1, "baseline": "1 km", "v_constant": "15 mm", "v_ppm": 1000.001}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/v_ppm"}),
    ]
    for inp, exp in cases:
        out.append(vec(len(out) + 1, inp, exp))
    assert len(out) == 25
    return out


def opus():
    out = []
    ngs_name = "NOAA NGS OPUS Projects User Guide v2.0, section 5.1"

    def success(session, date=None, station=None, hour=None, source=SRC):
        minutes = session * 60 if isinstance(session, float) else session
        inp = {"session": f"{session} h" if isinstance(session, float) else f"{session} min"}
        exp = {
            "result.processing": "rapid static" if minutes < 120 else "static",
            "result.software": "RSGPS rapid-static software" if minutes < 120 else "PAGES static software",
        }
        if date:
            inp["date"] = date
            if station is not None:
                inp["station"] = station
            if hour is not None:
                inp["start_hour"] = hour
            doy = dt.date.fromisoformat(date).timetuple().tm_yday
            mark = (station or "site").lower()
            code = "0" if minutes == 1440 else chr(ord("a") + hour)
            exp.update({"result.day_of_year": doy, "result.rinex": f"{mark}{doy:03d}{code}.{int(date[2:4]):02d}o"})
        out.append(vec(len(out) + 1, inp, exp, source))

    success(60, "2023-01-09", "g287", 16, ngs_name + " (published example g287009q.23o)")
    out[-1]["sourceVersion"] = "Version 2.0"
    success(240, "2024-12-31", "p123", 23)
    success(15, "2026-01-01", "AB12", 0)
    success(119)
    out.append(vec(5, {"session": "10 min"}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}))
    out.append(vec(6, {"session": "60 min", "date": "2026-02-30", "station": "base"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    success(120)
    success(2880)
    out.append(vec(9, {"session": "2880.001 min"}, {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": "/session"}))
    success(15)
    out.append(vec(11, {"session": "14.999 min"}, {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": "/session"}))
    success(0.25)
    success(2.0)
    success(24.0, "2026-09-22", "base")
    success(60, "2024-02-29", "leap", 12)
    success(60, "2025-12-31", "z9z9", 1)
    success(60, "2026-04-10", None, 8)
    failures = [
        ({"session": "60 min", "date": "2026-01-01", "station": "bad!", "start_hour": 0}, "/station"),
        ({"session": "60 min", "date": "2026-01-01", "station": "base", "start_hour": 12.5}, "/start_hour"),
        ({"session": "60 min", "date": "2026-01-01", "station": "base", "start_hour": 24}, "/start_hour"),
        ({"session": "60 min", "date": "2026-01-01", "station": "base"}, "/start_hour"),
        ({"session": "60 min", "station": "base"}, "/station"),
        ({"session": "60 min", "start_hour": 8}, "/start_hour"),
    ]
    for inp, field in failures:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT", "error.field": field}))
    success(60, "1980-01-01", "a000", 0)
    success(60, "2079-12-31", "x999", 23)
    assert len(out) == 25
    return out


def antenna():
    out = []
    for s, r, off in [(1.8, 0.1, 0.0), (1.523, 0.1692, 0.0), (2.0, 0.15, -0.035), (1.2, 0.0835, 0.0)]:
        v = math.sqrt(s * s - r * r) + off
        inp = {"slant": f"{s} m", "radius": f"{r} m", "offset": f"{off} m"}
        out.append(vec(len(out) + 1, inp, {"result.vertical.value": v, "result.difference.value": s - v}))
    out[0]["source"] = SPEC + " (1.800 m slant, 0.100 m radius: about 1.7972 m)"
    out.append(vec(len(out) + 1, {"slant": "0.1 m", "radius": "0.15 m"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    manual = "iGage iG9 User Manual, revision L (2021), page 83"
    for s, published in [(2.123, 2.0474), (1.569, 1.4931)]:
        out.append(vec(len(out) + 1,
                       {"slant": f"{s} m", "radius": "0.07505 m", "offset": "-0.0742 m"},
                       {"result.vertical.value": published}, manual, 1e-4))
        out[-1]["sourceVersion"] = "Revision L, 2021-05-13"

    cases = [
        ("2 m", "0 m", "0.1 m"),
        ("180 cm", "100 mm", "0 mm"),
        ("6 ft", "4 in", "-1 in"),
        ("0.1000001 m", "0.1 m", "0 m"),
        ("100 m", "0.2 m", "1 m"),
        ("2 m", "0.15 m", "-1 m"),
        ("1.5 m", "0 m", "-0.5 m"),
        ("0.5 m", "0.1 m", "0.02 m"),
        ("10 m", "1 m", "-0.25 m"),
        ("0.002 km", "75.05 mm", "-74.2 mm"),
        ("6.965 ft", "75.05 mm", "-74.2 mm"),
        ("80 in", "3 in", "-2.9 in"),
        ("2500 mm", "125 mm", "35 mm"),
        ("3 yd", "0.5 ft", "2 in"),
    ]
    units = {"m": 1.0, "cm": 0.01, "mm": 0.001, "km": 1000.0,
             "in": 0.0254, "ft": 0.3048, "yd": 0.9144}

    def meters(text):
        value, symbol = text.split()
        return float(value) * units[symbol]

    for slant, radius, offset in cases:
        s, r, off = meters(slant), meters(radius), meters(offset)
        v = math.sqrt(s * s - r * r) + off
        out.append(vec(len(out) + 1,
                       {"slant": slant, "radius": radius, "offset": offset},
                       {"result.vertical.value": v, "result.difference.value": s - v}))

    failures = [
        ({"slant": "1 m", "radius": "-0.1 m"}, "/radius"),
        ({"slant": "1 m", "radius": "1 m"}, "/slant"),
        ({"slant": "0 m", "radius": "0 m"}, "/slant"),
        ({"slant": "100.001 m", "radius": "0.1 m"}, "/slant"),
        ({"slant": "2 m", "radius": "0.1 m", "offset": "1.001 m"}, "/offset"),
    ]
    for inp, field in failures:
        out.append(vec(len(out) + 1, inp,
                       {"ok": False, "error.code": "INVALID_INPUT", "error.field": field}))
    assert len(out) == 26
    return out


def alta():
    K = math.sqrt(-2 * math.log(0.05))  # 2.4477, the 2-D 95% factor
    out = [vec(1, {"distance": "1000 ft", "semi_major": "0.05 ft"},
               {"result.allowable.value": 0.12, "result.semi_major_95.value": 0.05, "result.status": "Within your 0.12 ft allowable RPP (ALTA/NSPS 2026)"},
               SPEC + " (0.12 ft allowable at 1,000 ft)", 1e-12)]
    out.append(vec(2, {"distance": "300 m", "semi_major": "0.034 m"},
                   {"result.allowable.value": 0.035 / 0.3048, "result.status": "Near your 0.035 m allowable RPP (ALTA/NSPS 2026)"}, tol=1e-12))
    se, sn, c = 0.02, 0.015, 0.0001
    a, b = se * se, sn * sn
    big = (a + b) / 2 + math.sqrt(((a - b) / 2) ** 2 + c * c)
    e = K * math.sqrt(big)
    out.append(vec(3, {"distance": "500 ft", "sigma_e": f"{se} ft", "sigma_n": f"{sn} ft", "covariance": c},
                   {"result.allowable.value": 0.07 + 50e-6 * 500, "result.semi_major_95.value": e,
                    "result.status": "Within your 0.095 ft allowable RPP (ALTA/NSPS 2026)" if e < 0.095 * 0.9 else ("Near your 0.095 ft allowable RPP (ALTA/NSPS 2026)" if e <= 0.095 else "Beyond your 0.095 ft allowable RPP (ALTA/NSPS 2026)")}, tol=1e-9))
    out.append(vec(4, {"distance": "2000 ft", "semi_major": "0.2 ft"}, {"result.allowable.value": 0.17, "result.status": "Beyond your 0.17 ft allowable RPP (ALTA/NSPS 2026)"}, tol=1e-12))
    out.append(vec(5, {"distance": "1000 ft", "misclosure": "0.05 ft"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/misclosure"}, SPEC + " (a misclosure is refused: RPP comes from the error ellipse)"))
    source = "ALTA/NSPS Minimum Standard Detail Requirements (2026), Section 3.E.v, read in the official PDF on 2026-10-09: 2 cm (0.07 feet) plus 50 parts per million of the distance"
    for distance in [0, 100, 250, 660, 1320, 2640, 5280, 10000]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": {"distance": f"{distance} ft"},
                    "expect": {"ok": True, "result.allowable.value": 0.07 + 50e-6 * distance},
                    "source": source, "sourceVersion": "ALTA/NSPS 2026",
                    "tolerance": {"result.allowable.value": {"abs": 1e-9}}})
    for distance in [0, 50, 100, 250, 500, 1000, 2000]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": {"distance": f"{distance} m"},
                    "expect": {"ok": True, "result.allowable.value": (0.02 + 50e-6 * distance) / 0.3048},
                    "source": source + " (allowable shown in feet)", "sourceVersion": "ALTA/NSPS 2026",
                    "tolerance": {"result.allowable.value": {"abs": 1e-9}}})
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    selected = set(sys.argv[2:])
    for name, vs in [("survey.gnss.rtk-budget", rtk()), ("survey.gnss.opus-plan", opus()), ("survey.gnss.antenna-height", antenna()), ("survey.land.alta-rpp", alta())]:
        if selected and name not in selected:
            continue
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
