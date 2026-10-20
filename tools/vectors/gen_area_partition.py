#!/usr/bin/env python3
"""Golden vectors for survey.cogo.area-partition, worked with Shapely (GEOS
half-plane intersection, line substring, and polygon area) and SciPy's brentq,
independent of the core's clip and Brent code. Prints JSON lines; writes
nothing.

    python3 tools/vectors/gen_area_partition.py > core/vectors/survey.cogo.area-partition.jsonl

Needs shapely and scipy (a venv is fine)."""
import json
import math
import random

from scipy.optimize import brentq
from shapely import ops
from shapely.geometry import LineString, Point, Polygon

SRC = "Shapely (GEOS) half-plane clip and line substring with SciPy brentq (tools/vectors/gen_area_partition.py)"
VER = "Shapely 2.2, SciPy 1.18"


def star(rng, n, r):
    angles = sorted(rng.uniform(0, 2 * math.pi) for _ in range(n))
    cx, cy = rng.uniform(1000, 9000), rng.uniform(1000, 9000)
    return [(round(cx + r * rng.uniform(0.6, 1.0) * math.cos(a), 2),
             round(cy + r * rng.uniform(0.6, 1.0) * math.sin(a), 2)) for a in angles]


def halfplane(o, d, nrm, lo, t, big):
    corners = [(-big, lo - big), (big, lo - big), (big, t), (-big, t)]
    return Polygon([(o[0] + a * d[0] + b * nrm[0], o[1] + a * d[1] + b * nrm[1]) for a, b in corners])


def fixed_direction(pts, k, want, d, bearing_case):
    poly = Polygon(pts)
    sigma = 1.0 if poly.exterior.is_ccw else -1.0
    n = len(pts)
    nrm = (-d[1] * sigma, d[0] * sigma)
    o = pts[k]
    s = lambda q: nrm[0] * (q[0] - o[0]) + nrm[1] * (q[1] - o[1])
    if bearing_case:
        sv = [s(q) for q in pts]
        mid = (sv[k] + sv[(k + 1) % n]) / 2
        if mid - min(sv) > max(sv) - mid:
            nrm = (-nrm[0], -nrm[1])
    sv = [s(q) for q in pts]
    lo, hi = min(sv), max(sv)
    big = 10 * (hi - lo + poly.length)
    t = brentq(lambda t: poly.intersection(halfplane(o, d, nrm, lo, t, big)).area - want, lo, hi, xtol=1e-13, rtol=1e-15)
    line = LineString([(o[0] - big * d[0] + t * nrm[0], o[1] - big * d[1] + t * nrm[1]),
                       (o[0] + big * d[0] + t * nrm[0], o[1] + big * d[1] + t * nrm[1])])
    hit = poly.exterior.intersection(line)
    ends = list(hit.geoms) if hasattr(hit, "geoms") else [hit]
    if len(ends) != 2 or not all(isinstance(e, Point) for e in ends):
        return None
    return t, ends[0].distance(ends[1])


def through_point(pts, k, dist, want):
    n = len(pts)
    a, b = pts[k], pts[(k + 1) % n]
    L = math.dist(a, b)
    p = (a[0] + (b[0] - a[0]) * dist / L, a[1] + (b[1] - a[1]) * dist / L)
    sigma = 1.0 if Polygon(pts).exterior.is_ccw else -1.0
    ring = LineString([p] + [pts[(k + m) % n] for m in range(1, n + 1)] + [p])
    def area(sv):
        sub = ops.substring(ring, 0, sv)
        c = list(sub.coords)
        if len(c) < 3:
            return 0.0
        pol = Polygon(c)
        return pol.area if (pol.exterior.is_ccw == (sigma > 0)) else -pol.area
    sv = brentq(lambda x: area(x) - want, 0, ring.length, xtol=1e-13, rtol=1e-15)
    q = ring.interpolate(sv)
    chord = LineString([p, (q.x, q.y)])
    if not Polygon(pts).buffer(1e-9).covers(chord):
        return None
    return q.x, q.y, chord.length


def pts_json(pts):
    return [{"northing": y, "easting": x} for x, y in pts]


def main():
    rng = random.Random(20261020)
    out = []
    i = 0
    while len(out) < 18:
        i += 1
        n = rng.randint(3, 8)
        pts = star(rng, n, rng.uniform(100, 800))
        poly = Polygon(pts)
        if not poly.is_valid:
            continue
        k = rng.randrange(n)
        want = round(poly.area * rng.uniform(0.1, 0.9), 2)
        method = ["parallel", "through-point", "bearing"][len(out) % 3]
        inp = {"points": pts_json(pts), "area": f"{want} ft2", "method": method, "side": k + 1}
        exp = {"ok": True, "result.total_area.value": poly.area, "result.cut_area.value": want,
               "result.remainder_area.value": poly.area - want}
        tol = {"result.total_area.value": {"rel": 1e-12, "abs": 1e-6},
               "result.cut_area.value": {"abs": 5e-4}, "result.remainder_area.value": {"abs": 5e-4}}
        if method == "through-point":
            L = math.dist(pts[k], pts[(k + 1) % n])
            dist = round(rng.uniform(0, L), 2)
            r = through_point(pts, k, dist, want)
            if r is None:
                continue
            inp["distance"] = f"{dist} ft"
            exp.update({"result.end_easting.value": r[0], "result.end_northing.value": r[1], "result.length.value": r[2]})
            tol.update({f"result.{f}.value": {"abs": 1e-5} for f in ("end_easting", "end_northing", "length")})
        else:
            if method == "parallel":
                a, b = pts[k], pts[(k + 1) % n]
                L = math.dist(a, b)
                d = ((b[0] - a[0]) / L, (b[1] - a[1]) / L)
            else:
                deg = rng.randint(0, 89)
                mins = rng.randint(0, 59)
                quad = rng.choice(["N {} E", "S {} E", "S {} W", "N {} W"])
                inp["bearing"] = quad.format(f"{deg} {mins:02d} 00")
                ang = deg + mins / 60
                az = {"N {} E": ang, "S {} E": 180 - ang, "S {} W": 180 + ang, "N {} W": 360 - ang}[quad]
                d = (math.sin(math.radians(az)), math.cos(math.radians(az)))
            r = fixed_direction(pts, k, want, d, method == "bearing")
            if r is None:
                continue
            exp["result.length.value"] = r[1]
            tol["result.length.value"] = {"abs": 1e-5}
            if method == "parallel":
                exp["result.offset.value"] = r[0]
                tol["result.offset.value"] = {"abs": 1e-6}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    sq = [{"northing": 0, "easting": 0}, {"northing": 0, "easting": 100}, {"northing": 100, "easting": 100}, {"northing": 100, "easting": 0}]
    refusals = [
        ({"points": sq, "area": "12000 ft2", "side": 1}, "NO_SOLUTION", "/area"),
        ({"points": [dict(sq[0], radius="50 ft")] + sq[1:], "area": "2000 ft2", "side": 1}, "DEGENERATE_GEOMETRY", "/points/0/radius"),
        ({"points": [sq[0], sq[2], sq[1], sq[3]], "area": "2000 ft2", "side": 1}, "DEGENERATE_GEOMETRY", "/points"),
        ({"points": sq, "area": "2000 ft2", "method": "bearing", "side": 1}, "INVALID_INPUT", "/bearing"),
    ]
    for inp, code, field in refusals:
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"ok": False, "error.code": code, "error.field": field},
                    "source": SRC, "sourceVersion": VER, "tolerance": {}})
    for v in out:
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
