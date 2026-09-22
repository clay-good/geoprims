#!/usr/bin/env python3
"""Golden vectors for geometry.buffer.geodesic from closed forms on small
shapes near the equator, where a degree is a known length: a convex polygon
grown by d has area A + P·d + π·d² (Steiner); shrunk, a rectangle loses 2d
per side; a point's buffer is a circle of area π·r²; a line with flat ends is
a 2d-wide rectangle. Every vector also expects the measured deviation to be
within the tool's tolerance (0.1% of d or 0.5 m)."""
import json
import math
import sys
from pathlib import Path

A = 6378137.0
F = 1 / 298.257223563
E2 = F * (2 - F)
M0 = A * (1 - E2)  # meridian radius at the equator
SRC = "Steiner's formula and plane closed forms on small near-equator shapes, evaluated in Python (tools/vectors/gen_buffer.py)"
SPEC = "add-navigation-and-geometry scenarios"


def dlon(m):
    return math.degrees(m / A)


def dlat(m):
    return math.degrees(m / M0)


def rect(w, h, lat0=0.0, lon0=0.0):
    return [{"lat": lat0, "lon": lon0}, {"lat": lat0, "lon": lon0 + dlon(w)},
            {"lat": lat0 + dlat(h), "lon": lon0 + dlon(w)}, {"lat": lat0 + dlat(h), "lon": lon0}]


def vec(i, inp, exp, src=SRC, ver="2026", rel=None, dev=None):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {}
    for k, v in e.items():
        if isinstance(v, bool) or not isinstance(v, (int, float)):
            continue
        tol[k] = {"rel": rel if (rel and k == "result.area.value") else 1e-12, "abs": 1e-12}
    if dev is not None:
        e["result.max_deviation.value"] = 0.0
        tol["result.max_deviation.value"] = {"rel": 0, "abs": dev}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def main():
    out = []
    tol = lambda d: max(0.001 * abs(d), 0.5)
    # Squares and rectangles grown by d with round corners: A + P·d + π·d².
    for w, h, d in [(1000, 600, 500), (200, 200, 50), (2000, 300, 100), (500, 500, 1000), (100, 400, 20)]:
        area = w * h + 2 * (w + h) * d + math.pi * d * d
        out.append(vec(len(out) + 1, {"vertices": rect(w, h), "distance": f"{d} m"}, {"result.area.value": area / 1e6, "result.parts": 1}, rel=2e-3, dev=tol(d)))
    # Mitre corners: a rectangle again; bevel: minus four d²/2 corners.
    for w, h, d in [(1000, 600, 100), (300, 300, 30)]:
        out.append(vec(len(out) + 1, {"vertices": rect(w, h), "distance": f"{d} m", "join": "mitre"},
                       {"result.area.value": (w + 2 * d) * (h + 2 * d) / 1e6, "result.vertex_count": 4}, rel=1e-4))
        out.append(vec(len(out) + 1, {"vertices": rect(w, h), "distance": f"{d} m", "join": "bevel"},
                       {"result.area.value": ((w + 2 * d) * (h + 2 * d) - 2 * d * d) / 1e6, "result.vertex_count": 8}, rel=1e-4))
    # Shrinking a rectangle: sharp corners, 2d off each side.
    for w, h, d in [(850, 100, 20), (500, 300, 100)]:
        out.append(vec(len(out) + 1, {"vertices": rect(w, h), "distance": f"-{d} m"},
                       {"result.area.value": (w - 2 * d) * (h - 2 * d) / 1e6, "result.parts": 1}, rel=1e-4, dev=tol(d)))
    # A 100 m wide polygon shrunk by 60 m collapses (the spec scenario).
    v = vec(len(out) + 1, {"vertices": rect(850, 100), "distance": "-60 m"}, {"result.parts": 0, "meta.warnings.0.code": "BUFFER_COLLAPSED"}, SPEC)
    out.append(v)
    # Points: geodesic circles, anywhere, including across the antimeridian and near a pole.
    for lat, lon, r in [(0, 0, 2000), (40, -105, 500), (0, 179.9999, 3000), (89.99, 45, 1000), (-45, 170, 10000)]:
        out.append(vec(len(out) + 1, {"vertices": [{"lat": lat, "lon": lon}], "distance": f"{r} m"},
                       {"result.area.value": math.pi * r * r / 1e6, "result.parts": 1}, rel=1e-3, dev=tol(r)))
    # Lines along the equator: flat ends make a 2d × L rectangle; square ends add d at each end; round ends a circle.
    for L, d in [(1000, 50), (5000, 200)]:
        line = [{"lat": 0, "lon": 0}, {"lat": 0, "lon": dlon(L)}]
        out.append(vec(len(out) + 1, {"vertices": line, "distance": f"{d} m", "cap": "flat"}, {"result.area.value": 2 * d * L / 1e6}, rel=1e-4))
        out.append(vec(len(out) + 1, {"vertices": line, "distance": f"{d} m", "cap": "square"}, {"result.area.value": 2 * d * (L + 2 * d) / 1e6}, rel=1e-4))
        out.append(vec(len(out) + 1, {"vertices": line, "distance": f"{d} m"}, {"result.area.value": (2 * d * L + math.pi * d * d) / 1e6}, rel=2e-3, dev=tol(d)))
    # A square with a square hole: the hole shrinks to (s − 2d)², sharp-cornered.
    s, d = 400, 50
    poly = rect(1000, 1000) + [dict(p, ring=1) for p in rect(s, s, dlat(300), dlon(300))]
    out.append(vec(len(out) + 1, {"vertices": poly, "distance": f"{d} m"},
                   {"result.area.value": (1000 * 1000 + 4000 * d + math.pi * d * d - (s - 2 * d) ** 2) / 1e6, "result.parts": 1}, rel=2e-3, dev=tol(d)))
    # The spec's geofence scenario: 500 m ± 0.5 m everywhere.
    field = [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0, "lon": -104.99}, {"lat": 40.006, "lon": -104.99}, {"lat": 40.006, "lon": -105.0}]
    out.append(vec(len(out) + 1, {"vertices": field, "distance": "500 m"}, {"result.parts": 1}, SPEC, dev=0.5))
    # Refusals.
    for inp, code in [({"vertices": [{"lat": 0, "lon": 0}], "distance": "10 m", "cap": "flat"}, "INVALID_INPUT"),
                      ({"vertices": rect(100, 100), "distance": "0 m"}, "INVALID_INPUT"),
                      ({"vertices": rect(100, 100), "distance": "1500 km"}, "OUT_OF_DOMAIN")]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": code}, SPEC))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.buffer.geodesic.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
