#!/usr/bin/env python3
"""Golden vectors for geometry.sampling.systematic-points and
stratified-points: the methods in their derivation notes reimplemented in
Python, with PROJ's transverse Mercator (pyproj) for the plane and its scale
factor, and the stream and triangulation of gen_random_points.py. Prints JSON
lines; writes nothing.

    python3 tools/vectors/gen_grid_designs.py systematic > core/vectors/geometry.sampling.systematic-points.jsonl
    python3 tools/vectors/gen_grid_designs.py stratified > core/vectors/geometry.sampling.stratified-points.jsonl

Needs pyproj (a venv is fine)."""
import importlib.util
import json
import math
import os
import random
import sys

import pyproj

_spec = importlib.util.spec_from_file_location("rp", os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen_random_points.py"))
rp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rp)

VER = f"pyproj {pyproj.__version__}, PROJ {pyproj.proj_version_str}"


def plane(rings_ll):
    outline = rings_ll[0]
    lat_c = sum(p[0] for p in outline) / len(outline)
    lon_c = sum(p[1] for p in outline) / len(outline)
    cm = 6 * rp.zone(lat_c, lon_c) - 183
    crs = f"+proj=tmerc +lon_0={cm} +k=0.9996 +x_0=0 +y_0=0 +ellps=WGS84"
    tm = pyproj.Transformer.from_crs("EPSG:4326", crs, always_xy=True)
    scale = pyproj.Proj(crs).get_factors(lon_c, lat_c).meridional_scale
    rings = []
    for k, r in enumerate(rings_ll):
        xy = [tm.transform(lon, lat) for lat, lon in r]
        twice = sum(xy[i][0] * xy[(i + 1) % len(xy)][1] - xy[(i + 1) % len(xy)][0] * xy[i][1] for i in range(len(xy)))
        if (k == 0) != (twice > 0):
            xy.reverse()
        rings.append(xy)
    back = lambda p: tuple(reversed(tm.transform(*p, direction="INVERSE")))
    return rings, scale, back


def inside(rings, p):
    c = False
    for r in rings:
        for i in range(len(r)):
            a, b = r[i], r[(i + 1) % len(r)]
            if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
                c = not c
    return c


def turn(p, o, t):
    s, c = math.sin(t), math.cos(t)
    x, y = p[0] - o[0], p[1] - o[1]
    return (o[0] + x * c + y * s, o[1] - x * s + y * c)


def systematic(rings_ll, d, tri, random_start, rot, seed):
    rings, scale, back = plane(rings_ll)
    dp = d * scale
    row = dp * math.sqrt(3) / 2 if tri else dp
    o = (min(p[0] for p in rings[0]), min(p[1] for p in rings[0]))
    t = math.radians(rot)
    framed = [turn(p, o, -t) for p in rings[0]]
    fx0, fy0 = min(p[0] for p in framed), min(p[1] for p in framed)
    fx1, fy1 = max(p[0] for p in framed), max(p[1] for p in framed)
    if random_start:
        g = rp.Xoshiro(seed)
        ux, uy = g.f64() * dp, g.f64() * row
    else:
        ux = uy = 0.0
    cols, rows = math.ceil((fx1 - fx0) / dp) + 2, math.ceil((fy1 - fy0) / row) + 2
    pts = []
    for j in range(rows):
        y = fy0 + uy + j * row
        shift = dp / 2 if tri and j % 2 == 1 else 0.0
        for i in range(cols):
            x = fx0 + ux + shift + i * dp - (dp if shift > 0 and ux + shift >= dp else 0.0)
            p = turn((x, y), o, t)
            if inside(rings, p):
                pts.append(back(p))
    return pts


def clip_box(poly, x0, y0, x1, y1):
    out = list(poly)
    for side, v in [(lambda p, v: p[0] - v, x0), (lambda p, v: v - p[0], x1), (lambda p, v: p[1] - v, y0), (lambda p, v: v - p[1], y1)]:
        inp, out = out, []
        for i in range(len(inp)):
            a, b = inp[i], inp[(i + 1) % len(inp)]
            sa, sb = side(a, v), side(b, v)
            if sa >= 0:
                out.append(a)
            if (sa >= 0) != (sb >= 0):
                f = sa / (sa - sb)
                out.append((a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])))
        if len(out) < 3:
            return []
    return out


def stratified(rings_ll, s, k, seed):
    rings, scale, back = plane(rings_ll)
    sp = s * scale
    x0, y0 = min(p[0] for p in rings[0]), min(p[1] for p in rings[0])
    x1, y1 = max(p[0] for p in rings[0]), max(p[1] for p in rings[0])
    cols, rows = max(math.ceil((x1 - x0) / sp), 1), max(math.ceil((y1 - y0) / sp), 1)
    tris = rp.triangulate(rings)
    bucket = [[] for _ in range(cols * rows)]
    span = lambda v, lo, n: min(max(math.floor((v - lo) / sp), 0), n - 1)
    for ti, t in enumerate(tris):
        ca, cb = span(min(p[0] for p in t), x0, cols), span(max(p[0] for p in t), x0, cols)
        ra, rb = span(min(p[1] for p in t), y0, rows), span(max(p[1] for p in t), y0, rows)
        for r in range(ra, rb + 1):
            for c in range(ca, cb + 1):
                bucket[r * cols + c].append(ti)
    g = rp.Xoshiro(seed)
    pts, strata = [], 0
    for r in range(rows):
        for c in range(cols):
            bx0, by0 = x0 + c * sp, y0 + r * sp
            parts = []
            for ti in bucket[r * cols + c]:
                piece = clip_box(tris[ti], bx0, by0, bx0 + sp, by0 + sp)
                for i in range(1, len(piece) - 1):
                    if rp.orient(piece[0], piece[i], piece[i + 1]) > 0:
                        parts.append((piece[0], piece[i], piece[i + 1]))
            cum, total = [], 0.0
            for a, b, cc in parts:
                total += rp.orient(a, b, cc) / 2
                cum.append(total)
            if total < scale * scale:
                continue
            strata += 1
            for _ in range(k):
                u = g.f64() * total
                ix = min(next((i for i, v in enumerate(cum) if v > u), len(cum)), len(parts) - 1)
                lo = cum[ix - 1] if ix else 0.0
                r1 = min(max((u - lo) / (cum[ix] - lo), 0.0), 1.0)
                r2 = g.f64()
                q = math.sqrt(r1)
                a, b, cc = parts[ix]
                p = ((1 - q) * a[0] + q * (1 - r2) * b[0] + q * r2 * cc[0], (1 - q) * a[1] + q * (1 - r2) * b[1] + q * r2 * cc[1])
                pts.append((*back(p), f"r{r + 1}c{c + 1}"))
    return pts, strata


def poly_json(shape):
    return [{"lat": lat, "lon": lon, **({"ring": k} if k else {})} for k, r in enumerate(shape) for lat, lon in r]


def main(which):
    rng = random.Random(20261021 if which == "systematic" else 20261022)
    src = f"Python reimplementation of the derivation note with PROJ tmerc (tools/vectors/gen_grid_designs.py {which})"
    out = []
    for i in range(20):
        shape = rp.SHAPES[i % len(rp.SHAPES)]
        rings, _, _ = plane(shape)
        span = max(max(p[0] for p in rings[0]) - min(p[0] for p in rings[0]), max(p[1] for p in rings[0]) - min(p[1] for p in rings[0]))
        seed = rng.choice([1, 42, rng.randrange(1 << 64)])
        if which == "systematic":
            tri = rng.random() < 0.5
            rand = rng.random() < 0.75
            rot = rng.choice([0, 0, 15, 30, 45, 100])
            d = round(span / rng.uniform(3, 7), 1)
            pts = systematic(shape, d, tri, rand, rot, seed)
            inp = {"polygon": poly_json(shape), "spacing": f"{d} m", "pattern": "triangular" if tri else "square", "start": "random" if rand else "corner", "seed": str(seed)}
            if rot:
                inp["rotation"] = f"{rot} deg"
            exp = {"ok": True, "result.count": len(pts)}
        else:
            s = round(span / rng.uniform(2, 4), 1)
            k = rng.randint(1, 3)
            pts, strata = stratified(shape, s, k, seed)
            inp = {"polygon": poly_json(shape), "cell": f"{s} m", "per_stratum": k, "seed": str(seed)}
            exp = {"ok": True, "result.count": len(pts), "result.strata": strata}
        tol = {f: {"abs": 0} for f in exp if f != "ok"}
        for j, p in enumerate(pts[:12]):
            for n, v in (("lat", p[0]), ("lon", p[1])):
                exp[f"result.points.{j}.{n}.value"] = v
                tol[f"result.points.{j}.{n}.value"] = {"abs": 1e-9}
            if which == "stratified":
                exp[f"result.points.{j}.stratum"] = p[2]
        out.append({"id": f"v{i + 1:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": VER, "tolerance": tol})
    big = [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0, "lon": -104.8826}, {"lat": 40.09, "lon": -104.8826}, {"lat": 40.09, "lon": -105.0}]
    refusal = ({"polygon": big, "spacing": "5 m"}, "/spacing") if which == "systematic" else ({"polygon": big, "cell": "10 m", "per_stratum": 1}, "/cell")
    out.append({"id": f"v{len(out) + 1:03d}", "input": refusal[0], "expect": {"ok": False, "error.code": "LIMIT_EXCEEDED", "error.field": refusal[1]}, "source": src, "sourceVersion": VER, "tolerance": {}})
    for v in out:
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main(sys.argv[1])
