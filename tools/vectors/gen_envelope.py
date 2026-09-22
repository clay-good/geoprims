#!/usr/bin/env python3
"""Golden vectors for geometry.shape.bbox. Longitude boxes are checked by
hand; a geodesic edge's highest latitude comes from Clairaut's relation on
the ellipsoid, cos(beta_max) = |sin(azi1)| cos(beta1) with reduced latitude
tan(beta) = (1 - f) tan(phi), taking azi1 from GeographicLib's GeodSolve
(C++, independent of the geographiclib-rs the tool uses). Requires GeodSolve."""
import json
import math
import subprocess
import sys
from pathlib import Path

F = 1 / 298.257223563
SRC = "Hand-checked longitude boxes; edge vertices by Clairaut's relation with GeodSolve azimuths (tools/vectors/gen_envelope.py)"
SPEC = "add-navigation-and-geometry scenarios"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"


def azi1(a, b):
    out = subprocess.run(["GeodSolve", "-i", "-p", "12"], input=f"{a[0]} {a[1]} {b[0]} {b[1]}\n", capture_output=True, text=True, check=True).stdout.split()
    return float(out[0])


def vertex_lat(a, b):
    beta1 = math.atan((1 - F) * math.tan(math.radians(a[0])))
    bmax = math.acos(abs(math.sin(math.radians(azi1(a, b)))) * math.cos(beta1))
    return math.degrees(math.atan(math.tan(bmax) / (1 - F)))


def vec(i, inp, exp, src=SRC, ver="2026", tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def pts(*ll, ring=None):
    return [dict({"lat": a, "lon": b}, **({"ring": ring} if ring is not None else {})) for a, b in ll]


def box(w, s, e, n, span, crosses, pole="none"):
    return {"result.west.value": float(w), "result.south.value": float(s), "result.east.value": float(e), "result.north.value": float(n),
            "result.lon_span.value": float(span), "result.crosses_antimeridian": crosses, "result.pole": pole}


def direct(lat, lon, az, s):
    out = subprocess.run(["GeodSolve", "-p", "12"], input=f"{lat} {lon} {az} {s}\n", capture_output=True, text=True, check=True).stdout.split()
    return float(out[0]), float(out[1])


def inverse(a, b):
    out = subprocess.run(["GeodSolve", "-i", "-p", "12"], input=f"{a[0]} {a[1]} {b[0]} {b[1]}\n", capture_output=True, text=True, check=True).stdout.split()
    return float(out[0]), float(out[2])  # azi1, s12


def enclosing_vectors():
    out = []
    src = "Circles and rectangles built with GeographicLib's GeodSolve (C++), evaluated in Python (tools/vectors/gen_envelope.py)"
    cv = lambda lat, lon, r, tol_m: ({"result.circle_lat.value": lat, "result.circle_lon.value": lon, "result.circle_radius.value": r},
                                     {"result.circle_lat.value": tol_m / 111_000, "result.circle_lon.value": tol_m / 50_000, "result.circle_radius.value": tol_m})
    # Two points: the circle is centered on the geodesic midpoint.
    for a, b in [((40, -105), (40.01, -104.98)), ((-33.9, 18.4), (-34.2, 18.9)), ((64, -150), (65, -145))]:
        az, s12 = inverse(a, b)
        m = direct(a[0], a[1], az, s12 / 2)
        exp, tol = cv(m[0], m[1], s12 / 2, 1e-3)
        v = vec(len(out) + 1, {"points": pts(a, b)}, exp, src, VER)
        v["tolerance"].update({k: {"rel": 0, "abs": t} for k, t in tol.items()})
        out.append(v)
    # Points at one distance from a center, at 0, 120, and 240 degrees (and 90-degree steps
    # with interior points): that center is the smallest circle's, whatever the scale.
    for c, r, azs, inner in [((40, -105), 500, [0, 120, 240], []), ((10, 30), 5000, [0, 90, 180, 270], [(45, 1500), (200, 3000)]),
                             ((52, 4), 500_000, [10, 130, 250], [(0, 100_000)]), ((-70, 160), 20_000, [0, 120, 240], [])]:
        ring = [direct(c[0], c[1], az, r) for az in azs] + [direct(c[0], c[1], az, d) for az, d in inner]
        exp, tol = cv(float(c[0]), float(c[1]), float(r), 1e-3)
        if len(azs) == 4:
            exp["result.hull_count"] = 4
        v = vec(len(out) + 1, {"points": pts(*ring)}, exp, src, VER)
        v["tolerance"].update({k: {"rel": 0, "abs": t} for k, t in tol.items()})
        out.append(v)
    # A 300 m by 100 m rectangle turned to 30 degrees, with points inside.
    c = (39.95, -105.2)
    corners = []
    for along, across in [(-150, -50), (150, -50), (150, 50), (-150, 50), (20, 10), (-60, -30)]:
        d, az = math.hypot(along, across), math.degrees(math.atan2(across, along)) + 30
        corners.append(direct(c[0], c[1], az, d))
    v = vec(len(out) + 1, {"points": pts(*corners)}, {"result.rect_length.value": 300.0, "result.rect_width.value": 100.0, "result.rect_azimuth.value": 30.0, "result.hull_count": 4}, src, VER)
    v["tolerance"].update({"result.rect_length.value": {"rel": 0, "abs": 2e-3}, "result.rect_width.value": {"rel": 0, "abs": 2e-3}, "result.rect_azimuth.value": {"rel": 0, "abs": 1e-4}})
    out.append(v)
    out.append(vec(len(out) + 1, {"points": pts((0, 0), (0, 120), (0, -120))}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, SPEC))
    return out


def main():
    out = []
    # The spec scenario: 170° E and 170° W give a 20° box across the antimeridian.
    v = vec(len(out) + 1, {"points": pts((-17, 170), (-15, -170))}, box(170, -17, -170, -15, 20, "yes"), SPEC)
    v["expect"]["meta.warnings.0.code"] = "CROSSES_ANTIMERIDIAN"
    out.append(v)
    out.append(vec(len(out) + 1, {"points": pts((40.1, -105.2), (39.7, -104.8), (40.4, -105.0))}, box(-105.2, 39.7, -104.8, 40.4, 0.4, "no"), tol=1e-9))
    out.append(vec(len(out) + 1, {"points": pts((12.5, 7.25))}, box(7.25, 12.5, 7.25, 12.5, 0, "no")))
    out.append(vec(len(out) + 1, {"points": pts((0, 175), (5, -178), (-3, -170), (1, 179))}, box(175, -3, -170, 5, 15, "yes")))
    out.append(vec(len(out) + 1, {"points": pts((10, -30), (20, 100))}, box(-30, 10, 100, 20, 130, "no")))
    # Geodesic edges bow poleward: the box reaches each edge's vertex.
    for a, b in [((60, -10), (60, 10)), ((45, 0), (45, 30)), ((-50, 100), (-50, 140)), ((70, -170), (70, 170))]:
        top = vertex_lat(a, b)
        south, north = (min(a[0], b[0]), top) if a[0] > 0 else (-top, max(a[0], b[0]))
        w, e = (a[1], b[1]) if (b[1] - a[1]) % 360 < 180 else (b[1], a[1])
        span = (e - w) % 360
        out.append(vec(len(out) + 1, {"points": pts(a, b), "shape": "line"}, box(w, south, e, north, span, "yes" if e < w else "no"), SRC, VER, tol=1e-8))
    # A ring around the North Pole spans every longitude and reaches 90°.
    out.append(vec(len(out) + 1, {"points": pts((80, 0), (80, 90), (80, 180), (80, -90)), "shape": "polygon"},
                   {"result.west.value": -180.0, "result.east.value": 180.0, "result.north.value": 90.0, "result.south.value": 80.0, "result.pole": "north", "result.lon_span.value": 360.0}))
    # A polygon's edge across the antimeridian counts, not the long way round.
    out.append(vec(len(out) + 1, {"points": pts((0, 179), (0, -179), (1, -179), (1, 179)), "shape": "polygon"},
                   {"result.west.value": 179.0, "result.east.value": -179.0, "result.lon_span.value": 2.0, "result.crosses_antimeridian": "yes"}))
    out.append(vec(len(out) + 1, {"points": [{"lat": 91, "lon": 0}]}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, SPEC))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.shape.enclosing.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in enclosing_vectors()))
    (dest / "geometry.shape.bbox.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
