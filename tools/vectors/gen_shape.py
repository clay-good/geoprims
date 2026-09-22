#!/usr/bin/env python3
"""Golden vectors for geometry.shape.centroid, independent of the tool's
equal-area map: symmetric shapes put the centroid at their center; a small
triangle's centroid is its corners' mean on a local plane (to about 1e-8°
at 200 m); a ring around the pole centers on it; a square's interior point
is its middle, half a side from every edge."""
import json
import math
import sys
from pathlib import Path

A = 6378137.0
F = 1 / 298.257223563
E2 = F * (2 - F)
SRC = "Symmetry and local-plane centroids, evaluated in Python (tools/vectors/gen_shape.py)"
SPEC = "add-navigation-and-geometry scenarios"


def radii(lat):
    s = math.sin(math.radians(lat))
    w = math.sqrt(1 - E2 * s * s)
    return A * (1 - E2) / w**3, A / w  # meridian M, prime vertical N


def offset(lat0, lon0, north, east):
    m, n = radii(lat0)
    return {"lat": lat0 + math.degrees(north / m), "lon": lon0 + math.degrees(east / (n * math.cos(math.radians(lat0))))}


def vec(i, inp, exp, src=SRC, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": (tol or {}).get(k, 1e-9)} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def main():
    out = []
    # Rectangles centered on a point: the centroid is that point, and so is the interior point.
    for lat0, lon0, w, h in [(0, 0, 400, 300), (0, 10, 1000, 1000), (-33.9, 18.4, 250, 600), (60, -150, 800, 200)]:
        if lat0 == 0:
            m, n = radii(0)
            dy, dx = math.degrees(h / 2 / m), math.degrees(w / 2 / n)
            poly = [{"lat": -dy, "lon": lon0 - dx}, {"lat": -dy, "lon": lon0 + dx}, {"lat": dy, "lon": lon0 + dx}, {"lat": dy, "lon": lon0 - dx}]
            tol_c = 1e-11
        else:
            poly = [offset(lat0, lon0, -h / 2, -w / 2), offset(lat0, lon0, -h / 2, w / 2), offset(lat0, lon0, h / 2, w / 2), offset(lat0, lon0, h / 2, -w / 2)]
            # Edges between corners at one latitude are geodesics, which bow
            # poleward by L²·tan φ / (8R): about 2 cm for 800 m at 60°, moving
            # the true centroid that way. The local-plane expectation allows it.
            tol_c = 3e-7
        size_deg = math.degrees(max(w, h) / A) / math.cos(math.radians(lat0))
        exp = {"result.centroid_lat.value": float(lat0), "result.centroid_lon.value": float(lon0), "result.centroid_inside": "yes", "result.clearance.value": min(w, h) / 2}
        if w == h:  # only a square's interior point is unique: a longer rectangle's runs along its midline
            exp.update({"result.interior_lat.value": float(lat0), "result.interior_lon.value": float(lon0)})
        out.append(vec(len(out) + 1, {"polygon": poly}, exp,
                       tol={"result.centroid_lat.value": tol_c, "result.centroid_lon.value": tol_c, "result.interior_lat.value": 2e-3 * size_deg,
                            "result.interior_lon.value": 2e-3 * size_deg, "result.clearance.value": 2e-3 * max(w, h) + 0.01}))
    # Small triangles: the centroid is the corners' mean on the local plane.
    for lat0, lon0, pts in [(40, -105, [(0, 0), (150, 20), (40, 170)]), (-12, 130, [(0, 0), (-90, 60), (30, 200)]), (71, 25, [(10, 0), (200, 50), (0, 120)])]:
        poly = [offset(lat0, lon0, n_, e_) for n_, e_ in pts]
        cn, ce = sum(p[0] for p in pts) / 3, sum(p[1] for p in pts) / 3
        c = offset(lat0, lon0, cn, ce)
        out.append(vec(len(out) + 1, {"polygon": poly}, {"result.centroid_lat.value": c["lat"], "result.centroid_lon.value": c["lon"], "result.centroid_inside": "yes"},
                       tol={"result.centroid_lat.value": 2e-7, "result.centroid_lon.value": 2e-7}))
    # A square with a centered square hole: still centered; area is the difference.
    m, n = radii(0)
    d = lambda y, x: {"lat": math.degrees(y / m), "lon": math.degrees(x / n)}
    poly = [d(-500, -500), d(-500, 500), d(500, 500), d(500, -500)] + [dict(p, ring=1) for p in [d(-200, -200), d(-200, 200), d(200, 200), d(200, -200)]]
    out.append(vec(len(out) + 1, {"polygon": poly}, {"result.centroid_lat.value": 0.0, "result.centroid_lon.value": 0.0, "result.area.value": (1000**2 - 400**2) / 1e6},
                   tol={"result.centroid_lat.value": 1e-11, "result.centroid_lon.value": 1e-11, "result.area.value": 1e-6}))
    # A ring around the North Pole centers on it.
    out.append(vec(len(out) + 1, {"polygon": [{"lat": 80, "lon": 0}, {"lat": 80, "lon": 90}, {"lat": 80, "lon": 180}, {"lat": 80, "lon": -90}]},
                   {"result.centroid_lat.value": 90.0, "result.centroid_inside": "yes"}, tol={"result.centroid_lat.value": 1e-9}))
    # The spec's C-shape: the centroid falls in the notch, flagged, and the interior point is inside.
    c_shape = [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0, "lon": -104.997}, {"lat": 40.0006, "lon": -104.997}, {"lat": 40.0006, "lon": -104.9994},
               {"lat": 40.0024, "lon": -104.9994}, {"lat": 40.0024, "lon": -104.997}, {"lat": 40.003, "lon": -104.997}, {"lat": 40.003, "lon": -105.0}]
    out.append(vec(len(out) + 1, {"polygon": c_shape}, {"result.centroid_inside": "no", "meta.warnings.0.code": "CENTROID_OUTSIDE"}, SPEC))
    # Refusals: holes larger than the outline, and a ring number out of range.
    out.append(vec(len(out) + 1, {"polygon": [d(-10, -10), d(-10, 10), d(10, 10), d(10, -10)] + [dict(p, ring=1) for p in [d(-50, -50), d(-50, 50), d(50, 50), d(50, -50)]]},
                   {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    out.append(vec(len(out) + 1, {"polygon": [dict(d(0, 0), ring=0.5), d(0, 10), d(10, 10)]}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.shape.centroid.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
