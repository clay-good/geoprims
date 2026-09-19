#!/usr/bin/env python3
"""Golden vectors for indexing.geohash.*, indexing.tile.*, and
indexing.plus-code.*: geohash and slippy-map formulas implemented here in
Python, Open Location Code cases from the OLC project's test data
(encoding.csv), and the add-spatial-indexing-and-raster scenarios."""
import json
import math
import sys
from pathlib import Path

SPEC = "add-spatial-indexing-and-raster scenarios"
GH_SRC = "Geohash algorithm implemented in Python (tools/vectors/gen_indexing.py)"
TILE_SRC = "OpenStreetMap slippy-map formulas and Bing quadkeys in Python (tools/vectors/gen_indexing.py)"
OLC_SRC = "Open Location Code test data, encoding.csv"
B32 = "0123456789bcdefghjkmnpqrstuvwxyz"
PTS = [(40.446111, -79.982222), (-33.8688, 151.2093), (51.5074, -0.1278), (64.8378, -147.7164), (-0.5, 179.999)]


def vec(i, inp, exp, src, ver="2026", tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def gh_encode(lat, lon, p):
    la, lo, bits, out, even = [-90.0, 90.0], [-180.0, 180.0], [], "", True
    while len(out) < p:
        r, v = (lo, lon) if even else (la, lat)
        mid = (r[0] + r[1]) / 2
        if v >= mid:
            bits.append(1); r[0] = mid
        else:
            bits.append(0); r[1] = mid
        even = not even
        if len(bits) == 5:
            out += B32[int("".join(map(str, bits)), 2)]; bits = []
    return out, (la[0], lo[0], la[1], lo[1])


EXTRA_GH = [(0.0, 0.0, 1), (89.9999, 179.9999, 8), (-89.9999, -179.9999, 8), (35.6762, 139.6503, 10), (-22.9068, -43.1729, 4),
            (19.4326, -99.1332, 2), (-1.2921, 36.8219, 3), (55.7558, 37.6173, 11), (37.7749, -122.4194, 12), (-45.0312, 168.6626, 9),
            (1.3521, 103.8198, 7), (64.1466, -21.9426, 6), (-77.85, 166.6667, 5), (30.0444, 31.2357, 8), (0.0000001, -0.0000001, 12),
            (45.0, -90.0, 1), (-60.0, 120.0, 3)]


def geohash():
    enc, dec = [], []
    for i, ((la, lo), p) in enumerate(zip(PTS, [9, 7, 12, 5, 6]), 1):
        g, b = gh_encode(la, lo, p)
        src = SPEC if i == 1 else GH_SRC
        enc.append(vec(i, {"lat": la, "lon": lo, "precision": p}, {"result.geohash": g, "result.south.value": b[0], "result.north.value": b[2]}, src))
        dec.append(vec(i, {"geohash": g}, {"result.lat.value": (b[0] + b[2]) / 2, "result.lon.value": (b[1] + b[3]) / 2,
                                           "result.lat_error.value": (b[2] - b[0]) / 2}, GH_SRC))
    dec.append(vec(6, {"geohash": "dpan"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    for i, (la, lo, p) in enumerate(EXTRA_GH, len(enc) + 1):
        g, b = gh_encode(la, lo, p)
        enc.append(vec(i, {"lat": la, "lon": lo, "precision": p}, {"result.geohash": g, "result.south.value": b[0], "result.north.value": b[2]}, GH_SRC))
    # The worked example in the Wikipedia article (after Niemeyer's geohash.org)
    enc.append(vec(len(enc) + 1, {"lat": 57.64911, "lon": 10.40744, "precision": 11}, {"result.geohash": "u4pruydqqvj"},
                   "Wikipedia, Geohash (worked example)", "retrieved 2026-09-19"))
    nb = []
    for i, (la, lo) in enumerate(PTS, 1):
        g, b = gh_encode(la, lo, 6)
        h, w = b[2] - b[0], b[3] - b[1]
        e_lon = ((b[1] + b[3]) / 2 + w + 180) % 360 - 180
        nb.append(vec(i, {"geohash": g}, {"result.n": gh_encode((b[0] + b[2]) / 2 + h, (b[1] + b[3]) / 2, 6)[0], "result.e": gh_encode((b[0] + b[2]) / 2, e_lon, 6)[0]},
                      SPEC if i == 5 else GH_SRC))
    nb.append(vec(6, {"geohash": gh_encode(89.99, 0, 3)[0]}, {"result.n": "none (past the pole)"}, SPEC))
    return enc, dec, nb


def tile(lat, lon, z):
    n = 2 ** z
    x = int((lon + 180) / 360 * n)
    y = int((1 - math.asinh(math.tan(math.radians(lat))) / math.pi) / 2 * n)
    return x, y


def quadkey(z, x, y):
    return "".join(str(((x >> (i - 1)) & 1) + 2 * ((y >> (i - 1)) & 1)) for i in range(z, 0, -1))


def res(lat, z, px=256):
    return math.cos(math.radians(lat)) * 2 * math.pi * 6378137 / (px * 2 ** z)


def tiles():
    fp, bd, gr = [], [], []
    for i, ((la, lo), z) in enumerate(zip(PTS, [12, 5, 17, 9, 20]), 1):
        x, y = tile(la, lo, z)
        src = SPEC if i == 1 else TILE_SRC
        fp.append(vec(i, {"lat": la, "lon": lo, "zoom": z}, {"result.tile": f"{z}/{x}/{y}", "result.tms_y": float(2 ** z - 1 - y),
                                                           "result.quadkey": quadkey(z, x, y), "result.ground_resolution.value": res(la, z)}, src))
        n = 2 ** z
        lat_n = math.degrees(math.atan(math.sinh(math.pi * (1 - 2 * y / n))))
        lat_s = math.degrees(math.atan(math.sinh(math.pi * (1 - 2 * (y + 1) / n))))
        bd.append(vec(i, {"tile": f"{z}/{x}/{y}"}, {"result.north.value": lat_n, "result.south.value": lat_s, "result.west.value": x / n * 360 - 180,
                                                  "result.quadkey": quadkey(z, x, y)}, TILE_SRC))
        gr.append(vec(i, {"lat": la, "zoom": z, "tile_size": "512" if i == 3 else "256"}, {"result.resolution.value": res(la, z, 512 if i == 3 else 256)}, SPEC if i == 1 else TILE_SRC))
    fp.append(vec(6, {"lat": 89, "lon": 0, "zoom": 3}, {"result.tile": "3/4/0", "meta.warnings.0.code": "WEB_MERCATOR_CLAMPED"}, SPEC))
    bd.append(vec(6, {"tile": "12/1137/2551", "convention": "tms"}, {"result.xyz": "12/1137/1544", "result.quadkey": "032001112001"}, SPEC))
    bd.append(vec(7, {"tile": "032001112001"}, {"result.xyz": "12/1137/1544"}, SPEC))
    bd.append(vec(8, {"tile": "3/8/0"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    for la, lo, z in EXTRA_TILE:
        x, y = tile(la, lo, z)
        n = 2 ** z
        fp.append(vec(len(fp) + 1, {"lat": la, "lon": lo, "zoom": z}, {"result.tile": f"{z}/{x}/{y}", "result.tms_y": float(n - 1 - y),
                                                                     "result.quadkey": quadkey(z, x, y), "result.ground_resolution.value": res(la, z)}, TILE_SRC))
        lat_n = math.degrees(math.atan(math.sinh(math.pi * (1 - 2 * y / n))))
        lat_s = math.degrees(math.atan(math.sinh(math.pi * (1 - 2 * (y + 1) / n))))
        bd.append(vec(len(bd) + 1, {"tile": f"{z}/{x}/{y}"}, {"result.north.value": lat_n, "result.south.value": lat_s, "result.west.value": x / n * 360 - 180,
                                                            "result.east.value": (x + 1) / n * 360 - 180, "result.quadkey": quadkey(z, x, y)}, TILE_SRC))
    # Published examples: the mercantile README (10/486/332) and the Bing Maps Tile System article
    # (quadkey 213 for tile (3, 5) at level 3; ground resolution 78,271.5170 m/px at level 1 on the equator)
    fp.append(vec(len(fp) + 1, {"lat": 53.33087298301705, "lon": -9.140625, "zoom": 10}, {"result.tile": "10/486/332"}, MERC, MERC_VER))
    fp.append(vec(len(fp) + 1, {"lat": 0, "lon": 0, "zoom": 1}, {"result.ground_resolution.value": 78271.5170}, BING, BING_VER, tol=0))
    fp[-1]["tolerance"] = {"result.ground_resolution.value": {"abs": 5e-5}}
    bd.append(vec(len(bd) + 1, {"tile": "10/486/332"}, {"result.west.value": -9.140625, "result.south.value": 53.12040528310657,
                                                      "result.east.value": -8.7890625, "result.north.value": 53.33087298301705}, MERC, MERC_VER))
    bd.append(vec(len(bd) + 1, {"tile": "3/3/5"}, {"result.quadkey": "213"}, BING, BING_VER))
    return fp, bd, gr


EXTRA_TILE = [(0.0001, 0.0001, 0), (0.0001, 0.0001, 1), (-0.0001, -0.0001, 1), (85.0, 179.99, 4), (-85.0, -179.99, 4), (35.6762, 139.6503, 14),
              (-22.9068, -43.1729, 8), (19.4326, -99.1332, 11), (55.7558, 37.6173, 16), (37.7749, -122.4194, 18), (-45.0312, 168.6626, 13),
              (1.3521, 103.8198, 22), (64.1466, -21.9426, 7), (30.0444, 31.2357, 10), (-77.85, 166.6667, 6), (47.6062, -122.3321, 21)]
MERC = "mercantile README (Mapbox), worked examples"
MERC_VER = "mercantile 1.2.1"
BING = "Microsoft, Bing Maps Tile System (worked quadkey example and ground-resolution table)"
BING_VER = "2018-02-28"


OLC_CASES = [  # lat, lng, length, code (OLC encoding.csv)
    (20.375, 2.775, 6, "7FG49Q00+"), (20.3700625, 2.7821875, 10, "7FG49QCJ+2V"), (20.3701125, 2.782234375, 11, "7FG49QCJ+2VX"),
    (47.0000625, 8.0000625, 10, "8FVC2222+22"), (-41.2730625, 174.7859375, 10, "4VCPPQGP+Q9"), (0.5, 179.5, 4, "6VGX0000+"),
    (-89.5, -179.5, 4, "22220000+"), (20.5, 2.5, 4, "7FG40000+"), (-89.9999375, -179.9999375, 10, "22222222+22"),
]


def olc():
    enc = [vec(i, {"lat": la, "lon": lo, "length": n}, {"result.code": c}, OLC_SRC, "open-location-code main") for i, (la, lo, n, c) in enumerate(OLC_CASES, 1)]
    dec = [vec(i, {"code": c}, {"result.full_code": c}, OLC_SRC, "open-location-code main") for i, (_, _, _, c) in enumerate(OLC_CASES[:5], 1)]
    dec.append(vec(6, {"code": "9G8F+6X"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/ref_lat"}, SPEC))
    dec.append(vec(7, {"code": "9G8F+6X", "ref_lat": 47.4, "ref_lon": 8.6}, {"result.full_code": "8FVC9G8F+6X"}, "Open Location Code specification (recover nearest)", "open-location-code main"))
    sh = [
        vec(1, {"code": "8FVC9G8F+6X", "ref_lat": 47.4, "ref_lon": 8.6}, {"result.short_code": "9G8F+6X"}, "Open Location Code specification (shorten)", "open-location-code main"),
        vec(2, {"code": "8FVC9G8F+6X", "ref_lat": 47.37, "ref_lon": 8.52}, {"result.short_code": "8F+6X"}, "Open Location Code specification (shorten)", "open-location-code main"),
        vec(3, {"code": "8FVC9G8F+6X", "ref_lat": 40.0, "ref_lon": -80.0}, {"result.short_code": "8FVC9G8F+6X"}, "Open Location Code specification (shorten)", "open-location-code main"),
        vec(4, {"code": "7FG49QCJ+2V", "ref_lat": 20.37, "ref_lon": 2.78}, {"result.short_code": "CJ+2V"}, "Open Location Code specification (shorten)", "open-location-code main"),
        vec(5, {"code": "4VCPPQGP+Q9", "ref_lat": -41.3, "ref_lon": 174.8}, {"result.short_code": "PQGP+Q9"}, "Open Location Code specification (shorten)", "open-location-code main"),
        vec(6, {"code": "8FVC0000+", "ref_lat": 47.4, "ref_lon": 8.6}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC),
    ]
    return enc, dec, sh


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    ge, gd, gn = geohash()
    tf, tb, tr = tiles()
    oe, od, osh = olc()
    files = {"indexing.geohash.encode": ge, "indexing.geohash.decode": gd, "indexing.geohash.neighbors": gn, "indexing.tile.from-point": tf,
             "indexing.tile.bounds": tb, "indexing.tile.ground-resolution": tr, "indexing.plus-code.encode": oe, "indexing.plus-code.decode": od,
             "indexing.plus-code.shorten": osh}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
