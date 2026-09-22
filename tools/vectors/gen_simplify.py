#!/usr/bin/env python3
"""Golden vectors for geometry.simplify.rdp and geometry.simplify.visvalingam:
Douglas-Peucker and Visvalingam-Whyatt written out in plain Python on the
azimuthal equidistant plane from GeographicLib's GeodesicProj -z. Cases keep a
clear margin between the tolerance and every vertex's deviation, so the kept
vertices do not hinge on the plane's center; the deviation is compared at 5 cm."""
import json
import math
import subprocess
import sys
from pathlib import Path

SRC = "Douglas-Peucker (1973) and Visvalingam-Whyatt (1993) in Python on GeographicLib GeodesicProj -z (tools/vectors/gen_simplify.py)"
VER = "GeographicLib 2.x"


def ll(xy):
    """Local meters east and north of 40° N, 105° W, as vertices."""
    return [(40.0 + y / 111_034.0, -105.0 + x / 85_395.0) for x, y in xy]


def plane(pts):
    lat0 = sum(p[0] for p in pts) / len(pts)
    lon0 = sum(p[1] for p in pts) / len(pts)
    out = subprocess.run(["GeodesicProj", "-z", f"{lat0:.12f}", f"{lon0:.12f}", "-p", "9"],
                         input="".join(f"{a:.12f} {b:.12f}\n" for a, b in pts), capture_output=True, text=True, check=True).stdout
    return [tuple(float(v) for v in line.split()[:2]) for line in out.splitlines()]


def seg(a, b, p):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def rdp(xy, tol):
    keep = [False] * len(xy)
    keep[0] = keep[-1] = True
    stack = [(0, len(xy) - 1)]
    while stack:
        a, b = stack.pop()
        if b - a < 2:
            continue
        k, d = max(((k, seg(xy[a], xy[b], xy[k])) for k in range(a + 1, b)), key=lambda t: t[1])
        if d > tol:
            keep[k] = True
            stack += [(a, k), (k, b)]
    return keep


def vw(xy, target):
    alive = list(range(len(xy)))
    while len(alive) > target:
        areas = [(abs((xy[alive[j]][0] - xy[alive[j - 1]][0]) * (xy[alive[j + 1]][1] - xy[alive[j - 1]][1])
                      - (xy[alive[j]][1] - xy[alive[j - 1]][1]) * (xy[alive[j + 1]][0] - xy[alive[j - 1]][0])) / 2, j)
                 for j in range(1, len(alive) - 1)]
        alive.pop(min(areas)[1])
    return [i in alive for i in range(len(xy))]


def deviation(xy, keep):
    kept = [i for i, k in enumerate(keep) if k]
    worst = 0.0
    for a, b in zip(kept, kept[1:]):
        for k in range(a + 1, b):
            worst = max(worst, seg(xy[a], xy[b], xy[k]))
    return worst


def vec(i, inp, keep, pts, xy):
    kept = [p for p, k in zip(pts, keep) if k]
    exp = {"result.vertices_out": float(len(kept)), "result.vertices_in": float(len(pts)),
           "result.max_deviation.value": deviation(xy, keep)}
    for n, (la, lo) in enumerate(kept):
        exp[f"result.simplified.{n}.lat.value"] = la
        exp[f"result.simplified.{n}.lon.value"] = lo
    exp["ok"] = True
    tol = {k: ({"abs": 0.05} if "deviation" in k else {"abs": 1e-12}) for k, v in exp.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol}


LINES = [
    # A wavy 1.2 km track: two small wiggles and one 150 m bump.
    [(0, 0), (170, 17), (340, -14), (510, 25), (680, 0), (850, 150), (1020, 0), (1190, 3)],
    # A switchback road.
    [(0, 0), (400, 60), (0, 140), (400, 210), (0, 290), (10, 300), (400, 360)],
    # A long gentle curve sampled every 500 m.
    [(500 * i, 4000 * math.sin(i / 12)) for i in range(25)],
    # A coast with a narrow bay.
    [(0, 0), (300, 12), (600, -8), (650, -500), (700, -8), (1000, 10), (1300, 0)],
    # A near-straight line with growing noise (no two triangles tie).
    [(100 * i, 2.0 * ((-1) ** i) * (1 + i / 10)) for i in range(30)],
]


def rdp_vectors():
    out = []
    for i, (xy_local, tol) in enumerate(zip(LINES, [40, 30, 200, 50, 5]), 1):
        pts = ll(xy_local)
        xy = plane(pts)
        inp = {"points": [{"lat": a, "lon": b} for a, b in pts], "tolerance": f"{tol} m"}
        out.append(vec(i, inp, rdp(xy, tol), pts, xy))
    return out


def vw_vectors():
    out = []
    for i, (xy_local, target) in enumerate(zip(LINES, [4, 5, 8, 5, 2]), 1):
        pts = ll(xy_local)
        xy = plane(pts)
        inp = {"points": [{"lat": a, "lon": b} for a, b in pts], "target_vertices": target}
        out.append(vec(i, inp, vw(xy, target), pts, xy))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for tool, vs in {"geometry.simplify.rdp": rdp_vectors(), "geometry.simplify.visvalingam": vw_vectors()}.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
