#!/usr/bin/env python3
"""More golden vectors for the three cover and family tools.

`gen_cover.py` wrote five to seven vectors each from an independent Python
transcription -- the OpenStreetMap slippy-map formulas, a from-scratch geohash
encoder, and overlap by clipping each cell against the polygon. This reuses
those same helpers, unchanged, and adds the cases they did not reach:

- tile.family: every zoom from 0 to 20, both conventions, quadkey input, the
  corners of the grid, and tiles either side of the antimeridian.
- tile.cover: boxes that straddle the antimeridian and the equator, boxes that
  land exactly on tile edges, one-tile boxes, and the Mercator latitude clamp.
- geohash.cover: precisions 1 to 8, both modes, a box on a cell boundary, and
  polygons that are thin, concave and antimeridian-crossing.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_cover_more.py
"""
import json
import math
import sys
from pathlib import Path

from gen_cover import (
    SRC,
    VER,
    clip_area,
    gh_cells,
    gh_encode,
    lat2y,
    pip,
    quadkey,
    y2lat,
)

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
LIMIT = 85.05112877980659

# (tile or quadkey, convention)
FAMILY_CASES = [
    ("1/0/0", None), ("1/1/1", None), ("2/3/1", None), ("3/0/7", None),
    ("8/128/128", None), ("10/511/340", None), ("15/16383/16383", None),
    ("18/131072/131072", None), ("20/1048575/1048575", None),
    ("4/0/15", "tms"), ("6/33/20", "tms"), ("12/2048/2048", "tms"),
    ("0", None), ("13", None), ("3012", None), ("1203031", None),
    ("7/0/0", None), ("7/127/127", None),
]

# (south, west, north, east, zoom)
TILE_COVER_CASES = [
    (0.0, 0.0, 0.0001, 0.0001, 10),          # one tile
    (-1.0, -1.0, 1.0, 1.0, 6),               # across the equator and meridian
    (-45.0, 179.0, -44.0, -179.0, 7),        # across the antimeridian
    (0.0, -180.0, 45.0, -90.0, 2),           # exactly on tile edges
    (-85.0, -180.0, 85.0, 180.0, 1),         # the whole world at zoom 1
    (51.4, -0.3, 51.6, 0.1, 12),
    (35.6, 139.6, 35.8, 139.8, 13),
    (-33.95, 151.1, -33.8, 151.3, 12),
    (-90.0, -10.0, -80.0, 10.0, 4),          # past the Mercator clamp, south
    (19.4, -99.2, 19.5, -99.1, 11),
    (60.0, 4.0, 61.0, 6.0, 9),
    (-0.5, 36.5, 0.5, 37.5, 8),
    (25.1, 55.2, 25.3, 55.4, 11),
    (48.0, 2.0, 49.0, 3.0, 7),
]

# (precision, box, mode) for boxes; (precision, polygon, mode) for polygons
GH_BOX_CASES = [
    (1, (-30.0, -60.0, 30.0, 0.0), False),
    (2, (40.0, -80.0, 42.0, -78.0), False),
    (3, (0.0, 0.0, 10.0, 10.0), True),
    (4, (51.4, -0.3, 51.6, 0.1), False),
    (5, (-33.95, 151.1, -33.8, 151.3), False),
    (6, (35.68, 139.76, 35.70, 139.78), True),
    (7, (39.7390, -104.9905, 39.7395, -104.9900), False),
    (8, (39.73920, -104.99030, 39.73925, -104.99025), False),
    (4, (-5.0, 178.0, 5.0, -178.0), False),  # across the antimeridian
    (3, (-90.0, -180.0, -80.0, -170.0), False),  # the south-west corner
]
GH_POLY_CASES = [
    (5, [(51.50, -0.15), (51.52, -0.10), (51.48, -0.08)], False),
    # Smaller than a precision-5 cell, so centre mode finds nothing at all.
    (5, [(51.50, -0.15), (51.52, -0.10), (51.48, -0.08)], True),
    (6, [(35.68, 139.75), (35.69, 139.78), (35.67, 139.79), (35.66, 139.76)], True),
    (4, [(-33.80, 151.10), (-33.85, 151.30), (-33.95, 151.15)], False),
    # A thin sliver, where a centre test and an overlap test disagree most.
    (6, [(40.430, -80.020), (40.432, -79.980), (40.431, -79.980), (40.429, -80.020)], False),
]


def tile_of(text, convention):
    """(z, x, y) in XYZ, from a z/x/y string or a quadkey."""
    if "/" in text:
        z, x, y = map(int, text.split("/"))
        if convention == "tms":
            y = 2 ** z - 1 - y
        return z, x, y
    z, x, y = len(text), 0, 0
    for c in text:
        d = int(c)
        x, y = x * 2 + (d & 1), y * 2 + (d >> 1)
    return z, x, y


def family_rows(start):
    rows = []
    for i, (text, conv) in enumerate(FAMILY_CASES, start=start + 1):
        z, x, y = tile_of(text, conv)
        flip = (lambda zz, yy: 2 ** zz - 1 - yy) if conv == "tms" else (lambda zz, yy: yy)
        exp = {"result.parent": "none" if z == 0 else f"{z - 1}/{x // 2}/{flip(z - 1, y // 2)}"}
        for k, (dx, dy) in enumerate([(0, 0), (1, 0), (0, 1), (1, 1)]):
            exp[f"result.children.{k}.tile"] = f"{z + 1}/{2 * x + dx}/{flip(z + 1, 2 * y + dy)}"
            exp[f"result.children.{k}.quadkey"] = quadkey(z + 1, 2 * x + dx, 2 * y + dy)
        exp["ok"] = True
        inp = {"tile": text}
        if conv:
            inp["convention"] = conv
        rows.append((i, inp, exp, {}))
    return rows


def tile_cover_rows(start):
    rows = []
    for i, (s, w, n, e, z) in enumerate(TILE_COVER_CASES, start=start + 1):
        N = 2 ** z

        def xcol(lon, east):
            t = (lon + 180) / 360 * N
            if east and t == int(t) and t > 0:
                t -= 1
            return min(N - 1, int(math.floor(t)))

        y_top, y_bot = lat2y(n, z), lat2y(s, z)
        if y_bot > y_top and y2lat(y_bot, z) <= max(-LIMIT, s):
            y_bot -= 1
        x0, x1 = xcol(w, False), xcol(e, True)
        cols = list(range(x0, x1 + 1)) if w < e else list(range(x0, N)) + list(range(0, x1 + 1))
        tiles = [f"{z}/{x}/{y}" for y in range(y_top, y_bot + 1) for x in cols]
        exp = {"result.count": float(len(tiles)), "result.tiles.0.tile": tiles[0],
               f"result.tiles.{len(tiles) - 1}.tile": tiles[-1], "ok": True}
        rows.append((i, {"south": s, "west": w, "north": n, "east": e, "zoom": z}, exp,
                     {"result.count": {"abs": 0}}))
    return rows


def gh_cover_rows(start):
    rows = []
    i = start
    for p, (s, w, n, e), center in GH_BOX_CASES:
        i += 1
        hs = []
        for b in gh_cells(p, s, w, n, e):
            clat, clon = (b[0] + b[2]) / 2, (b[1] + b[3]) / 2
            if center and not (s <= clat <= n and (w <= clon <= e if w < e else (clon >= w or clon <= e))):
                continue
            hs.append(gh_encode(clat, clon, p))
        inp = {"precision": p, "bbox": f"{s}, {w}, {n}, {e}"}
        if center:
            inp["mode"] = "center"
        rows.append((i, inp, {"result.count": float(len(hs)), "result.geohashes.0.geohash": hs[0],
                              f"result.geohashes.{len(hs) - 1}.geohash": hs[-1], "ok": True},
                     {"result.count": {"abs": 0}}))
    for p, poly, center in GH_POLY_CASES:
        i += 1
        la = [q[0] for q in poly]
        lo = [q[1] for q in poly]
        hs = []
        for b in gh_cells(p, min(la), min(lo), max(la), max(lo)):
            clat, clon = (b[0] + b[2]) / 2, (b[1] + b[3]) / 2
            ok = pip(poly, clat, clon) if center else (
                clip_area(poly, b) > 0 or any(b[0] <= a <= b[2] and b[1] <= o <= b[3] for a, o in poly))
            if ok:
                hs.append(gh_encode(clat, clon, p))
        inp = {"precision": p, "polygon": [{"lat": a, "lon": o} for a, o in poly]}
        if center:
            inp["mode"] = "center"
        # A polygon smaller than one cell has no cell centre inside it, so
        # centre mode covers nothing. That is the right answer and the reason
        # overlap mode exists, so the case is kept and its count pinned at 0.
        expect = {"result.count": float(len(hs)), "ok": True}
        if hs:
            expect["result.geohashes.0.geohash"] = hs[0]
        rows.append((i, inp, expect, {"result.count": {"abs": 0}}))
    return rows


def main():
    plan = [
        ("indexing.tile.family", family_rows),
        ("indexing.tile.cover", tile_cover_rows),
        ("indexing.geohash.cover", gh_cover_rows),
    ]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 8:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": SRC, "sourceVersion": VER, "tolerance": tol},
                                   ensure_ascii=False, separators=(",", ":")) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
