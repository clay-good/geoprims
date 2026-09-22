#!/usr/bin/env python3
"""Golden vectors for indexing.tile.family, indexing.tile.cover, and
indexing.geohash.cover, from an independent Python transcription: the
OpenStreetMap slippy-map formulas, a from-scratch geohash encoder, and
overlap by clipping each cell against the polygon (Sutherland-Hodgman)."""
import json
import math
import sys
from pathlib import Path

SRC, VER = "Slippy-map and geohash arithmetic in Python (tools/vectors/gen_cover.py)", "2026"
B32 = "0123456789bcdefghjkmnpqrstuvwxyz"


def quadkey(z, x, y):
    return "".join(str(((x >> (i - 1)) & 1) + 2 * ((y >> (i - 1)) & 1)) for i in range(z, 0, -1))


def lat2y(lat, z):
    lat = max(-85.05112877980659, min(85.05112877980659, lat))
    n = 2 ** z
    r = math.radians(lat)
    return min(n - 1, max(0, int(math.floor((1 - math.asinh(math.tan(r)) / math.pi) / 2 * n))))


def y2lat(y, z):
    return math.degrees(math.atan(math.sinh(math.pi * (1 - 2 * y / 2 ** z))))


def family():
    out = []
    for tile, conv in [("12/1137/1544", None), ("0/0/0", None), ("5/17/10", "tms"), ("021230", None), ("20/301847/394238", None)]:
        if "/" in tile:
            z, x, y = map(int, tile.split("/"))
            if conv == "tms":
                y = 2 ** z - 1 - y
        else:
            z, x, y = len(tile), 0, 0
            for c in tile:
                d = int(c)
                x, y = x * 2 + (d & 1), y * 2 + (d >> 1)
        flip = (lambda zz, yy: 2 ** zz - 1 - yy) if conv == "tms" else (lambda zz, yy: yy)
        exp = {"result.parent": "none" if z == 0 else f"{z - 1}/{x // 2}/{flip(z - 1, y // 2)}"}
        for k, (dx, dy) in enumerate([(0, 0), (1, 0), (0, 1), (1, 1)]):
            exp[f"result.children.{k}.tile"] = f"{z + 1}/{2 * x + dx}/{flip(z + 1, 2 * y + dy)}"
            exp[f"result.children.{k}.quadkey"] = quadkey(z + 1, 2 * x + dx, 2 * y + dy)
        exp["ok"] = True
        inp = {"tile": tile}
        if conv:
            inp["convention"] = conv
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    return out


def tile_cover():
    out = []
    for s, w, n, e, z in [(40.43, -80.02, 40.45, -79.98, 14), (40.0, -106.0, 41.0, -104.0, 8), (-10.0, 170.0, 10.0, -170.0, 5),
                          (0.0, 0.0, 22.5, 22.5, 4), (50.0, -1.0, 52.0, 1.0, 10), (84.0, -10.0, 89.0, 10.0, 3)]:
        N = 2 ** z
        def xcol(lon, east):
            t = (lon + 180) / 360 * N
            if east and t == int(t) and t > 0:
                t -= 1
            return min(N - 1, int(math.floor(t)))
        y_top, y_bot = lat2y(n, z), lat2y(s, z)
        if y_bot > y_top and y2lat(y_bot, z) <= max(-85.05112877980659, s):
            y_bot -= 1
        x0, x1 = xcol(w, False), xcol(e, True)
        cols = list(range(x0, x1 + 1)) if w < e else list(range(x0, N)) + list(range(0, x1 + 1))
        tiles = [f"{z}/{x}/{y}" for y in range(y_top, y_bot + 1) for x in cols]
        exp = {"result.count": float(len(tiles)), "result.tiles.0.tile": tiles[0], f"result.tiles.{len(tiles) - 1}.tile": tiles[-1], "ok": True}
        out.append({"id": f"v{len(out) + 1:03d}", "input": {"south": s, "west": w, "north": n, "east": e, "zoom": z}, "expect": exp,
                    "source": SRC, "sourceVersion": VER, "tolerance": {"result.count": {"abs": 0}}})
    out.append({"id": f"v{len(out) + 1:03d}", "input": {"south": -60, "west": -170, "north": 60, "east": 170, "zoom": 12},
                "expect": {"ok": False, "error.code": "LIMIT_EXCEEDED"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    return out


def gh_encode(lat, lon, p):
    lat_r, lon_r, bits, even, s = [-90.0, 90.0], [-180.0, 180.0], 0, True, ""
    ch = 0
    while len(s) < p:
        r, v = (lon_r, lon) if even else (lat_r, lat)
        mid = (r[0] + r[1]) / 2
        ch = ch * 2 + (1 if v >= mid else 0)
        if v >= mid:
            r[0] = mid
        else:
            r[1] = mid
        even = not even
        bits += 1
        if bits == 5:
            s, bits, ch = s + B32[ch], 0, 0
    return s


def clip_area(poly, box):
    """Area of the polygon clipped to the box (Sutherland-Hodgman), in deg²."""
    s, w, n, e = box
    pts = [(lo, la) for la, lo in poly]
    for keep, inter in [(lambda p: p[0] >= w, lambda a, b: (w, a[1] + (w - a[0]) / (b[0] - a[0]) * (b[1] - a[1]))),
                        (lambda p: p[0] <= e, lambda a, b: (e, a[1] + (e - a[0]) / (b[0] - a[0]) * (b[1] - a[1]))),
                        (lambda p: p[1] >= s, lambda a, b: (a[0] + (s - a[1]) / (b[1] - a[1]) * (b[0] - a[0]), s)),
                        (lambda p: p[1] <= n, lambda a, b: (a[0] + (n - a[1]) / (b[1] - a[1]) * (b[0] - a[0]), n))]:
        res = []
        for i in range(len(pts)):
            a, b = pts[i - 1], pts[i]
            if keep(b):
                if not keep(a):
                    res.append(inter(a, b))
                res.append(b)
            elif keep(a):
                res.append(inter(a, b))
        pts = res
        if not pts:
            return 0.0
    return abs(sum(pts[i - 1][0] * pts[i][1] - pts[i][0] * pts[i - 1][1] for i in range(len(pts)))) / 2


def pip(poly, lat, lon):
    c = False
    for i in range(len(poly)):
        a, b = poly[i - 1], poly[i]
        if (a[0] > lat) != (b[0] > lat) and lon < a[1] + (lat - a[0]) / (b[0] - a[0]) * (b[1] - a[1]):
            c = not c
    return c


def gh_cells(p, s, w, n, e):
    lon_bits, lat_bits = (5 * p + 1) // 2, 5 * p // 2
    cw, chh = 360 / 2 ** lon_bits, 180 / 2 ** lat_bits
    row = lambda la: min(2 ** lat_bits - 1, int(math.floor((la + 90) / chh)))
    col = lambda lo: min(2 ** lon_bits - 1, int(math.floor((lo + 180) / cw)))
    cols = list(range(col(w), col(e) + 1)) if w < e else list(range(col(w), 2 ** lon_bits)) + list(range(0, col(e) + 1))
    for r in range(row(s), row(n) + 1):
        for c in cols:
            yield (-90 + r * chh, -180 + c * cw, -90 + (r + 1) * chh, -180 + (c + 1) * cw)


def gh_cover():
    out = []
    boxes = [(6, (40.43, -80.02, 40.45, -79.98), False), (4, (40.0, -106.0, 41.0, -104.0), False), (3, (-5.0, 175.0, 5.0, -175.0), False),
             (5, (51.4, -0.3, 51.6, 0.1), True)]
    for p, (s, w, n, e), center in boxes:
        hs = []
        for b in gh_cells(p, s, w, n, e):
            clat, clon = (b[0] + b[2]) / 2, (b[1] + b[3]) / 2
            if center and not (s <= clat <= n and (w <= clon <= e if w < e else (clon >= w or clon <= e))):
                continue
            hs.append(gh_encode(clat, clon, p))
        inp = {"precision": p, "bbox": f"{s}, {w}, {n}, {e}"}
        if center:
            inp["mode"] = "center"
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"result.count": float(len(hs)), "result.geohashes.0.geohash": hs[0],
                    f"result.geohashes.{len(hs) - 1}.geohash": hs[-1], "ok": True}, "source": SRC, "sourceVersion": VER, "tolerance": {"result.count": {"abs": 0}}})
    tri = [(40.40, -80.05), (40.47, -80.00), (40.41, -79.93)]
    for p, center in [(6, False), (6, True), (5, False)]:
        la = [q[0] for q in tri]
        lo = [q[1] for q in tri]
        hs = []
        for b in gh_cells(p, min(la), min(lo), max(la), max(lo)):
            clat, clon = (b[0] + b[2]) / 2, (b[1] + b[3]) / 2
            ok = pip(tri, clat, clon) if center else clip_area(tri, b) > 0 or any(b[0] <= a <= b[2] and b[1] <= o <= b[3] for a, o in tri)
            if ok:
                hs.append(gh_encode(clat, clon, p))
        inp = {"precision": p, "polygon": [{"lat": a, "lon": o} for a, o in tri]}
        if center:
            inp["mode"] = "center"
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"result.count": float(len(hs)), "result.geohashes.0.geohash": hs[0], "ok": True},
                    "source": SRC, "sourceVersion": VER, "tolerance": {"result.count": {"abs": 0}}})
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for tool, vs in {"indexing.tile.family": family(), "indexing.tile.cover": tile_cover(), "indexing.geohash.cover": gh_cover()}.items():
        (dest / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
        print(tool, [v["expect"].get("result.count") for v in vs])


if __name__ == "__main__":
    main()
