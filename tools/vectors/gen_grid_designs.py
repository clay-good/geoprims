#!/usr/bin/env python3
"""Golden vectors for geometry.sampling.systematic-points and
stratified-points: the methods in their derivation notes reimplemented in
Python, with PROJ's transverse Mercator (pyproj) for the plane and its scale
factor, and the stream and triangulation of gen_random_points.py. Prints JSON
lines; writes nothing.

    python3 tools/vectors/gen_grid_designs.py systematic > core/vectors/geometry.sampling.systematic-points.jsonl
    python3 tools/vectors/gen_grid_designs.py stratified > core/vectors/geometry.sampling.stratified-points.jsonl
    python3 tools/vectors/gen_grid_designs.py transects > core/vectors/geometry.sampling.transects.jsonl
    python3 tools/vectors/gen_grid_designs.py along > core/vectors/geometry.sampling.points-along-line.jsonl
    python3 tools/vectors/gen_grid_designs.py fishnet > core/vectors/geometry.sampling.fishnet.jsonl
    python3 tools/vectors/gen_grid_designs.py hex > core/vectors/geometry.sampling.hex-grid.jsonl

Needs pyproj, and shapely for the grids' inside fractions (a venv is fine)."""
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


GEOD = pyproj.Geod(ellps="WGS84")


def frame(points_ll):
    """The grid-north bearing at the mean corner, from geodesic azimuths to
    points 1 m up and down grid north."""
    lat_c = sum(p[0] for p in points_ll) / len(points_ll)
    lon_c = sum(p[1] for p in points_ll) / len(points_ll)
    cm = 6 * rp.zone(lat_c, lon_c) - 183
    crs = f"+proj=tmerc +lon_0={cm} +k=0.9996 +x_0=0 +y_0=0 +ellps=WGS84"
    tm = pyproj.Transformer.from_crs("EPSG:4326", crs, always_xy=True)
    x, y = tm.transform(lon_c, lat_c)
    # Centered: the azimuth up 1 m and the reverse of the one down 1 m,
    # averaged, so the grid line's curvature cancels to first order.
    up = tm.transform(x, y + 1.0, direction="INVERSE")
    dn = tm.transform(x, y - 1.0, direction="INVERSE")
    a1 = GEOD.inv(lon_c, lat_c, *up)[0]
    a2 = GEOD.inv(lon_c, lat_c, *dn)[0] + 180.0
    numeric = a1 + ((a2 - a1 + 180.0) % 360.0 - 180.0) / 2
    # PROJ's analytic value, checked against the geodesic estimate so its
    # sign convention is confirmed independently of the core.
    exact = pyproj.Proj(crs).get_factors(lon_c, lat_c).meridian_convergence
    assert abs(exact - numeric) < 1e-6, (exact, numeric)
    return exact


def transects(shape, d, bearing, zigzag, seed):
    rings, scale, back = plane(shape)
    gamma = frame(shape[0])
    dp = d * scale
    az = math.radians(bearing - gamma)
    dr = (math.sin(az), math.cos(az))
    ac = (dr[1], -dr[0])
    sf = lambda p: p[0] * ac[0] + p[1] * ac[1]
    lo, hi = min(sf(p) for p in rings[0]), max(sf(p) for p in rings[0])
    u = rp.Xoshiro(seed).f64() * dp
    out, route, number, k = [], [], 0, 0
    while True:
        c = lo + u + k * dp
        if c >= hi:
            break
        k += 1
        hits = []
        for r in rings:
            for i in range(len(r)):
                a, b = r[i], r[(i + 1) % len(r)]
                sa, sb = sf(a) - c, sf(b) - c
                if (sa < 0) != (sb < 0):
                    f = sa / (sa - sb)
                    q = (a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1]))
                    hits.append(q[0] * dr[0] + q[1] * dr[1])
        hits.sort()
        if len(hits) < 2:
            continue
        number += 1
        at = lambda t: (dr[0] * t + ac[0] * c, dr[1] * t + ac[1] * c)
        parts = [(at(hits[i]), at(hits[i + 1])) for i in range(0, len(hits) - 1, 2)]
        if zigzag and number % 2 == 0:
            parts = [(b, a) for a, b in reversed(parts)]
        for a, b in parts:
            la, lb = back(a), back(b)
            out.append((la, lb, GEOD.inv(la[1], la[0], lb[1], lb[0])[2]))
            route += [la, lb]
    rlen = sum(GEOD.inv(route[i][1], route[i][0], route[i + 1][1], route[i + 1][0])[2] for i in range(len(route) - 1))
    return number, out, rlen, u / scale


def along(line, step, start):
    rings, scale, back = plane([line])
    pts = [tuple(p) for p in rings[0]]
    # plane() orients ring 0 counterclockwise; a line keeps its order.
    lat_c = sum(p[0] for p in line) / len(line)
    lon_c = sum(p[1] for p in line) / len(line)
    cm = 6 * rp.zone(lat_c, lon_c) - 183
    tm = pyproj.Transformer.from_crs("EPSG:4326", f"+proj=tmerc +lon_0={cm} +k=0.9996 +x_0=0 +y_0=0 +ellps=WGS84", always_xy=True)
    pts = [tm.transform(lon, lat) for lat, lon in line]
    cum = [0.0]
    for a, b in zip(pts, pts[1:]):
        cum.append(cum[-1] + math.hypot(b[0] - a[0], b[1] - a[1]))
    length = cum[-1]
    count = 0 if start > length else math.floor((length - start) / step) + 1
    out, seg = [], 0
    for i in range(count):
        t = start + i * step
        while seg + 1 < len(pts) - 1 and cum[seg + 1] < t:
            seg += 1
        l = cum[seg + 1] - cum[seg]
        f = min(max((t - cum[seg]) / l, 0.0), 1.0) if l > 0 else 0.0
        a, b = pts[seg], pts[seg + 1]
        lon, lat = tm.transform(a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1]), direction="INVERSE")
        out.append((t, lat, lon))
    ground = sum(GEOD.inv(a[1], a[0], b[1], b[0])[2] for a, b in zip(line, line[1:]))
    return out, length, ground


def poly_json(shape):
    return [{"lat": lat, "lon": lon, **({"ring": k} if k else {})} for k, r in enumerate(shape) for lat, lon in r]


def main(which):
    if which in ("transects", "along"):
        return main_lines(which)
    if which in ("fishnet", "hex"):
        return main_grid(which)
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


def grid(shape, hex_, w, h, rot, clip):
    from shapely.geometry import Polygon as SP
    rings, scale, back = plane(shape)
    o = (min(p[0] for p in rings[0]), max(p[1] for p in rings[0]))
    t = math.radians(rot)
    framed = [[turn(p, o, -t) for p in r] for r in rings]
    poly = SP(framed[0], framed[1:])
    fx0, fx1 = min(p[0] for p in framed[0]), max(p[0] for p in framed[0])
    fy0, fy1 = min(p[1] for p in framed[0]), max(p[1] for p in framed[0])
    step_x = 0.866025403784439 * w if hex_ else w
    fit = lambda length, step: max(math.ceil(length / step - 1e-9), 1)
    cols, rows = fit(fx1 - fx0, step_x), fit(fy1 - fy0, h)
    cells = []
    if hex_:
        v = w
        lo, hi = 0.288675134594813 * v, 0.577350269189626 * v
        hs = lo + hi
        window = SP([(fx0, fy0), (fx1, fy0), (fx1, fy1), (fx0, fy1)])
        for c in range(-1, cols + 1):
            xa = fx0 + c * hs
            xb = xa + (hi - lo)
            xc = fx0 + c * hs + hs
            xd = xc + (hi - lo)
            k = c % 2
            for r in range(-1, rows + 1):
                ya = fy1 - (2 * r + k) * v / 2
                yb = fy1 - (2 * r + k + 1) * v / 2
                yc = fy1 - (2 * r + k + 2) * v / 2
                ring = [(xa, yb), (xb, yc), (xc, yc), (xd, yb), (xc, ya), (xb, ya)]
                edge = r < 0 or c < 0 or r >= rows or c >= cols
                if not edge or SP(ring).intersection(window).area > 0.5e-9 * v * v:
                    cells.append((r, c, ring))
    else:
        for c in range(cols):
            xa = fx0 + c * w
            for r in range(rows):
                ya = fy1 - r * h
                cells.append((r, c, [(xa, ya - h), (xa + w, ya - h), (xa + w, ya), (xa, ya)]))
    area = 0.866025403784439 * w * w if hex_ else w * h
    out = []
    for r, c, ring in cells:
        inside = min(max(SP(ring).intersection(poly).area / area, 0.0), 1.0)
        if clip and inside <= 0:
            continue
        cx, cy = sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring)
        out.append((r, c, back(turn((cx, cy), o, t)), inside, [back(turn(p, o, t)) for p in ring]))
    return out


def main_grid(which):
    rng = random.Random(20261025 if which == "fishnet" else 20261026)
    tool = "fishnet" if which == "fishnet" else "hex-grid"
    src = f"Python reimplementation of the derivation note with PROJ tmerc and Shapely (GEOS) intersection areas (tools/vectors/gen_grid_designs.py {which})"
    out = []
    for i in range(20):
        shape = rp.SHAPES[i % len(rp.SHAPES)]
        rings, _, _ = plane(shape)
        span = max(max(p[0] for p in rings[0]) - min(p[0] for p in rings[0]), max(p[1] for p in rings[0]) - min(p[1] for p in rings[0]))
        w = round(span / rng.uniform(3, 9), 1)
        h = round(span / rng.uniform(3, 9), 1) if which == "fishnet" and rng.random() < 0.5 else w
        rot = rng.choice([0, 0, 0, 12, 45])
        clip = rng.random() < 0.5
        cells = grid(shape, which == "hex", w, h, rot, clip)
        inp = {"polygon": poly_json(shape)}
        if which == "fishnet":
            inp["width"] = f"{w} m"
            if h != w:
                inp["height"] = f"{h} m"
        else:
            inp["size"] = f"{w} m"
        if rot:
            inp["rotation"] = f"{rot} deg"
        if clip:
            inp["cells"] = "clip"
        exp = {"ok": True, "result.count": len(cells)}
        tol = {"result.count": {"abs": 0}}
        for j, (r, c, (lat, lon), inside, ring) in enumerate(cells[:10]):
            exp[f"result.cells.{j}.row"] = r
            exp[f"result.cells.{j}.col"] = c
            tol[f"result.cells.{j}.row"] = {"abs": 0}
            tol[f"result.cells.{j}.col"] = {"abs": 0}
            for f, v in (("lat", lat), ("lon", lon)):
                exp[f"result.cells.{j}.{f}.value"] = v
                tol[f"result.cells.{j}.{f}.value"] = {"abs": 1e-9}
            exp[f"result.cells.{j}.inside"] = inside
            tol[f"result.cells.{j}.inside"] = {"abs": 1e-7}
        for k, (lat, lon) in enumerate(cells[0][4] if cells else []):
            exp[f"result.outlines.{k}.lat.value"] = lat
            exp[f"result.outlines.{k}.lon.value"] = lon
            tol[f"result.outlines.{k}.lat.value"] = {"abs": 1e-9}
            tol[f"result.outlines.{k}.lon.value"] = {"abs": 1e-9}
        out.append({"id": f"v{i + 1:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": VER + ", shapely", "tolerance": tol})
    big = [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0, "lon": -104.8826}, {"lat": 40.09, "lon": -104.8826}, {"lat": 40.09, "lon": -105.0}]
    field = "/width" if which == "fishnet" else "/size"
    out.append({"id": f"v{len(out) + 1:03d}", "input": {"polygon": big, field[1:]: "10 m"}, "expect": {"ok": False, "error.code": "LIMIT_EXCEEDED", "error.field": field}, "source": src, "sourceVersion": VER, "tolerance": {}})
    for v in out:
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))


def main_lines(which):
    rng = random.Random(20261023 if which == "transects" else 20261024)
    src = f"Python reimplementation of the derivation note with PROJ tmerc and pyproj.Geod (tools/vectors/gen_grid_designs.py {which})"
    out = []
    for i in range(20):
        shape = rp.SHAPES[i % len(rp.SHAPES)]
        rings, _, _ = plane(shape)
        span = max(max(p[0] for p in rings[0]) - min(p[0] for p in rings[0]), max(p[1] for p in rings[0]) - min(p[1] for p in rings[0]))
        if which == "transects":
            seed = rng.choice([1, 42, rng.randrange(1 << 64)])
            d = round(span / rng.uniform(3, 8), 1)
            bearing = rng.choice([0, 45, 90, 137.5, 200])
            zig = rng.random() < 0.4
            n, parts, rlen, off = transects(shape, d, bearing, zig, seed)
            inp = {"polygon": poly_json(shape), "spacing": f"{d} m", "bearing": f"{bearing} deg", "seed": str(seed)}
            if zig:
                inp["route"] = "zigzag"
            exp = {"ok": True, "result.count": n, "result.total_length.value": sum(p[2] for p in parts), "result.offset.value": off}
            tol = {"result.count": {"abs": 0}, "result.total_length.value": {"abs": 1e-5}, "result.offset.value": {"abs": 1e-6}}
            if zig:
                exp["result.route_length.value"] = rlen
                tol["result.route_length.value"] = {"abs": 1e-5}
            for j, (a, b, l) in enumerate(parts[:8]):
                for f, v in (("start_lat", a[0]), ("start_lon", a[1]), ("end_lat", b[0]), ("end_lon", b[1])):
                    exp[f"result.transects.{j}.{f}.value"] = v
                    tol[f"result.transects.{j}.{f}.value"] = {"abs": 1e-9}
                exp[f"result.transects.{j}.length.value"] = l
                tol[f"result.transects.{j}.length.value"] = {"abs": 1e-5}
        else:
            line = [p for p in shape[0]][: rng.randint(2, len(shape[0]))]
            step = round(span / rng.uniform(4, 20), 2)
            start = rng.choice([0, 0, round(step * rng.random(), 2)])
            pts, length, ground = along(line, step, start)
            inp = {"line": [{"lat": a, "lon": b} for a, b in line], "interval": f"{step} m"}
            if start:
                inp["start"] = f"{start} m"
            exp = {"ok": True, "result.count": len(pts), "result.plane_length.value": length, "result.geodesic_length.value": ground}
            tol = {"result.count": {"abs": 0}, "result.plane_length.value": {"abs": 1e-6}, "result.geodesic_length.value": {"abs": 1e-6}}
            for j, (t, lat, lon) in enumerate(pts[:12]):
                for f, v in (("distance", t), ("lat", lat), ("lon", lon)):
                    exp[f"result.points.{j}.{f}.value"] = v
                    tol[f"result.points.{j}.{f}.value"] = {"abs": 1e-9}
        out.append({"id": f"v{i + 1:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": VER, "tolerance": tol})
    big = [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0, "lon": -104.8826}, {"lat": 40.09, "lon": -104.8826}, {"lat": 40.09, "lon": -105.0}]
    if which == "transects":
        refusal = ({"polygon": big, "spacing": "0.5 m"}, "LIMIT_EXCEEDED", "/spacing")
    else:
        refusal = ({"line": [{"lat": 40, "lon": -105}, {"lat": 40.1, "lon": -105}], "interval": "0.1 m"}, "LIMIT_EXCEEDED", "/interval")
    out.append({"id": f"v{len(out) + 1:03d}", "input": refusal[0], "expect": {"ok": False, "error.code": refusal[1], "error.field": refusal[2]}, "source": src, "sourceVersion": VER, "tolerance": {}})
    for v in out:
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main(sys.argv[1])
