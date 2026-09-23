#!/usr/bin/env python3
"""Golden vectors for the four navigation vector tools.

Two are plane trigonometry and two are geodetic, so they take two references:

- **polar-cartesian** and **operations** are exact arithmetic, computed here
  from the definitions. The one decision worth checking is the convention:
  navigational angles run clockwise from north (x = m sin theta, y = m cos
  theta), mathematical ones counterclockwise from east (x = m cos theta,
  y = m sin theta). Swapping them is the failure this tool exists to prevent,
  and both are covered.
- **distance-3d** and **look-angles** go through earth-centred coordinates, so
  the reference is PROJ: EPSG:4979 to EPSG:4978 is geodetic to geocentric on
  WGS 84. The east-north-up rotation at the observer is then the textbook
  matrix, applied here rather than read back from the core.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_nav_vector.py
"""
import json
import math
import sys
from pathlib import Path

from pyproj import Transformer

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
TRIG_SRC = "Exact trigonometry and vector algebra from the definitions (tools/vectors/gen_nav_vector.py)"
TRIG_VER = "definition"
ECEF_SRC = ("PROJ through pyproj, EPSG:4979 to EPSG:4978 (WGS 84 geodetic to geocentric), with the "
            "east-north-up rotation applied from its own definition (tools/vectors/gen_nav_vector.py)")
ECEF_VER = "pyproj 3.6.1 / PROJ 9.3.0"

TO_ECEF = Transformer.from_crs("EPSG:4979", "EPSG:4978", always_xy=True)


def ecef(lat, lon, h):
    x, y, z = TO_ECEF.transform(lon, lat, h)
    return (x, y, z)


def enu(olat, olon, oh, tlat, tlon, th):
    """(east, north, up) of a target seen from an observer, in metres."""
    ox, oy, oz = ecef(olat, olon, oh)
    tx, ty, tz = ecef(tlat, tlon, th)
    dx, dy, dz = tx - ox, ty - oy, tz - oz
    p, l = math.radians(olat), math.radians(olon)
    east = -math.sin(l) * dx + math.cos(l) * dy
    north = -math.sin(p) * math.cos(l) * dx - math.sin(p) * math.sin(l) * dy + math.cos(p) * dz
    up = math.cos(p) * math.cos(l) * dx + math.cos(p) * math.sin(l) * dy + math.sin(p) * dz
    return east, north, up


# (magnitude, direction deg, convention, elevation deg or None)
POLAR = [
    (10.0, 0.0, None, None), (10.0, 45.0, None, None), (10.0, 180.0, None, None),
    (10.0, 270.0, None, None), (10.0, 359.0, None, None),
    (10.0, 0.0, "mathematical", None), (10.0, 45.0, "mathematical", None),
    (10.0, 180.0, "mathematical", None), (10.0, 270.0, "mathematical", None),
    (25.0, 30.0, None, 0.0), (25.0, 30.0, None, 45.0), (25.0, 30.0, None, -30.0),
    (25.0, 30.0, None, 90.0), (100.0, 123.456, None, 12.5),
    (1.0, 90.0, "mathematical", 60.0), (0.5, 200.0, None, -75.0),
]

# Lists of vectors to add, each a list of (x, y, z or None)
OPS = [
    [(1.0, 0.0, None), (0.0, 1.0, None)],
    [(3.0, 4.0, None), (-3.0, -4.0, None)],
    [(1.0, 2.0, None), (3.0, 4.0, None), (5.0, 6.0, None)],
    [(1.0, 0.0, 0.0), (0.0, 0.0, 1.0)],
    [(2.0, -3.0, 6.0), (1.0, 2.0, -2.0)],
    [(0.0, 0.0, 0.0), (5.0, 12.0, None)],
    [(-7.0, 24.0, None), (7.0, -24.0, None)],
    [(1.0, 1.0, 1.0), (1.0, 1.0, 1.0)],
    [(10.0, 0.0, 0.0), (0.0, 10.0, 0.0), (0.0, 0.0, 10.0)],
    [(6.0, 8.0, None), (8.0, -6.0, None)],
    [(1.5, -2.5, 3.5), (-1.5, 2.5, -3.5)],
    [(100.0, 0.0, None), (0.0, 0.0001, None)],
]

# (lat1, lon1, h1, lat2, lon2, h2)
PAIRS_3D = [
    (40.0, -105.0, 1600.0, 40.1, -105.0, 1700.0),
    (40.0, -105.0, 0.0, 40.0, -105.0, 1000.0),
    (0.0, 0.0, 0.0, 0.0, 1.0, 0.0),
    (51.47, -0.4543, 25.0, 51.5074, -0.1278, 310.0),
    (-33.8688, 151.2093, 58.0, -33.9399, 151.1753, 6.0),
    (35.6762, 139.6503, 40.0, 35.5494, 139.7798, 5.0),
    (89.0, 0.0, 100.0, 89.0, 180.0, 100.0),
    (25.2048, 55.2708, 828.0, 25.2532, 55.3657, 10.0),
    (-22.9068, -43.1729, 710.0, -22.8099, -43.2436, 5.0),
    (64.1466, -21.9426, 50.0, 63.9850, -22.6056, 40.0),
    (1.3521, 103.8198, 100.0, 1.3644, 103.9915, 7.0),
    (60.0, 5.0, 300.0, 60.5, 7.0, 50.0),
]

# (observer lat, lon, height, target lat, lon, height)
LOOKS = [
    (40.0, -105.0, 10.0, 40.0, -104.0, 3000.0),
    (40.0, -105.0, 10.0, 41.0, -105.0, 10000.0),
    (40.0, -105.0, 1600.0, 39.5, -105.5, 4000.0),
    (0.0, 0.0, 0.0, 0.0, 0.5, 5000.0),
    (51.5, -0.12, 20.0, 51.9, -0.12, 11000.0),
    (-33.87, 151.21, 30.0, -34.2, 150.7, 8000.0),
    (35.68, 139.69, 50.0, 35.68, 140.69, 12000.0),
    (89.5, 0.0, 5.0, 89.0, 90.0, 2000.0),
    (25.2, 55.27, 5.0, 25.6, 55.9, 9000.0),
    (-45.0, 170.0, 100.0, -44.0, 171.0, 6000.0),
    (19.43, -99.13, 2240.0, 19.9, -99.6, 7000.0),
    (64.15, -21.94, 20.0, 63.9, -22.6, 3000.0),
]
# The mean radius this module uses for the horizon. 6,371,008.8 m is the
# usual mean radius of WGS 84 and is NOT what is used here: the difference is
# 8.8 m, which moves the horizon angle by a part in a million and shows up
# immediately at a tolerance of 1e-9 degrees.
EARTH_R = 6371000.0


def polar_rows(start):
    rows = []
    for i, (m, d, conv, el) in enumerate(POLAR, start=start + 1):
        e = math.radians(el or 0.0)
        th = math.radians(d)
        if conv == "mathematical":
            x, y = m * math.cos(e) * math.cos(th), m * math.cos(e) * math.sin(th)
        else:
            x, y = m * math.cos(e) * math.sin(th), m * math.cos(e) * math.cos(th)
        inp = {"magnitude": m, "direction": f"{d:g} deg"}
        expect = {"ok": True, "result.x": x, "result.y": y, "result.magnitude": m}
        tol = {"result.x": {"abs": 1e-9}, "result.y": {"abs": 1e-9},
               "result.magnitude": {"abs": 1e-9}}
        if conv:
            inp["convention"] = conv
        if el is not None:
            inp["elevation"] = f"{el:g} deg"
            expect["result.z"] = m * math.sin(e)
            tol["result.z"] = {"abs": 1e-9}
        rows.append((i, inp, expect, tol, TRIG_SRC, TRIG_VER))
    return rows


def ops_rows(start):
    rows = []
    for i, vecs in enumerate(OPS, start=start + 1):
        three = any(v[2] is not None for v in vecs)
        sx = sum(v[0] for v in vecs)
        sy = sum(v[1] for v in vecs)
        sz = sum(v[2] or 0.0 for v in vecs)
        mag = math.sqrt(sx * sx + sy * sy + sz * sz)
        payload = []
        for x, y, z in vecs:
            v = {"x": x, "y": y}
            if three:
                v["z"] = z or 0.0
            payload.append(v)
        expect = {"ok": True, "result.x": sx, "result.y": sy, "result.magnitude": mag}
        tol = {"result.x": {"abs": 1e-12}, "result.y": {"abs": 1e-12},
               "result.magnitude": {"abs": 1e-12}}
        if three:
            expect["result.z"] = sz
            tol["result.z"] = {"abs": 1e-12}
        if len(vecs) == 2:
            a, b = vecs
            az, bz = a[2] or 0.0, b[2] or 0.0
            expect["result.dot"] = a[0] * b[0] + a[1] * b[1] + az * bz
            tol["result.dot"] = {"abs": 1e-12}
        rows.append((i, {"vectors": payload}, expect, tol, TRIG_SRC, TRIG_VER))
    return rows


# `hae` is height above the ellipsoid; the other choice is `msl`, height above
# mean sea level, which the tool converts through the geoid first.
def distance_rows(start):
    rows = []
    for i, (la1, lo1, h1, la2, lo2, h2) in enumerate(PAIRS_3D, start=start + 1):
        p1, p2 = ecef(la1, lo1, h1), ecef(la2, lo2, h2)
        slant = math.dist(p1, p2)
        e, n, u = enu(la1, lo1, h1, la2, lo2, h2)
        rows.append((i, {"lat1": la1, "lon1": lo1, "height1": f"{h1:g} m",
                         "lat2": la2, "lon2": lo2, "height2": f"{h2:g} m",
                         "reference1": "hae", "reference2": "hae"},
                     {"ok": True, "result.slant_range.value": slant,
                      "result.height_difference.value": h2 - h1,
                      "result.elevation_angle.value": math.degrees(math.atan2(u, math.hypot(e, n)))},
                     {"result.slant_range.value": {"abs": 1e-6},
                      "result.height_difference.value": {"abs": 1e-9},
                      "result.elevation_angle.value": {"abs": 1e-9}},
                     ECEF_SRC, ECEF_VER))
    return rows


def look_rows(start):
    rows = []
    for i, (ola, olo, oh, tla, tlo, th) in enumerate(LOOKS, start=start + 1):
        e, n, u = enu(ola, olo, oh, tla, tlo, th)
        rows.append((i, {"observer_lat": ola, "observer_lon": olo, "observer_height": f"{oh:g} m",
                         "target_lat": tla, "target_lon": tlo, "target_height": f"{th:g} m"},
                     {"ok": True,
                      # Wrap into [0, 360). Due north comes out as a tiny
                      # negative angle whose remainder rounds up to exactly
                      # 360.0, which is the one value the range excludes.
                      "result.azimuth.value": math.degrees(math.atan2(e, n)) % 360.0 % 360.0
                      if math.degrees(math.atan2(e, n)) % 360.0 < 360.0 else 0.0,
                      "result.elevation.value": math.degrees(math.atan2(u, math.hypot(e, n))),
                      "result.slant_range.value": math.sqrt(e * e + n * n + u * u),
                      # The geometric horizon from an observer h above a sphere
                      # of the Earth's mean radius.
                      "result.horizon_elevation.value": -math.degrees(
                          math.acos(EARTH_R / (EARTH_R + oh))
                      )},
                     {"result.azimuth.value": {"abs": 1e-9},
                      "result.elevation.value": {"abs": 1e-9},
                      "result.slant_range.value": {"abs": 1e-6},
                      "result.horizon_elevation.value": {"abs": 1e-9}},
                     ECEF_SRC, ECEF_VER))
    return rows


def main():
    plan = [
        ("navigation.vector.polar-cartesian", polar_rows),
        ("navigation.vector.operations", ops_rows),
        ("navigation.vector.distance-3d", distance_rows),
        ("navigation.vector.look-angles", look_rows),
    ]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 10:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol, src, ver in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": src, "sourceVersion": ver, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
