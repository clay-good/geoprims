#!/usr/bin/env python3
"""More golden vectors for the last two geodesy tools that needed them.

Both already had GeographicLib as their reference; both were short of the
twenty the stable bar asks for. This adds the cases the first pass did not
reach, from the same references:

- `geodesy.projection.arc-to-chord`: GeoConvert for the grid coordinates and
  the zone convergence, GeodSolve for the geodesic azimuths and length, and
  t - T = atan2(dE, dN) - (azimuth - convergence) at each end. Lines that run
  north-south (where t - T is near zero), lines that straddle a central
  meridian, long lines, and both hemispheres.
- `geodesy.height.convert`: h = H + N, with N read from the committed EGM96
  fixture -- 2,010 geoid heights that GeographicLib's GeoidEval produced with
  the egm96-15 grid. The grid itself is not installed here, so rather than
  guess at N the values are taken from the run that is already in the
  repository, which is the same reference at one remove and is said so.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_geodesy_last.py
"""
import csv
import json
import math
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else REPO / "core/vectors"
EGM96 = REPO / "core/crates/gp-geodesy/tests/data/egm96_diff.csv"

ARC_SRC = "GeographicLib GeoConvert and GeodSolve (tools/vectors/gen_geodesy_last.py)"
ARC_VER = "GeographicLib 2.7"
H_SRC = ("h = H + N with N from GeographicLib's GeoidEval on the egm96-15 grid, as captured in "
         "core/crates/gp-geodesy/tests/data/egm96_diff.csv")
H_VER = "GeographicLib GeoidEval 2.7, egm96-15"


def run(args, line):
    return subprocess.run(args, input=line + "\n", capture_output=True, text=True, check=True).stdout.split()


def utm(lat, lon, z):
    o = run(["GeoConvert", "-u", "-z", str(z), "-p", "9"], f"{lat:.12f} {lon:.12f}")
    return float(o[1]), float(o[2])


def conv(lat, lon, z):
    return float(run(["GeoConvert", "-c", "-z", str(z), "-p", "12"], f"{lat:.12f} {lon:.12f}")[0])


def wrap(x):
    return (x + 180) % 360 - 180


# (lat1, lon1, lat2, lon2, zone)
ARC_CASES = [
    # Due north-south on the central meridian: t - T should vanish.
    (40.0, -81.0, 40.5, -81.0, 17),
    (40.0, -81.0, 41.0, -81.0, 17),
    # Due east-west, where it is largest for the length.
    (40.0, -81.5, 40.0, -80.5, 17),
    (45.0, -81.5, 45.0, -80.5, 17),
    # Straddling the central meridian, so the two ends have opposite signs.
    (40.0, -81.4, 40.3, -80.6, 17),
    # Far from the meridian, near the zone edge.
    (40.0, -78.2, 40.3, -78.0, 17),
    # Long lines.
    (39.0, -80.0, 41.0, -79.0, 17),
    (38.5, -81.5, 41.5, -79.5, 17),
    # High latitude, where the convergence grows.
    (60.0, 5.0, 61.0, 6.0, 32),
    (70.0, 25.0, 70.5, 26.0, 35),
    # Southern hemisphere.
    (-33.0, 151.0, -34.0, 150.5, 56),
    (-45.0, 170.0, -45.5, 170.8, 59),
    (-22.9, -43.2, -23.2, -43.0, 23),
    # Equatorial, where it is smallest.
    (0.0, 9.0, 0.5, 9.5, 32),
    (1.3, 103.8, 1.5, 104.0, 48),
    (10.0, -80.9, 10.3, -80.2, 17),
]

# (lat, lon, height, from) -- the heights are read off the EGM96 fixture.
HEIGHT_PICKS = 18
HEIGHTS = [0.0, 100.0, 250.5, 1000.0, -50.0, 3000.0]


def arc_rows(start):
    rows = []
    for i, (la1, lo1, la2, lo2, z) in enumerate(ARC_CASES, start=start + 1):
        (e1, n1), (e2, n2) = utm(la1, lo1, z), utm(la2, lo2, z)
        g1, g2 = conv(la1, lo1, z), conv(la2, lo2, z)
        o = run(["GeodSolve", "-i", "-p", "12"], f"{la1:.12f} {lo1:.12f} {la2:.12f} {lo2:.12f}")
        az1, az2, s12 = float(o[0]), float(o[1]), float(o[2])
        t1 = math.degrees(math.atan2(e2 - e1, n2 - n1))
        t2 = math.degrees(math.atan2(e1 - e2, n1 - n2))
        d1 = wrap(t1 - (az1 - g1)) * 3600
        d2 = wrap(t2 - (az2 + 180 - g2)) * 3600
        rows.append((i, {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2, "grid": "utm", "zone": str(z)},
                     {"result.t_minus_t_from.value": d1, "result.t_minus_t_to.value": d2,
                      "result.grid_distance.value": math.hypot(e2 - e1, n2 - n1),
                      "result.ellipsoid_distance.value": s12, "ok": True},
                     {"result.t_minus_t_from.value": {"abs": 1e-4},
                      "result.t_minus_t_to.value": {"abs": 1e-4},
                      "result.grid_distance.value": {"abs": 1e-5},
                      "result.ellipsoid_distance.value": {"abs": 1e-6}},
                     ARC_SRC, ARC_VER))
    return rows


def height_rows(start):
    with EGM96.open() as f:
        table = [r for r in csv.DictReader(f)]
    # Spread the picks across the file so they are not all in one region.
    step = max(1, len(table) // HEIGHT_PICKS)
    picks = [table[k * step] for k in range(HEIGHT_PICKS)]
    rows = []
    for i, row in enumerate(picks, start=start + 1):
        lat, lon, n = float(row["lat"]), float(row["lon"]), float(row["cubic"])
        h = HEIGHTS[i % len(HEIGHTS)]
        rows.append((i, {"lat": lat, "lon": lon, "height": f"{h:g} m"},
                     {"ok": True, "result.orthometric.value": h - n,
                      "result.ellipsoidal.value": h, "result.geoid_height.value": n},
                     # GeoidEval prints to four decimals, so the geoid height
                     # is known to half a tenth of a millimetre and no better.
                     {"result.orthometric.value": {"abs": 5e-5},
                      "result.ellipsoidal.value": {"abs": 1e-9},
                      "result.geoid_height.value": {"abs": 5e-5}},
                     H_SRC, H_VER))
    return rows


def main():
    plan = [("geodesy.projection.arc-to-chord", arc_rows), ("geodesy.height.convert", height_rows)]
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
