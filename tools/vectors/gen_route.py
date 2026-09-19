#!/usr/bin/env python3
"""Golden vectors for navigation.route.cross-track, computed with Karney's own
Python geographiclib (pip install geographiclib): the closest point on the
geodesic A→B is where the course to P is perpendicular to the line, found by
bisection after a coarse scan; independent of the core's gnomonic method.

Writes core/vectors/navigation.route.cross-track.jsonl.
"""
import json
import math
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/navigation.route.cross-track.jsonl"
SRC = "Karney's geographiclib (Python): perpendicularity bisection along the geodesic"
VER = "geographiclib 2.1"


def closest(a, b, p):
    line = G.InverseLine(a[0], a[1], b[0], b[1])
    L = line.s13

    def ahead(s):
        x = line.Position(s)
        to_p = G.Inverse(x["lat2"], x["lon2"], p[0], p[1])["azi1"]
        return math.cos(math.radians(to_p - x["azi2"]))

    # Coarse scan for the sign change, then bisection.
    grid = [L * (k / 400 * 2 - 0.5) for k in range(401)]
    for s0, s1 in zip(grid, grid[1:]):
        if ahead(s0) > 0 >= ahead(s1):
            lo, hi = s0, s1
            break
    else:
        raise ValueError("no foot in range")
    for _ in range(80):
        mid = (lo + hi) / 2
        if ahead(mid) > 0:
            lo = mid
        else:
            hi = mid
    s = (lo + hi) / 2
    x = line.Position(s)
    xt = G.Inverse(x["lat2"], x["lon2"], p[0], p[1])["s12"]
    # Right of course when P is clockwise from the line direction.
    to_p = G.Inverse(x["lat2"], x["lon2"], p[0], p[1])["azi1"]
    side = math.sin(math.radians(to_p - x["azi2"]))
    return s, (xt if side > 0 else -xt), x["lat2"], x["lon2"], L


def main():
    rnd = random.Random(99)
    cases = [((40.6413, -73.7781), (51.47, -0.4543), (44.0, -60.0))]
    for _ in range(21):
        a = (rnd.uniform(-70, 70), rnd.uniform(-180, 180))
        d = G.Direct(a[0], a[1], rnd.uniform(0, 360), rnd.uniform(50e3, 2000e3))
        b = (d["lat2"], d["lon2"])
        f = G.Direct(a[0], a[1], d["azi1"], d["s12"] * rnd.uniform(-0.3, 1.3))
        pp = G.Direct(f["lat2"], f["lon2"], f["azi2"] + rnd.choice([90, -90]), rnd.uniform(0, 300e3))
        cases.append((a, b, (pp["lat2"], pp["lon2"])))
    out = []
    for i, (a, b, p) in enumerate(cases, 1):
        s, xt, flat, flon, L = closest(a, b, p)
        exp = {"result.cross_track.value": xt / 1000, "result.along_track.value": s / 1000,
               "result.foot_lat.value": flat, "result.foot_lon.value": flon, "result.segment.value": L / 1000,
               "result.within": "yes" if 0 <= s <= L else "no", "ok": True}
        tol = {k: {"abs": 1e-6} for k in ("result.cross_track.value", "result.along_track.value", "result.segment.value")}
        tol.update({k: {"abs": 1e-9} for k in ("result.foot_lat.value", "result.foot_lon.value")})
        out.append({"id": f"v{i:03d}", "input": {"lat1": a[0], "lon1": a[1], "lat2": b[0], "lon2": b[1], "lat": p[0], "lon": p[1]},
                    "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    OUT.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    print(len(out), "vectors ->", OUT.name)


if __name__ == "__main__":
    main()
