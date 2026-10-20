#!/usr/bin/env python3
"""Golden vectors for geometry.sampling.random-points: the method in the
derivation note reimplemented in Python with the same seeded stream
(xoshiro256** from splitmix64, written here from the authors' reference C),
PROJ's transverse Mercator through pyproj in place of the core's, and
pyproj.Geod for the area. Prints JSON lines; writes nothing.

    python3 tools/vectors/gen_random_points.py > core/vectors/geometry.sampling.random-points.jsonl

Needs pyproj (a venv is fine)."""
import json
import math
import random

import pyproj

SRC = "Python reimplementation of the derivation note with PROJ tmerc and pyproj.Geod (tools/vectors/gen_random_points.py)"
VER = f"pyproj {pyproj.__version__}, PROJ {pyproj.proj_version_str}"
M64 = (1 << 64) - 1


class Xoshiro:
    def __init__(self, seed):
        x = seed
        s = []
        for _ in range(4):
            x = (x + 0x9E3779B97F4A7C15) & M64
            z = x
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
            s.append(z ^ (z >> 31))
        self.s = s

    def next(self):
        s = self.s
        rotl = lambda x, k: ((x << k) | (x >> (64 - k))) & M64
        result = (rotl((s[1] * 5) & M64, 7) * 9) & M64
        t = (s[1] << 17) & M64
        s[2] ^= s[0]
        s[3] ^= s[1]
        s[1] ^= s[2]
        s[0] ^= s[3]
        s[2] ^= t
        s[3] = rotl(s[3], 45)
        return result

    def f64(self):
        return (self.next() >> 11) * 2.0 ** -53


def orient(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def blocks(a, b, c, d):
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    if o1 * o2 < 0 and o3 * o4 < 0:
        return True
    within = lambda p, q, r: r != p and r != q and min(p[0], q[0]) <= r[0] <= max(p[0], q[0]) and min(p[1], q[1]) <= r[1] <= max(p[1], q[1])
    return (o1 == 0 and within(a, b, c)) or (o2 == 0 and within(a, b, d))


def triangulate(rings):
    outer = list(rings[0])
    holes = sorted(enumerate(rings[1:]), key=lambda h: (-max(p[0] for p in h[1]), h[0]))
    sides = lambda r: [(r[i], r[(i + 1) % len(r)]) for i in range(len(r))]
    for k, (_, hole) in enumerate(holes):
        m = 0
        for i in range(len(hole)):
            if hole[i][0] > hole[m][0]:
                m = i
        mp = hole[m]
        d2 = lambda p: (p[0] - mp[0]) ** 2 + (p[1] - mp[1]) ** 2
        order = sorted(range(len(outer)), key=lambda i: (d2(outer[i]), i))
        walls = sides(outer) + [w for _, h in holes[k:] for w in sides(h)]
        p = next(i for i in order if outer[i] != mp and all(not blocks(mp, outer[i], c, d) and not blocks(c, d, mp, outer[i]) for c, d in walls))
        outer = outer[: p + 1] + [hole[(m + j) % len(hole)] for j in range(len(hole) + 1)] + outer[p:]
    v = outer
    tris = []
    i = 0
    while len(v) > 3:
        n = len(v)
        for _ in range(n):
            a, b, c = v[(i + n - 1) % n], v[i], v[(i + 1) % n]
            if orient(a, b, c) > 0 and is_ear(v, a, b, c):
                tris.append((a, b, c))
                del v[i]
                i %= len(v)
                break
            i = (i + 1) % n
        else:
            raise RuntimeError("no ear")
    tris.append((v[0], v[1], v[2]))
    return tris


def is_ear(v, a, b, c):
    n = len(v)
    for j, p in enumerate(v):
        if p in (a, b, c):
            continue
        reflex = orient(v[(j + n - 1) % n], p, v[(j + 1) % n]) <= 0
        if reflex and orient(a, b, p) >= 0 and orient(b, c, p) >= 0 and orient(c, a, p) >= 0:
            return False
    return True


def zone(lat, lon):
    z = int((lon + 180) // 6) + 1
    if 56 <= lat < 64 and 3 <= lon < 12:
        z = 32
    return min(z, 60)


def sample(rings_ll, n, seed):
    outline = rings_ll[0]
    lat_c = sum(p[0] for p in outline) / len(outline)
    lon_c = sum(p[1] for p in outline) / len(outline)
    cm = 6 * zone(lat_c, lon_c) - 183
    tm = pyproj.Transformer.from_crs("EPSG:4326", f"+proj=tmerc +lon_0={cm} +k=0.9996 +x_0=0 +y_0=0 +ellps=WGS84", always_xy=True)
    rings = []
    for k, r in enumerate(rings_ll):
        xy = [tm.transform(lon, lat) for lat, lon in r]
        twice = sum(xy[i][0] * xy[(i + 1) % len(xy)][1] - xy[(i + 1) % len(xy)][0] * xy[i][1] for i in range(len(xy)))
        if (k == 0) != (twice > 0):
            xy.reverse()
        rings.append(xy)
    tris = triangulate(rings)
    cum, total = [], 0.0
    for a, b, c in tris:
        total += orient(a, b, c) / 2
        cum.append(total)
    g = Xoshiro(seed)
    pts = []
    for _ in range(n):
        u = g.f64() * total
        k = min(next((i for i, c in enumerate(cum) if c > u), len(cum)), len(tris) - 1)
        lo = cum[k - 1] if k else 0.0
        r1 = min(max((u - lo) / (cum[k] - lo), 0.0), 1.0)
        r2 = g.f64()
        s = math.sqrt(r1)
        a, b, c = tris[k]
        p = ((1 - s) * a[0] + s * (1 - r2) * b[0] + s * r2 * c[0], (1 - s) * a[1] + s * (1 - r2) * b[1] + s * r2 * c[1])
        lon, lat = tm.transform(*p, direction="INVERSE")
        pts.append((lat, lon))
    geod = pyproj.Geod(ellps="WGS84")
    area = abs(geod.polygon_area_perimeter([p[1] for p in outline], [p[0] for p in outline])[0])
    for h in rings_ll[1:]:
        area -= abs(geod.polygon_area_perimeter([p[1] for p in h], [p[0] for p in h])[0])
    return pts, area


SHAPES = [
    [[(40.0, -105.0), (40.0, -104.996), (40.003, -104.996), (40.003, -105.0)],
     [(40.001, -104.999), (40.002, -104.999), (40.002, -104.998), (40.001, -104.998)]],
    [[(40.0, -105.0), (40.0, -104.99), (40.004, -104.99), (40.004, -104.996), (40.01, -104.996), (40.01, -105.0)],
     [(40.001, -104.999), (40.003, -104.999), (40.003, -104.997), (40.001, -104.997)]],
    [[(-33.9, 151.2), (-33.9, 151.26), (-33.85, 151.27), (-33.82, 151.23), (-33.86, 151.21)]],
    [[(51.5, -0.2), (51.5, 0.0), (51.6, 0.0), (51.6, -0.2)],
     [(51.52, -0.18), (51.54, -0.18), (51.54, -0.15), (51.52, -0.15)],
     [(51.56, -0.08), (51.58, -0.08), (51.58, -0.03), (51.56, -0.03)]],
    [[(0.0, 9.0), (-0.02, 9.03), (0.02, 9.05), (0.03, 9.01)]],
]


def main():
    rng = random.Random(20261020)
    out = []
    for i in range(20):
        shape = SHAPES[i % len(SHAPES)]
        seed = rng.choice([0, 1, 42, 7, rng.randrange(1 << 64)])
        n = rng.randint(3, 12)
        pts, area = sample(shape, n, seed)
        poly = [{"lat": lat, "lon": lon, **({"ring": k} if k else {})} for k, r in enumerate(shape) for lat, lon in r]
        exp = {"ok": True, "result.count": n, "result.seed": str(seed), "result.area.value": area}
        tol = {"result.area.value": {"rel": 1e-9, "abs": 1e-6}, "result.count": {"abs": 0}}
        for j, (lat, lon) in enumerate(pts):
            exp[f"result.points.{j}.lat.value"] = lat
            exp[f"result.points.{j}.lon.value"] = lon
            tol[f"result.points.{j}.lat.value"] = {"abs": 1e-9}
            tol[f"result.points.{j}.lon.value"] = {"abs": 1e-9}
        out.append({"id": f"v{i + 1:03d}", "input": {"polygon": poly, "count": n, "seed": str(seed)}, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    tri = [{"lat": 40, "lon": -105}, {"lat": 40, "lon": -104.99}, {"lat": 40.01, "lon": -105}]
    for inp, code, field in [
        ({"polygon": [{"lat": 30, "lon": -100}, {"lat": 30, "lon": -94}, {"lat": 35, "lon": -94}], "count": 5}, "LIMIT_EXCEEDED", "/polygon"),
        ({"polygon": tri, "count": 5, "seed": "-1"}, "INVALID_INPUT", "/seed"),
    ]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"ok": False, "error.code": code, "error.field": field}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    for v in out:
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
