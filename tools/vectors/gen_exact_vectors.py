#!/usr/bin/env python3
"""Strongly flattened ellipsoid vectors (|f| > 0.02, the exact method) for
navigation.geodesic.inverse and direct, from GeographicLib's GeodSolve -E
(GeodesicExact). Appended to the tools' vector files; vectors already there
(same input) are left alone, so a rerun never rewrites a published vector.
Numbers go in fixed-point because GeographicLib reads an exponent's "e" as a
hemisphere."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = "GeographicLib GeodSolve -E (GeodesicExact), custom ellipsoids"
SPEC = "add-navigation-and-geometry high-flattening scenario (GeodSolve -E)"
VER = "GeographicLib 2.x"
A = 6378137.0

INVERSE = [(0, 0, 10, 10, 10), (40.6413, -73.7781, 51.47, -0.4543, 10), (-33.9, 151.2, 35.7, 139.7, 5),
           (60, -150, -20, 30, 3), (5, 5, 5, 60, 2), (-80, 0, -10, 170, 40)]
DIRECT = [(40.6413, -73.7781, 51, 1000e3, 10), (0, 0, 45, 5000e3, 5), (-50, 120, 200, 8000e3, 3), (70, 10, 300, 3000e3, 2), (10, -60, 90, 12000e3, 40)]


def append(tool, new):
    path = ROOT / f"core/vectors/{tool}.jsonl"
    lines = path.read_text().splitlines()
    have = [json.loads(l)["input"] for l in lines]
    added = 0
    for inp, exp, tol, src in new:
        if inp in have:
            continue
        exp["ok"] = True
        lines.append(json.dumps({"id": f"v{len(lines) + 1:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": VER, "tolerance": tol},
                                ensure_ascii=False, separators=(",", ":")))
        added += 1
    path.write_text("\n".join(lines) + "\n")
    print(tool, "appended", added)


def main():
    inv = []
    for i, (la1, lo1, la2, lo2, rf) in enumerate(INVERSE):
        out = subprocess.run(["GeodSolve", "-i", "-E", "-p", "12", "-e", f"{A}", f"{1 / rf:.17f}"], input=f"{la1:.12f} {lo1:.12f} {la2:.12f} {lo2:.12f}\n",
                             capture_output=True, text=True, check=True).stdout.split()
        az1, az2, s12 = float(out[0]), float(out[1]), float(out[2])
        inp = {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2, "a": "6378137 m", "inverse_flattening": rf}
        exp = {"result.distance.value": s12 / 1000, "result.azimuth1.value": az1 % 360, "result.azimuth2.value": az2 % 360}
        tol = {"result.distance.value": {"rel": 1e-12, "abs": 1e-9}, "result.azimuth1.value": {"abs": 1e-9}, "result.azimuth2.value": {"abs": 1e-9}}
        inv.append((inp, exp, tol, SPEC if i == 0 else SRC))
    append("navigation.geodesic.inverse", inv)
    dire = []
    for la1, lo1, az, s, rf in DIRECT:
        out = subprocess.run(["GeodSolve", "-E", "-p", "12", "-e", f"{A}", f"{1 / rf:.17f}"], input=f"{la1:.12f} {lo1:.12f} {az:.12f} {s:.6f}\n",
                             capture_output=True, text=True, check=True).stdout.split()
        la2, lo2, az2 = float(out[0]), float(out[1]), float(out[2])
        inp = {"lat1": la1, "lon1": lo1, "azimuth": az, "distance": f"{s / 1000:g} km", "a": "6378137 m", "inverse_flattening": rf}
        exp = {"result.lat2.value": la2, "result.lon2.value": lo2, "result.azimuth2.value": az2 % 360}
        tol = {k: {"abs": 1e-10} for k in exp}
        dire.append((inp, exp, tol, SRC))
    append("navigation.geodesic.direct", dire)


if __name__ == "__main__":
    main()
