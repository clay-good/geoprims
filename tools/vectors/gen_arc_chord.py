#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.arc-to-chord in UTM, from
GeographicLib's command-line tools alone: GeoConvert -u -z for the grid
coordinates and -c for the convergence in the zone, GeodSolve -i for the
geodesic azimuths and length; t - T = atan2(dE, dN) - (azimuth - convergence)."""
import json
import math
import subprocess
import sys
from pathlib import Path

SRC, VER = "GeographicLib GeoConvert and GeodSolve (tools/vectors/gen_arc_chord.py)", "GeographicLib 2.x"


def run(args, line):
    return subprocess.run(args, input=line + "\n", capture_output=True, text=True, check=True).stdout.split()


def utm(lat, lon, z):
    o = run(["GeoConvert", "-u", "-z", str(z), "-p", "9"], f"{lat:.12f} {lon:.12f}")
    return float(o[1]), float(o[2])


def conv(lat, lon, z):
    return float(run(["GeoConvert", "-c", "-z", str(z), "-p", "12"], f"{lat:.12f} {lon:.12f}")[0])


def wrap(x):
    return (x + 180) % 360 - 180


def main():
    cases = [(40.44, -79.99, 40.52, -79.91, 17), (40.44, -79.99, 40.40, -79.50, 17), (60.0, 5.0, 60.5, 7.0, 32),
             (-33.9, 151.2, -34.2, 150.7, 56), (10.0, -80.9, 10.3, -80.2, 17), (45.0, -84.0, 45.0, -83.0, 16)]
    out = []
    for la1, lo1, la2, lo2, z in cases:
        (e1, n1), (e2, n2) = utm(la1, lo1, z), utm(la2, lo2, z)
        g1, g2 = conv(la1, lo1, z), conv(la2, lo2, z)
        o = run(["GeodSolve", "-i", "-p", "12"], f"{la1:.12f} {lo1:.12f} {la2:.12f} {lo2:.12f}")
        az1, az2, s12 = float(o[0]), float(o[1]), float(o[2])
        t1 = math.degrees(math.atan2(e2 - e1, n2 - n1))
        t2 = math.degrees(math.atan2(e1 - e2, n1 - n2))
        d1 = wrap(t1 - (az1 - g1)) * 3600
        d2 = wrap(t2 - (az2 + 180 - g2)) * 3600
        inp = {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2, "grid": "utm", "zone": str(z)}
        exp = {"result.t_minus_t_from.value": d1, "result.t_minus_t_to.value": d2, "result.grid_distance.value": math.hypot(e2 - e1, n2 - n1),
               "result.ellipsoid_distance.value": s12, "ok": True}
        tol = {"result.t_minus_t_from.value": {"abs": 1e-4}, "result.t_minus_t_to.value": {"abs": 1e-4},
               "result.grid_distance.value": {"abs": 1e-5}, "result.ellipsoid_distance.value": {"abs": 1e-6}}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
        print(round(d1, 4), round(d2, 4))
    out.append({"id": f"v{len(out) + 1:03d}", "input": {"lat1": 40, "lon1": -80, "lat2": 40.1, "lon2": -80, "grid": "spcs"},
                "expect": {"ok": False, "error.code": "INVALID_INPUT"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geodesy.projection.arc-to-chord.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
