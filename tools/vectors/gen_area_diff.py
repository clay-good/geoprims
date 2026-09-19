#!/usr/bin/env python3
"""Polygon-area differential data and golden vectors from GeographicLib's
Planimeter (Karney's geodesic polygon area), WGS 84.

Writes core/crates/gp-geometry/tests/data/planimeter_diff.txt (500 seeded
polygons: fields to continents, clockwise and counterclockwise, across the
antimeridian, and rings around a pole) and core/vectors/geometry.area.polygon.jsonl.
Each polygon in the data file is a line "n perimeter signed_area" from
`Planimeter -p 9` followed by its n corners. Requires Planimeter on PATH.
"""
import json
import math
import random
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "core/crates/gp-geometry/tests/data/planimeter_diff.txt"
VECTORS = ROOT / "core/vectors/geometry.area.polygon.jsonl"


def planimeter(polys):
    # Polygons are separated by a blank line; Planimeter prints one result each.
    text = "\n\n".join("\n".join(f"{a!r} {b!r}" for a, b in p) for p in polys) + "\n"
    out = subprocess.run(["Planimeter", "-p", "9"], input=text, capture_output=True, text=True, check=True).stdout
    return [tuple(map(float, l.split())) for l in out.splitlines() if l.strip()]


def star(rnd, lat0, lon0, radius_deg, n, clockwise):
    """A non-convex ring around (lat0, lon0) with random radii."""
    pts = []
    for k in range(n):
        t = 2 * math.pi * k / n
        r = radius_deg * rnd.uniform(0.4, 1.0)
        pts.append((max(-89.9, min(89.9, lat0 + r * math.sin(t))), lon0 + r * math.cos(t) / max(0.1, math.cos(math.radians(lat0)))))
    return pts[::-1] if clockwise else pts


def polygons(rnd, count):
    out = []
    for i in range(count):
        kind = i % 5
        if kind == 3:  # across the antimeridian
            out.append(star(rnd, rnd.uniform(-60, 60), 180 + rnd.uniform(-2, 2), rnd.uniform(0.5, 5), rnd.randint(3, 12), rnd.random() < 0.5))
        elif kind == 4:  # a ring circling a pole
            lat = rnd.uniform(60, 89) * rnd.choice([-1, 1])
            n = rnd.randint(4, 16)
            ring = [(lat + rnd.uniform(-0.5, 0.5), -180 + 360 * k / n + rnd.uniform(-5, 5)) for k in range(n)]
            out.append(ring[::-1] if rnd.random() < 0.5 else ring)
        else:  # fields to continents
            size = 10 ** rnd.uniform(-3, 1.3)
            out.append(star(rnd, rnd.uniform(-80, 80), rnd.uniform(-180, 180), size, rnd.randint(3, 20), rnd.random() < 0.5))
    return out


def main():
    rnd = random.Random(20260919)
    polys = polygons(rnd, 500)
    res = planimeter(polys)
    assert len(res) == len(polys), (len(res), len(polys))
    lines = []
    for p, (n, per, area) in zip(polys, res):
        lines.append(f"{int(n)} {per!r} {area!r}")
        lines += [f"{a!r} {b!r}" for a, b in p]
    DATA.parent.mkdir(parents=True, exist_ok=True)
    DATA.write_text("\n".join(lines) + "\n")

    # Golden vectors: the spec scenarios, then Planimeter polygons.
    src, ver = "GeographicLib Planimeter (Karney 2013 geodesic polygon area), WGS 84", "GeographicLib 2.7"
    colorado = [(37, -109.05), (41, -109.05), (41, -102.05), (37, -102.05)]
    cap = [(80, -180 + 30 * k) for k in range(12)]
    outer = [(40, -105), (40, -104), (41, -104), (41, -105)]
    hole = [(40.4, -104.6), (40.6, -104.6), (40.6, -104.4), (40.4, -104.4)]
    cases = [colorado, cap, outer, hole] + rnd.sample(polys, 20)
    got = planimeter(cases)
    vs = []

    def vec(i, rows, exp, tol):
        e = dict(exp)
        e["ok"] = True
        return {"id": f"v{i:03d}", "input": {"polygon": rows}, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}

    tol = {"result.area.value": {"rel": 1e-8, "abs": 1e-9}, "result.perimeter.value": {"rel": 1e-10, "abs": 1e-9}}
    col = got[0]
    vs.append(vec(1, [{"lat": a, "lon": b} for a, b in colorado],
                  {"result.area.value": abs(col[2]) / 1e6, "result.perimeter.value": col[1] / 1000, "result.orientation": "clockwise", "result.pole": "none"}, tol))
    capr = got[1]
    vs.append(vec(2, [{"lat": a, "lon": b} for a, b in cap],
                  {"result.area.value": abs(capr[2]) / 1e6, "result.pole": "north", "meta.warnings.*.code": "POLE_ENCLOSED"}, {"result.area.value": tol["result.area.value"]}))
    o, h = got[2], got[3]
    vs.append(vec(3, [{"lat": a, "lon": b} for a, b in outer] + [{"lat": a, "lon": b, "ring": 1} for a, b in hole],
                  {"result.area.value": (abs(o[2]) - abs(h[2])) / 1e6, "result.perimeter.value": (o[1] + h[1]) / 1000, "result.holes": 1,
                   "result.outline_area.value": abs(o[2]) / 1e6}, {**tol, "result.outline_area.value": tol["result.area.value"], "result.holes": {"abs": 0}}))
    for k, (p, (n, per, area)) in enumerate(zip(cases[4:], got[4:]), 4):
        vs.append(vec(k, [{"lat": a, "lon": b} for a, b in p],
                      {"result.area.value": abs(area) / 1e6, "result.perimeter.value": per / 1000,
                       "result.orientation": "counterclockwise" if area > 0 else "clockwise"}, tol))
    VECTORS.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
    print(len(polys), "polygons ->", DATA.name, ";", len(vs), "vectors")


if __name__ == "__main__":
    main()
