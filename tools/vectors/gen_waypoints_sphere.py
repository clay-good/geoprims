#!/usr/bin/env python3
"""Appends great-circle vectors to navigation.geodesic.waypoints (path
"great-circle"), from GeographicLib's GeodSolve on a sphere of the IUGG mean
radius R1 (-e 6371008.771 0), where the geodesic is exactly the great circle.
An independent reference: the core uses its own spherical formulas.

Appends only ids not already in the file, so frozen vectors stay as pushed.
Usage: python3 tools/vectors/gen_waypoints_sphere.py
"""
import json
import subprocess
from pathlib import Path

R1 = "6371008.771"
FILE = Path(__file__).resolve().parents[2] / "core/vectors/navigation.geodesic.waypoints.jsonl"
VERSION = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.split("\n")[0].strip()


def geod(args, line):
    out = subprocess.run(["GeodSolve", "-e", R1, "0", "-p", "12", *args], input=line, capture_output=True, text=True, check=True)
    return [float(x) for x in out.stdout.split()]


CASES = [
    # id, input, which fractions of the length to check
    ("v023", {"lat1": 40.6413, "lon1": -73.7781, "lat2": 51.47, "lon2": -0.4543, "intervals": 10, "path": "great-circle"}),
    ("v024", {"lat1": -33.9461, "lon1": 151.1772, "lat2": 37.6213, "lon2": -122.379, "intervals": 8, "path": "great-circle"}),
    ("v025", {"lat1": 10, "lon1": 20, "lat2": -20, "lon2": 170, "intervals": 3, "path": "great-circle"}),
    ("v026", {"lat1": 61.17, "lon1": -150.0, "lat2": 59.65, "lon2": 17.92, "intervals": 4, "path": "great-circle"}),
]


def vector(vid, inp):
    azi1, _, s12 = geod(["-i"], f"{inp['lat1']} {inp['lon1']} {inp['lat2']} {inp['lon2']}")
    n = inp["intervals"]
    expect = {"ok": True, "result.count": n + 1, "result.length.value": s12 / 1000}
    tol = {"result.count": {"abs": 0}, "result.length.value": {"abs": 1e-9}}
    for i in sorted({0, n // 2, n}):
        lat, lon, azi = geod([], f"{inp['lat1']} {inp['lon1']} {azi1} {s12 * i / n}")
        for k, v in (("lat", lat), ("lon", lon), ("azimuth", azi % 360)):
            key = f"result.points.{i}.{k}.value"
            expect[key] = v
            tol[key] = {"abs": 1e-9}
    return {
        "id": vid,
        "input": inp,
        "expect": expect,
        "source": "GeographicLib GeodSolve on a sphere of radius R1 = 6,371,008.771 m (-e 6371008.771 0), where the geodesic is the great circle",
        "sourceVersion": VERSION,
        "tolerance": tol,
    }


have = {json.loads(l)["id"] for l in FILE.read_text().splitlines() if l.strip()}
with FILE.open("a") as f:
    for vid, inp in CASES:
        if vid not in have:
            f.write(json.dumps(vector(vid, inp), separators=(",", ":")) + "\n")
print(f"{FILE.name}: {len(FILE.read_text().splitlines())} vectors")
