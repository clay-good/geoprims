#!/usr/bin/env python3
"""Golden vectors for drone.mission.corridor from GEOS and PROJ: each
centerline is projected to a transverse Mercator plane at its middle point
(pyproj), Shapely (GEOS) offsets it there with mitered joins, and pyproj takes
every offset corner back to latitude and longitude. The centerline length is
the geodesic length from GeographicLib for Python. None of the three is used by
the core, which has its own projection and its own offset.

Centerlines of two to six points with bends up to 120 degrees, from 55 S to
69 N, one to six lines each. Legs are kept long beside the offsets, so GEOS returns one corner per centerline point and no case turns on
how a pinched inside corner is trimmed.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_corridor.py   (with pyproj, shapely, and geographiclib installed)
"""
import json
import math
from importlib.metadata import version
from pathlib import Path

import pyproj
import shapely
from geographiclib.geodesic import Geodesic
from shapely.geometry import LineString

VER = f"GEOS {shapely.geos_version_string}, PROJ {pyproj.proj_version_str}, geographiclib {version('geographiclib')}"
SRC = f"Shapely {shapely.__version__} offset_curve (GEOS {shapely.geos_version_string}) on a transverse Mercator plane from PROJ {pyproj.proj_version_str}; length from GeographicLib for Python {version('geographiclib')} (tools/vectors/gen_corridor.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/drone.mission.corridor.jsonl"

# (center lat, center lon, centerline in meters east and north of the center, width m, spacing m)
CASES = [
    (40.0, -105.0, [(-900, -400), (0, 0), (800, 700)], 120, 52.5),
    (51.48, -0.12, [(-1500, 0), (1500, 0)], 100, 50),
    (-33.86, 151.21, [(-2000, -500), (-800, -100), (300, 400), (1800, 300)], 300, 60),
    (68.9, 33.1, [(0, -3000), (200, -1000), (-100, 800), (600, 2600)], 250, 45),
    (-54.8, -68.3, [(-600, 600), (0, 0), (700, -500), (1500, -400), (2300, 200)], 90, 30),
    (1.35, 103.82, [(-5000, -5000), (-2000, -3500), (1000, -3000), (4000, -500), (6000, 3000)], 400, 80),
    (35.68, 139.69, [(500, 900), (-400, 100), (-600, -1100)], 30, 40),
    (19.43, -99.13, [(-12000, 2000), (-6000, 0), (0, 500), (7000, -1500), (13000, 1000), (20000, 800)], 600, 150),
    (64.15, -21.94, [(-300, -300), (300, 300)], 200, 35),
    (-22.9, -43.2, [(0, 0), (1000, 200), (1900, 900), (2200, 2000)], 160, 40),
    (47.6, 7.6, [(-800, 1200), (-200, 300), (100, -900), (900, -1500), (2100, -1600)], 75, 25),
    (-1.29, 36.82, [(-25000, -8000), (-9000, -2000), (6000, -3000), (24000, 5000)], 500, 125),
    (60.17, 24.94, [(1000, -700), (200, -100), (-900, 100), (-1700, 900)], 140, 70),
    (-41.29, 174.78, [(-400, -1800), (-100, -600), (-300, 700), (200, 1900)], 55, 20),
    (25.2, 55.27, [(-3000, 1000), (-1000, 400), (1500, 900), (3500, 100)], 210, 52.5),
    (45.0, 9.0, [(-1000, 0), (0, 0), (0, 1200)], 150, 50),
    (-12.05, -77.04, [(-1500, -200), (0, 0), (900, 1400), (2600, 1300)], 100, 50),
    (55.75, 37.62, [(0, 0), (2000, 0), (1000, 1732), (-1000, 1732)], 80, 40),
]

rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "gen_corridor.py" not in r["source"]]
for lat0, lon0, line, width, spacing in CASES:
    lay = pyproj.Proj(proj="aeqd", lat_0=lat0, lon_0=lon0, ellps="WGS84")
    ll = [{"lat": round(la, 9), "lon": round(lo, 9)} for lo, la in (lay(x, y, inverse=True) for x, y in line)]
    # The tool's plane is centered on the middle point as entered.
    mid = ll[len(ll) // 2]
    plane = pyproj.Proj(proj="tmerc", lat_0=mid["lat"], lon_0=mid["lon"], k=1, ellps="WGS84")
    center = LineString([plane(p["lon"], p["lat"]) for p in ll])
    n = max(1, math.ceil(width / spacing - 1e-9))
    exp = {"ok": True, "result.line_count": n,
           "result.centerline_length.value": sum(Geodesic.WGS84.Inverse(a["lat"], a["lon"], b["lat"], b["lon"])["s12"] for a, b in zip(ll, ll[1:])) / 1000}
    tol = {"result.line_count": {"abs": 0}, "result.centerline_length.value": {"abs": 1e-9}}
    flown = []
    for k in range(n):
        off = (k - (n - 1) / 2) * spacing
        # Shapely offsets to the left for a positive distance; the tool's offsets are right positive.
        pts = list(center.coords) if off == 0 else list(center.offset_curve(-off, join_style="mitre", mitre_limit=10).coords)
        assert len(pts) == len(ll), (lat0, off, len(pts))
        if math.hypot(pts[0][0] - center.coords[0][0], pts[0][1] - center.coords[0][1]) > abs(off) + 1e-6:
            pts.reverse()  # GEOS before 3.11 returned right-side offsets reversed
        if k % 2:
            pts.reverse()
        flown += pts
        exp[f"result.lines.{k}.offset.value"] = off
        tol[f"result.lines.{k}.offset.value"] = {"abs": 1e-9}
        for j, (x, y) in enumerate(pts):
            lo, la = plane(x, y, inverse=True)
            exp[f"result.lines.{k}.waypoints.{j}.lat.value"], exp[f"result.lines.{k}.waypoints.{j}.lon.value"] = la, lo
            tol[f"result.lines.{k}.waypoints.{j}.lat.value"] = tol[f"result.lines.{k}.waypoints.{j}.lon.value"] = {"abs": 2e-8}
    exp["result.path_length.value"] = sum(math.dist(a, b) for a, b in zip(flown, flown[1:])) / 1000
    tol["result.path_length.value"] = {"abs": 1e-6}
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": {"centerline": ll, "width": f"{width} m", "line_spacing": f"{spacing} m"},
                 "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
