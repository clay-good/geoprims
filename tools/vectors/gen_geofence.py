#!/usr/bin/env python3
"""Golden vectors for drone.mission.geofence from GEOS and PROJ: each field is
projected to an azimuthal equidistant plane at its own centroid (pyproj), where
distances within a kilometer of the center are true to parts per billion, and
Shapely (GEOS) measures each waypoint's distance to the field there. A waypoint
farther out than the fence distance is outside by the difference.

The fields are irregular and concave, at latitudes from 55 S to 69 N; the
tool's first vectors are rectangles on the equator in closed form. Waypoints
within 5 cm of the fence or the warning line are left out, so no case turns on
the last millimeter.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_geofence.py   (with pyproj and shapely installed)
"""
import json
import random
from pathlib import Path

import pyproj
import shapely
from shapely.geometry import Point, Polygon

SRC = f"Shapely {shapely.__version__} (GEOS {shapely.geos_version_string}) on an azimuthal equidistant plane from PROJ {pyproj.proj_version_str} (tools/vectors/gen_geofence.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/drone.mission.geofence.jsonl"

# (center lat, center lon, outline in meters east and north of the center, fence m, warning m or None)
FIELDS = [
    (40.44, -80.0, [(-120, -80), (140, -95), (160, 60), (20, 110), (-130, 70)], 50, None),
    (51.48, -0.12, [(-200, -150), (200, -150), (200, 150), (40, 150), (40, 0), (-40, 0), (-40, 150), (-200, 150)], 30, 20),
    (-33.86, 151.21, [(-90, -60), (95, -70), (60, 80), (-70, 65)], 25, None),
    (68.9, 33.1, [(-300, -100), (-50, -160), (250, -90), (320, 120), (0, 60), (-280, 140)], 80, 50),
    (-54.8, -68.3, [(-60, -60), (60, -60), (60, 60), (-60, 60)], 40, 25),
    (1.35, 103.82, [(-180, -40), (0, -120), (190, -30), (150, 100), (-20, 40), (-160, 110)], 60, None),
    (35.68, 139.69, [(-75, -110), (80, -100), (110, 0), (70, 115), (-85, 105), (-115, 5)], 35, 15),
    (19.43, -99.13, [(-400, -250), (380, -260), (410, 240), (-390, 255)], 100, 70),
    (64.15, -21.94, [(-50, -140), (55, -140), (55, 140), (-50, 140)], 20, None),
    (-22.9, -43.2, [(-150, -20), (-20, -150), (140, -30), (30, 20), (150, 140), (-10, 60), (-140, 150)], 45, 30),
]

rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "azimuthal equidistant plane" not in r["source"]]
rng = random.Random(20261010)
for lat0, lon0, outline, d, warn in FIELDS:
    plane = pyproj.Proj(proj="aeqd", lat_0=lat0, lon_0=lon0, ellps="WGS84")
    field = Polygon(outline)
    assert field.is_valid
    area = [{"lat": round(la, 9), "lon": round(lo, 9)} for lo, la in (plane(x, y, inverse=True) for x, y in outline)]
    # Measure from the rounded corners, as the tool will read them.
    field = Polygon([plane(p["lon"], p["lat"]) for p in area])
    wps, dists = [], []
    minx, miny, maxx, maxy = field.bounds
    reach = 2.2 * d
    while len(wps) < 7:
        x, y = rng.uniform(minx - reach, maxx + reach), rng.uniform(miny - reach, maxy + reach)
        lo, la = plane(x, y, inverse=True)
        w = {"lat": round(la, 9), "lon": round(lo, 9)}
        dist = field.distance(Point(plane(w["lon"], w["lat"])))
        if abs(dist - d) < 0.05 or (warn and abs(dist - warn) < 0.05):
            continue
        wps.append(w)
        dists.append(dist)
    inp = {"area": area, "distance": f"{d} m", "waypoints": wps}
    if warn:
        inp["warning_distance"] = f"{warn} m"
    exp = {"ok": True, "result.outside_count": sum(x > d for x in dists)}
    tol = {"result.outside_count": {"abs": 0}}
    flagged = [(i + 1, x) for i, x in enumerate(dists) if x > d or (warn and x > warn)]
    for k, (idx, x) in enumerate(flagged):
        exp[f"result.flagged.{k}.waypoint"] = idx
        exp[f"result.flagged.{k}.beyond.value"] = x - d if x > d else x - warn
        tol[f"result.flagged.{k}.waypoint"] = {"abs": 0}
        tol[f"result.flagged.{k}.beyond.value"] = {"abs": 2e-3}
    if warn:
        exp["result.warning_count"] = sum(warn < x <= d for x in dists)
        tol["result.warning_count"] = {"abs": 0}
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": f"GEOS {shapely.geos_version_string}, PROJ {pyproj.proj_version_str}", "tolerance": tol})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
