#!/usr/bin/env python3
"""Golden vectors for aviation.airspace.tfr-area from GeographicLib for Python
(Karney), which the core does not use: each packed coordinate is decoded here
from its degrees, minutes, and seconds, the 72 circle points come from
Geodesic.WGS84.Direct, and every area from Geodesic.WGS84.Polygon.

Circles from 0.5 to 400 NM between 71 N and 55 S, both fix-radial-distance
forms, point lists entered clockwise, counterclockwise, and closed on their
first point, and the two areas the tool refuses to draw across the 180th
meridian.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_tfr_ref.py   (with geographiclib installed)
"""
import json
from importlib.metadata import version
from pathlib import Path

from geographiclib.geodesic import Geodesic

VER = f"geographiclib {version('geographiclib')}"
SRC = f"GeographicLib for Python {version('geographiclib')} (Geodesic.WGS84.Direct and Polygon) (tools/vectors/gen_tfr_ref.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.airspace.tfr-area.jsonl"
G = Geodesic.WGS84
NM = 1852.0


def packed(lat, lon):
    """Packed text and degrees from ((d, m, s, hemi), (d, m, s, hemi)); s None for the minutes form."""
    def one(p, width):
        d, m, s, h = p
        text = f"{d:0{width}d}{m:02d}" + ("" if s is None else (f"{s:02d}" if isinstance(s, int) else f"{int(s):02d}{str(s)[str(s).index('.'):]}")) + h
        return text, (d + m / 60 + (s or 0) / 3600) * (-1 if h in "SW" else 1)
    (a, la), (b, lo) = one(lat, 2), one(lon, 3)
    return a + b, (la, lo)


def direct(lat, lon, az, s):
    r = G.Direct(lat, lon, az, s)
    return r["lat2"], (r["lon2"] + 540) % 360 - 180


def area(pts):
    p = G.Polygon()
    for la, lo in pts:
        p.AddPoint(la, lo)
    return abs(p.Compute(False, True)[2]) / NM ** 2


def bounds(pts):
    return {"result.south.value": min(p[0] for p in pts), "result.north.value": max(p[0] for p in pts),
            "result.west.value": min(p[1] for p in pts), "result.east.value": max(p[1] for p in pts)}


def circle(c, r_nm):
    return [direct(c[0], c[1], 360 - 5 * k, r_nm * NM) for k in range(72)]


rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "gen_tfr_ref.py" not in r["source"]]


def add(inp, exp):
    e = {"ok": True, **exp}
    tol = {k: {"rel": 1e-12 if k == "result.area.value" else 0, "abs": 1e-9} for k, v in e.items() if isinstance(v, float)}
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": tol})


# Circles: center as (lat dms, lon dms), radius NM.
for lat, lon, r in [((40, 38, 23, "N"), (73, 46, 44, "W"), 30), ((51, 28, None, "N"), (0, 27, None, "W"), 2.5),
                    ((33, 56, 46.5, "S"), (151, 10, 38.25, "E"), 0.5), ((0, 0, 0, "N"), (0, 0, 0, "E"), 100),
                    ((71, 17, 8, "N"), (156, 45, 58, "W"), 25), ((54, 48, None, "S"), (68, 18, None, "W"), 400),
                    ((1, 21, 33, "N"), (103, 59, 22, "E"), 7.25)]:
    text, c = packed(lat, lon)
    ring = circle(c, r)
    add({"center": text, "radius": f"{r} NM"}, {"result.area.value": area(ring), "result.center_lat.value": c[0], "result.center_lon.value": c[1], **bounds(ring)})
# Fix-radial-distance: ident, radial, NM, navaid lat, lon, variation, radius NM.
for ident, radial, nm, la, lo, var, r in [("DEN", 265, 30, 39.8125, -104.6608, 11, 3), ("LON", 90, 5.5, 51.4871, -0.4667, -1, 1), ("NTAA", 359, 250, -17.5537, -149.6071, 12, 60)]:
    c = direct(la, lo, (radial + var) % 360, nm * NM)
    ring = circle(c, r)
    dist = f"{nm:05.1f}" if nm != int(nm) else f"{int(nm):03d}"
    add({"center": f"{ident}{radial:03d}{dist}", "radius": f"{r} NM", "navaid_lat": la, "navaid_lon": lo, "navaid_variation": f"{var} deg"},
        {"result.area.value": area(ring), "result.center_lat.value": c[0], "result.center_lon.value": c[1], **bounds(ring)})
# Point lists: counterclockwise, clockwise, closed on the first point, and concave.
POLYS = [
    [((38, 51, 0, "N"), (77, 2, 30, "W")), ((38, 51, 0, "N"), (76, 55, 0, "W")), ((38, 56, 30, "N"), (76, 58, 0, "W"))],
    [((28, 40, None, "N"), (80, 45, None, "W")), ((28, 40, None, "N"), (80, 30, None, "W")), ((28, 25, None, "N"), (80, 30, None, "W")), ((28, 25, None, "N"), (80, 45, None, "W"))],
    [((34, 0, 0, "S"), (18, 20, 0, "E")), ((34, 0, 0, "S"), (18, 40, 0, "E")), ((33, 45, 0, "S"), (18, 40, 0, "E")), ((33, 50, 0, "S"), (18, 30, 0, "E")), ((33, 45, 0, "S"), (18, 20, 0, "E")), ((34, 0, 0, "S"), (18, 20, 0, "E"))],
    [((64, 0, 0, "N"), (22, 30, 0, "W")), ((63, 30, 0, "N"), (20, 0, 0, "W")), ((64, 30, 0, "N"), (17, 0, 0, "W")), ((66, 0, 0, "N"), (18, 0, 0, "W")), ((65, 45, 30.5, "N"), (23, 15, 10.25, "W"))],
]
for poly in POLYS:
    both = [packed(a, b) for a, b in poly]
    pts = [p for _, p in both]
    distinct = pts[:-1] if pts[0] == pts[-1] else pts
    add({"points": [{"point": t} for t, _ in both]}, {"result.area.value": area(distinct), **bounds(distinct)})
# Refused: areas across the 180th meridian.
rows.append({"id": f"v{len(rows) + 1:03d}", "input": {"center": "1640S17959E", "radius": "5 NM"}, "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
rows.append({"id": f"v{len(rows) + 1:03d}", "input": {"points": [{"point": "5200N17900E"}, {"point": "5200N17900W"}, {"point": "5300N17900W"}]},
             "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
