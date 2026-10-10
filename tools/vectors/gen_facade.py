#!/usr/bin/env python3
"""Golden vectors for drone.mission.facade with every station placed by
GeographicLib for Python (Karney), which the core does not use: the facade's
length and bearing from Geodesic.WGS84.Inverse, each station along it and then
square to it by the standoff from Direct, and the path between stations from
Inverse again. Footprints, counts, and spacings are worked here from the
tool's stated equations.

Walls of 12 to 400 m facing every way, from 55 S to 78 N, flown from either
side, with scans that start above the base and overlaps other than the
defaults. No case has a photo or pass count within a millionth of a step of
the next whole number.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_facade.py   (with geographiclib installed)
"""
import json
import math
from importlib.metadata import version
from pathlib import Path

from geographiclib.geodesic import Geodesic

VER = f"geographiclib {version('geographiclib')}"
SRC = f"Stations by GeographicLib for Python {version('geographiclib')} (Geodesic.WGS84 Inverse and Direct); coverage arithmetic (Wolf, Dewitt & Wilkinson 2014, ch. 6) (tools/vectors/gen_facade.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/drone.mission.facade.jsonl"
G = Geodesic.WGS84


def stations(span, foot, overlap):
    if span <= foot:
        return [span / 2]
    steps = (span - foot) / (foot * (1 - overlap))
    assert abs(steps - round(steps)) > 1e-6, (span, foot, overlap)
    n = math.ceil(steps) + 1
    return [foot / 2 + k * (span - foot) / (n - 1) for k in range(n)]


# start lat, lon, wall bearing, wall length m, standoff m, top m, bottom m or None,
# sensor w mm, sensor h mm, focal mm, image width px, overlaps % (along, up) or None, side
CASES = [
    (40.4406, -80.002, 90, 50, 30, 25, None, 13.2, 8.8, 8.8, 5472, None, None),
    (51.5007, -0.1246, 20, 96, 45, 93, 10, 13.2, 8.8, 8.8, 5472, (80, 70), "left"),
    (-33.8568, 151.2153, 135, 180, 60, 67, None, 35.9, 24.0, 35.0, 8192, (70, 60), "right"),
    (78.2232, 15.6267, 250, 40, 15, 12, 2, 6.17, 4.55, 4.5, 4000, None, "left"),
    (-54.8019, -68.303, 0, 25, 10, 9, None, 17.3, 13.0, 12.29, 5280, (75, 65), None),
    (1.2834, 103.8607, 310, 150, 45, 55, 5, 23.5, 15.6, 16.0, 6000, (60, 50), "right"),
    (35.6586, 139.7454, 180, 12, 40, 30, None, 13.2, 8.8, 8.8, 5472, None, "left"),
    (36.0156, -114.7378, 65, 400, 230, 221, None, 35.9, 24.0, 50.0, 8192, (65, 55), "left"),
    (64.1417, -21.9266, 200, 30, 8, 20, 4, 6.17, 4.55, 4.5, 4000, (70, 55), None),
    (-22.9519, -43.2105, 100, 75, 25, 38, None, 23.5, 15.6, 24.0, 6000, None, "right"),
    (19.4326, -99.1332, 45, 110, 35, 44, 6, 17.3, 13.0, 12.29, 5280, (70, 70), "left"),
    (48.8584, 2.2945, 290, 125, 80, 300, 57, 35.9, 24.0, 35.0, 8192, (60, 60), "right"),
]

rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "gen_facade.py" not in r["source"]]
for lat, lon, brg, length, standoff, top, bottom, sw, sh, f, iw, overlaps, side in CASES:
    end = G.Direct(lat, lon, brg, length)
    a, b = (lat, lon), (round(end["lat2"], 9), round(end["lon2"], 9))
    inp = {"facade": [{"lat": a[0], "lon": a[1]}, {"lat": b[0], "lon": b[1]}], "standoff": f"{standoff} m", "top_height": f"{top} m",
           "sensor_width": f"{sw} mm", "focal_length": f"{f} mm", "image_width": iw, "sensor_height": f"{sh} mm"}
    if bottom is not None:
        inp["bottom_height"] = f"{bottom} m"
    if overlaps:
        inp["horizontal_overlap"], inp["vertical_overlap"] = overlaps
    if side:
        inp["side"] = side
    oh, ov = (overlaps[0] / 100, overlaps[1] / 100) if overlaps else (0.75, 0.60)
    wall = G.Inverse(a[0], a[1], b[0], b[1])
    w, h = sw * standoff / f, sh * standoff / f
    cols, levels = stations(wall["s12"], w, oh), stations(top - (bottom or 0), h, ov)
    stn = []
    for s in cols:
        p = G.Direct(a[0], a[1], wall["azi1"], s)
        out = p["azi2"] + (-90 if side == "left" else 90)
        q = G.Direct(p["lat2"], p["lon2"], out, standoff)
        stn.append((q["lat2"], (q["lon2"] + 540) % 360 - 180, (out + 180) % 360))
    wps = []
    for r, z in enumerate(levels):
        wps += [(*stn[k], (bottom or 0) + z) for k in (range(len(stn)) if r % 2 == 0 else reversed(range(len(stn))))]
    assert len(wps) <= 60, len(wps)
    path = sum(math.hypot(G.Inverse(p[0], p[1], q[0], q[1])["s12"], q[3] - p[3]) for p, q in zip(wps, wps[1:]))
    gap = lambda v: v[1] - v[0] if len(v) > 1 else 0.0
    exp = {"ok": True, "result.gsd.value": sw / iw * standoff / f * 100, "result.footprint_width.value": w, "result.footprint_height.value": h,
           "result.photo_spacing.value": gap(cols), "result.pass_spacing.value": gap(levels), "result.passes": len(levels),
           "result.photos_per_pass": len(cols), "result.photos": len(wps), "result.facade_length.value": wall["s12"], "result.path_length.value": path}
    tol = {k: {"abs": 0} if isinstance(v, int) else {"abs": 1e-8} for k, v in exp.items() if k != "ok"}
    for i, (la, lo, hd, z) in enumerate(wps):
        for key, v, t in (("lat", la, 1e-10), ("lon", lo, 1e-10), ("height", z, 1e-9), ("heading", hd, 1e-7)):
            exp[f"result.waypoints.{i}.{key}.value"] = v
            tol[f"result.waypoints.{i}.{key}.value"] = {"abs": t}
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
