#!/usr/bin/env python3
"""Golden vectors for raster.terrain.line-of-sight, worked independently of the
core: the sight line is a straight line in the plane of the path, written as the
polar equation of a line through two points,

    1 / r(phi) = [sin(theta - phi) / r_o + sin(phi) / r_t] / sin(theta),

on a sphere of effective radius Re = R / (1 - k), with each profile point at the
angle s / Re. The core instead intersects the chord with each vertical in
Cartesian coordinates. The observer height that clears a point solves the same
equation for r_o with the line through the point and the target. The first
Fresnel radius is sqrt(lambda d1 d2 / d) (ITU-R P.530).

Usage: python3 tools/vectors/gen_sightline.py
"""
import json
import math
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/vectors/raster.terrain.line-of-sight.jsonl"
C = 299_792_458.0
UNITS = {"m": 1.0, "km": 1000.0, "ft": 0.3048, "mi": 1609.344}


def solve(profile, ho, ht, k=0.13, radius=6_371_000.0, freq_hz=None):
    re = radius / (1 - k)
    s0 = profile[0][0]
    pts = [(s - s0, e) for s, e in profile]
    total = pts[-1][0]
    theta = total / re
    ro = re + pts[0][1] + ho
    rt = re + pts[-1][1] + ht
    rows, need = [], -math.inf
    for s, e in pts[1:-1]:
        phi = s / re
        r = math.sin(theta) / (math.sin(theta - phi) / ro + math.sin(phi) / rt)
        clear = r - re - e
        rg = re + e
        inv_ro = (math.sin(theta) / rg - math.sin(phi) / rt) / math.sin(theta - phi)
        need = max(need, 1 / inv_ro - re - pts[0][1])
        row = {"s": s, "e": e, "line": r - re, "clear": clear}
        if freq_hz:
            row["f1"] = math.sqrt(C / freq_hz * s * (total - s) / total)
            row["ratio"] = clear / row["f1"]
        rows.append(row)
    least = min(rows, key=lambda w: w["clear"])
    out = {
        "visible": "yes" if all(w["clear"] >= 0 for w in rows) else "no",
        "clearance": least["clear"],
        "clearance_at": least["s"],
        "need": max(need, 0.0),
        "obstructions": float(sum(w["clear"] < 0 for w in rows)),
        "rows": rows,
        "least": least,
    }
    if freq_hz:
        worst = min(rows, key=lambda w: w["ratio"])
        short = sum(w["ratio"] < 0.6 for w in rows)
        out.update(fresnel_clear="yes" if short == 0 else "no", worst=worst, short=float(short))
    return out


def case(profile, ho, ht, du="km", eu="m", k=None, radius=None, freq_ghz=None, note=""):
    """profile in the given units; returns one vector row."""
    base = [(d * UNITS[du], e * UNITS[eu]) for d, e in profile]
    kw = {}
    if k is not None:
        kw["k"] = k
    if radius is not None:
        kw["radius"] = radius
    r = solve(base, ho * UNITS[eu], ht * UNITS[eu], freq_hz=freq_ghz * 1e9 if freq_ghz else None, **kw)
    inp = {
        "points": [{"distance": f"{d} {du}", "elevation": f"{e} {eu}"} for d, e in profile],
        "observer_height": f"{ho} {eu}",
        "target_height": f"{ht} {eu}",
    }
    if freq_ghz:
        inp["frequency"] = f"{freq_ghz} GHz"
    if k is not None:
        inp["k"] = k
    if radius is not None:
        inp["radius"] = f"{radius} m"
    L, D = UNITS[eu], UNITS[du]
    exp = {
        "ok": True,
        "result.visible": r["visible"],
        "result.clearance.value": r["clearance"] / L,
        "result.clearance_at.value": r["clearance_at"] / D,
        "result.observer_height_needed.value": r["need"] / L,
        "result.obstructions": r["obstructions"],
    }
    i = r["rows"].index(r["least"])
    exp[f"result.profile.{i}.sight_line.value"] = r["least"]["line"] / L
    if r["visible"] == "no":
        exp["result.obstruction_elevation.value"] = r["least"]["e"] / L
        exp["result.obstruction_height.value"] = -r["clearance"] / L
    if freq_ghz:
        j = r["rows"].index(r["worst"])
        exp["result.fresnel_clear"] = r["fresnel_clear"]
        exp["result.fresnel_worst_percent"] = r["worst"]["ratio"] * 100
        exp["result.fresnel_worst_at.value"] = r["worst"]["s"] / D
        exp["result.fresnel_short"] = r["short"]
        exp[f"result.profile.{j}.fresnel_radius.value"] = r["worst"]["f1"] / L
    tol = {}
    for key, v in exp.items():
        if isinstance(v, float):
            # Heights are differences of radii near 6,400 km: a micrometer of
            # rounding in either geometry is far below any displayed digit.
            tol[key] = {"rel": 1e-9, "abs": 1e-6}
    src = "Exact circular geometry worked independently in Python: the polar equation of the line through the two antennas on a sphere of radius R/(1 - k)"
    if note:
        src += f"; {note}"
    return {"input": inp, "expect": exp, "source": src, "sourceVersion": "2026", "tolerance": tol}


RIDGE = [(0, 300), (4, 340), (8, 395), (12, 330), (16, 310)]
CASES = [
    case(RIDGE, 2, 30, freq_ghz=5.8, note="the ridge scenario: the 395 m ridge at 8 km blocks the view"),
    case(RIDGE, 2, 30, note="the ridge scenario without a frequency"),
    case(RIDGE, 165, 30, freq_ghz=5.8, note="raised past the ridge, the view clears but the Fresnel zone does not"),
    case(RIDGE, 160, 60, freq_ghz=5.8, note="the Fresnel scenario: 60% of the zone clear"),
    case(RIDGE, 160, 60, freq_ghz=5.8, k=0.25, note="radio refraction, k = 0.25"),
    case(RIDGE, 160, 60, freq_ghz=5.8, k=0.0, note="no refraction"),
    case([(0, 0), (10, 0), (20, 0), (30, 0)], 10, 12, note="flat sea: a 10 m and a 12 m mast 30 km apart are hidden by the curve alone"),
    case([(0, 0), (10, 0), (20, 0), (30, 0)], 60, 70, freq_ghz=2.4, note="flat sea, 60 m and 70 m masts at 2.4 GHz"),
    case([(0, 0), (5, 0), (10, 0)], 2, 2, note="flat ground 10 km: two eyes at 2 m just see each other over the curve"),
    case([(0, 1000), (0.5, 1010), (1, 1005), (1.5, 1020), (2, 1000)], 1.5, 1.5, note="a short hiking ridge"),
    case([(0, 1200), (0.3, 1180), (0.6, 1150), (0.9, 1100)], 1.7, 1.7, note="a convex slope hides the foot of the hill"),
    case([(0, 600), (1, 610), (2, 640), (3, 630), (4, 620), (5, 615)], 30, 2, freq_ghz=0.9, note="a 900 MHz link"),
    case([(0, 500), (2, 800), (4, 500)], 10, 10, note="a single tall peak"),
    case([(0, 1000), (2000, 1030), (4000, 1100), (6000, 1060), (8000, 1020)], 6, 100, du="ft", eu="ft", note="feet throughout"),
    case([(0, 300), (1, 320), (2, 350), (3, 340)], 400, 20, du="mi", eu="ft", freq_ghz=5.8, note="a drone 400 ft up, miles and feet"),
    case([(10, 300), (14, 340), (18, 395), (22, 330), (26, 310)], 160, 60, freq_ghz=5.8, note="the Fresnel scenario with the profile starting at 10 km"),
    case([(0, 0), (20, 50), (40, 120), (60, 80), (80, 0)], 200, 200, freq_ghz=6.0, note="an 80 km microwave hop"),
    case([(0, 0), (20, 50), (40, 120), (60, 80), (80, 0)], 200, 200, freq_ghz=6.0, k=-0.5, note="sub-refraction, k = -0.5"),
    case([(0, 0), (50, 0), (100, 0)], 300, 300, radius=6_378_137, note="the WGS 84 equatorial radius"),
    case([(0, 120), (0.2, 125), (0.4, 128), (0.6, 126), (0.8, 124), (1.0, 121)], 1.8, 0.5, freq_ghz=2.4, note="a 1 km drone control link at 2.4 GHz"),
    case([(0, 50), (3, 52), (6, 49)], 0, 0, note="both points on the ground"),
    case([(0, 200), (100, 2000), (200, 200)], 10, 10, note="a 2,000 m mountain 100 km out"),
]

with OUT.open("w") as f:
    for i, row in enumerate(CASES, 1):
        f.write(json.dumps({"id": f"v{i:03d}", **row}) + "\n")
print(f"wrote {len(CASES)} vectors to {OUT}")
