#!/usr/bin/env python3
"""Golden vectors for drone.mission.orbit from GeographicLib for Python 2.1
(C. F. F. Karney's own implementation; pip install geographiclib), which the
core's Rust port is checked against: each waypoint is the geodesic direct
problem from the center at an equal step of azimuth, its heading is the way
back to the center, and the circumference is the chord sum scaled to the arc.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_orbit.py   (with geographiclib installed)
"""
import json
import math
from importlib.metadata import version
from pathlib import Path

from geographiclib.geodesic import Geodesic

VER = version("geographiclib")
SRC = f"GeographicLib for Python {VER} (Karney): Geodesic.WGS84 Direct and Inverse (tools/vectors/gen_orbit.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/drone.mission.orbit.jsonl"
G = Geodesic.WGS84


def orbit(lat, lon, r, n, clockwise=True):
    pts = []
    for k in range(n + 1):
        az = (360.0 if clockwise else -360.0) * k / n
        d = G.Direct(lat, lon, az, r)
        pts.append((d["lat2"], (d["lon2"] + 540) % 360 - 180, (d["azi2"] + 180) % 360))
    chords = sum(G.Inverse(a[0], a[1], b[0], b[1])["s12"] for a, b in zip(pts, pts[1:]))
    arc = chords * (math.pi / n) / math.sin(math.pi / n)
    return pts[:n], arc


rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "GeographicLib for Python" not in r["source"]]
CASES = [(40.0, -105.0, 50, 80, 60, 12, True), (40.0, -105.0, 50, 80, 60, 12, False), (-33.8568, 151.2153, 120, 100, 0, 36, True),
         (64.84, -147.72, 30, 40, 10, 8, True), (0.0, 0.0, 200, 120, 0, 24, True), (51.4779, -0.0015, 75, 60, 30, 16, False),
         (19.5, 179.9995, 150, 90, 0, 20, True), (-54.8, -68.3, 40, 35, 12, 10, True), (35.68, 139.69, 500, 120, 100, 48, True),
         (88.5, 20.0, 60, 50, 0, 12, True), (-88.5, -120.0, 60, 50, 0, 12, False), (25.2, 55.27, 300, 400, 380, 30, True),
         (47.4, 8.5, 10, 15, 5, 6, True), (1.35, 103.82, 1000, 120, 0, 60, True), (-22.9, -43.2, 85, 70, 38, 18, False)]
for lat, lon, r, h, t, n, cw in CASES:
    pts, arc = orbit(lat, lon, r, n, cw)
    inp = {"lat": lat, "lon": lon, "radius": f"{r} m", "height": f"{h} m", "target_height": f"{t} m", "photos": n}
    if not cw:
        inp["rotation"] = "counterclockwise"
    exp = {"ok": True, "result.gimbal_pitch.value": -math.degrees(math.atan((h - t) / r)), "result.circumference.value": arc, "result.photo_spacing.value": arc / n}
    tol = {"result.gimbal_pitch.value": {"abs": 1e-9}, "result.circumference.value": {"abs": 1e-6}, "result.photo_spacing.value": {"abs": 1e-6}}
    for k in sorted({0, 1, n // 2, n - 1}):
        la, lo, hd = pts[k]
        for key, val, eps in [("lat", la, 1e-9), ("lon", lo, 1e-9), ("heading", hd, 1e-6)]:
            exp[f"result.waypoints.{k}.{key}.value"] = val
            tol[f"result.waypoints.{k}.{key}.value"] = {"abs": eps}
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": f"geographiclib {VER}", "tolerance": tol})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
